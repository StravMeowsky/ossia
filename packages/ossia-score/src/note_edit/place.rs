//! Apply source changes and correct octave marks so every other note keeps its pitch.
use super::*;

/// Where the cursor goes: an old byte offset, or an offset into the text of the change that
/// started at an old byte offset.
#[derive(Debug, Clone, Copy)]
pub(super) enum Cursor {
    At(usize),
    Within(usize, usize),
}

/// Source text changes: replace `start..end` with text whose new pitch heads are recorded.
pub(super) struct Change {
    pub(super) start: usize,
    pub(super) end: usize,
    pub(super) text: String,
    /// Offsets into `text` of pitch heads to place at a sounding pitch, in source order.
    pub(super) heads: Vec<(usize, Spelled)>,
}

/// Apply changes, place new heads at their pitches and keep every other note sounding as before.
pub(super) fn finish(
    ctx: &Context,
    before: &[pitch::PitchEvent],
    mut changes: Vec<Change>,
    cursor: Cursor,
) -> Result<NoteEditOutcome, AppError> {
    changes.sort_by_key(|c| (c.start, c.end));
    for pair in changes.windows(2) {
        if pair[0].end > pair[1].start {
            return Err(invalid("overlapping note edits"));
        }
    }
    // New start of each change, and the new position of an unedited old offset.
    let mut starts = Vec::with_capacity(changes.len());
    let mut delta: isize = 0;
    for c in &changes {
        starts.push(c.start.saturating_add_signed(delta));
        delta += c.text.len() as isize - (c.end - c.start) as isize;
    }
    let shift = |position: usize| -> usize {
        let moved: isize = changes
            .iter()
            .filter(|c| c.end <= position)
            .map(|c| c.text.len() as isize - (c.end - c.start) as isize)
            .sum();
        position.saturating_add_signed(moved)
    };
    let mut source = ctx.source.to_owned();
    for c in changes.iter().rev() {
        source.replace_range(c.start..c.end, &c.text);
    }
    let placed: Vec<(usize, Spelled)> = changes
        .iter()
        .zip(&starts)
        .flat_map(|(c, start)| {
            c.heads
                .iter()
                .map(move |(offset, pitch)| (start + offset, *pitch))
        })
        .collect();
    let inside_old = |p: usize| changes.iter().any(|c| c.start <= p && p < c.end);
    // The first unedited note after the edits keeps its pitch even if its reference moved.
    let next_old = before
        .iter()
        .filter(|e| !e.members.is_empty() && e.span.byte_start >= changes[0].start)
        .find(|e| !inside_old(e.span.byte_start))
        .and_then(|e| {
            let m = &e.members[0];
            let head = ctx
                .events
                .iter()
                .find(|x| x.start == e.span.byte_start)?
                .members
                .first()?
                .0;
            Some((
                shift(head),
                Spelled {
                    di: m.di,
                    alter: m.alter,
                },
            ))
        });
    let anchor = starts[0];
    // Placing a head adds octave marks, so later positions move by the marks written.
    let mut placements: Vec<(usize, Spelled)> = placed.into_iter().chain(next_old).collect();
    placements.sort_by_key(|(p, _)| *p);
    let mut grown: isize = 0;
    let mut growth: Vec<(usize, isize)> = Vec::new();
    for (position, pitch) in placements {
        let at = position.saturating_add_signed(grown);
        let added = place(&mut source, anchor, at, pitch)?;
        grown += added;
        growth.push((position, added));
    }
    let regrow = |p: usize| -> usize {
        let extra: isize = growth.iter().filter(|(q, _)| *q < p).map(|(_, a)| a).sum();
        p.saturating_add_signed(extra)
    };
    // Everything outside the edits must sound exactly as before.
    let after = analysis(&source, anchor)?;
    let old: Vec<&Vec<i32>> = before
        .iter()
        .filter(|e| !inside_old(e.span.byte_start))
        .map(|e| &e.pitches)
        .collect();
    let new: Vec<&Vec<i32>> = after
        .iter()
        .filter(|e| {
            let p = e.span.byte_start;
            !changes.iter().zip(&starts).any(|(c, s)| {
                let (a, b) = (regrow(*s), regrow(s + c.text.len()));
                a <= p && p < b.max(a + usize::from(c.text.is_empty()))
            })
        })
        .map(|e| &e.pitches)
        .collect();
    if old != new {
        return Err(AppError::unsupported(
            "this edit would change other notes; it was not applied",
        ));
    }
    let cursor = match cursor {
        Cursor::Within(old_start, offset) => changes
            .iter()
            .zip(&starts)
            .find(|(c, _)| c.start == old_start)
            .map_or(shift(old_start), |(_, s)| s + offset),
        Cursor::At(p) => match changes
            .iter()
            .zip(&starts)
            .find(|(c, _)| c.start <= p && p < c.end)
        {
            Some((c, s)) => s + (p - c.start).min(c.text.len()),
            None => shift(p),
        },
    };
    Ok(NoteEditOutcome {
        cursor: regrow(cursor).min(source.len()),
        source,
    })
}

/// Set the octave marks of the head at `position` so it sounds at `pitch`; returns the change in
/// source length.
pub(super) fn place(
    source: &mut String,
    anchor: usize,
    position: usize,
    pitch: Spelled,
) -> Result<isize, AppError> {
    let letters = source[position..]
        .bytes()
        .take_while(u8::is_ascii_lowercase)
        .count();
    let marks_start = position + letters;
    let marks_end = marks_start
        + source[marks_start..]
            .bytes()
            .take_while(|b| matches!(b, b'\'' | b','))
            .count();
    let written: i32 = source[marks_start..marks_end]
        .bytes()
        .map(|b| if b == b'\'' { 1 } else { -1 })
        .sum();
    let sounding = head_di(source, anchor, position)?;
    let delta = pitch.di - sounding;
    if delta % 7 != 0 {
        return Err(AppError::unsupported("the edited note could not be placed"));
    }
    let text = marks(written + delta / 7);
    let grown = text.len() as isize - (marks_end - marks_start) as isize;
    source.replace_range(marks_start..marks_end, &text);
    Ok(grown)
}

/// Diatonic pitch of the head whose name starts at `position`.
pub(super) fn head_di(source: &str, anchor: usize, position: usize) -> Result<i32, AppError> {
    let unanalysed = || AppError::unsupported("the edited note could not be analysed");
    let events = scan(source)?;
    let (event, k) = events
        .iter()
        .find_map(|e| {
            e.members
                .iter()
                .position(|m| m.0 == position)
                .map(|k| (e, k))
        })
        .ok_or_else(unanalysed)?;
    analysis(source, anchor)?
        .into_iter()
        .find(|e| e.span.byte_start == event.start)
        .and_then(|e| e.members.get(k).map(|m| m.di))
        .ok_or_else(unanalysed)
}
