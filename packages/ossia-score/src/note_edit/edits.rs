//! Source changes for lengths, chords, slurs and measure rests.
use super::*;

/// Replace `event` (and rests after it) with `head` text of length `need`, followed by rests
/// for whatever length remains.
pub(super) fn overwrite(
    ctx: &Context,
    index: usize,
    need: Frac,
    head: String,
    heads: Vec<(usize, Spelled)>,
) -> Result<(Vec<Change>, Cursor), AppError> {
    let event = &ctx.events[index];
    if event.in_tuplet {
        return Err(AppError::unsupported(
            "note input inside tuplets is not supported yet",
        ));
    }
    let mut total = event.length();
    let mut last = index;
    while total < need {
        let next = ctx
            .events
            .get(last + 1)
            .filter(|e| matches!(e.kind, EventKind::Rest | EventKind::Skip))
            .filter(|e| e.block == event.block && !e.in_tuplet)
            .ok_or_else(|| invalid("the value does not fit: the following notes are not rests"))?;
        total = total.add(next.length());
        last += 1;
    }
    let end = if last == index {
        ctx.events[index].full_end
    } else {
        ctx.events[last].full_end
    };
    if ctx.has_bar_between(event.start, end) {
        return Err(invalid("the value does not fit in this measure"));
    }
    let rest = if event.kind == EventKind::Skip {
        "s"
    } else {
        "r"
    };
    let remainder = values_for(total.sub(need).unwrap_or(Frac::new(0, 1)))
        .ok_or_else(|| invalid("the remaining length cannot be written"))?;
    let mut text = head;
    let mut cursor_offset = None;
    for value in remainder {
        text.push(' ');
        cursor_offset.get_or_insert(text.len());
        text.push_str(rest);
        text.push_str(&value.text());
    }
    let mut changes = vec![Change {
        start: event.start,
        end,
        text,
        heads,
    }];
    // The next event may have taken its length from the text just replaced.
    let after = ctx.events.get(last + 1);
    if let Some(next) = after.filter(|e| e.written.is_none()) {
        let value = values_for(next.value)
            .filter(|v| v.len() == 1)
            .ok_or_else(|| AppError::unsupported("the next event's length cannot be written"))?;
        changes.push(Change {
            start: next.end,
            end: next.end,
            text: value[0].text(),
            heads: vec![],
        });
    }
    let cursor = match cursor_offset {
        Some(offset) => Cursor::Within(event.start, offset),
        None => Cursor::At(after.map_or(end, |e| e.start)),
    };
    Ok((changes, cursor))
}

/// Expand a multi-measure rest so its first measure can be edited as an ordinary rest.
pub(super) fn split_measure_rest(ctx: &Context, event: &Event) -> Result<String, AppError> {
    let value = written(ctx, event);
    let scale = if event.scale == Frac::new(1, 1) {
        String::new()
    } else {
        format!("*{}/{}", event.scale.0, event.scale.1)
    };
    let mut text = format!("r{value}{scale}");
    if event.count > 1 {
        let more = if event.count > 2 {
            format!("*{}", event.count - 1)
        } else {
            String::new()
        };
        text.push_str(&format!(" R{value}{scale}{more}"));
    }
    let mut source = ctx.source.to_owned();
    source.replace_range(event.start..event.full_end, &text);
    Ok(source)
}

/// The written pitch text and sounding pitch of each head of a note or chord.
pub(super) fn own_heads(
    ctx: &Context,
    event: &Event,
    pitches: Vec<Spelled>,
) -> Vec<(String, Spelled)> {
    event
        .members
        .iter()
        .zip(pitches)
        .map(|((start, end, _), pitch)| (ctx.source[*start..*end].to_owned(), pitch))
        .collect()
}

/// Rewrite a note or chord with the given heads, keeping its written duration.
pub(super) fn rebuild(ctx: &Context, event: &Event, heads: Vec<(String, Spelled)>) -> Change {
    let duration = written(ctx, event);
    let mut text = String::new();
    let mut placed = Vec::new();
    if heads.len() > 1 {
        text.push('<');
    }
    for (k, (name, pitch)) in heads.into_iter().enumerate() {
        if k > 0 {
            text.push(' ');
        }
        placed.push((text.len(), pitch));
        text.push_str(&name);
    }
    if text.starts_with('<') {
        text.push('>');
    }
    text.push_str(&duration);
    Change {
        start: event.start,
        end: event.end,
        text,
        heads: placed,
    }
}

pub(super) fn cursor_of(event: &Event) -> usize {
    event.members.first().map_or(event.start, |m| m.0)
}

/// The event's own length as text, written out when it was carried over.
pub(super) fn value_text(ctx: &Context, event: &Event) -> Result<String, AppError> {
    let values = values_for(event.value)
        .filter(|v| v.len() == 1)
        .ok_or_else(|| AppError::unsupported("this length cannot be written"))?;
    Ok(if event.written.is_some() {
        written(ctx, event)
    } else {
        values[0].text()
    })
}

pub(super) fn set_value(
    ctx: &Context,
    index: usize,
    value: NoteValue,
    pitches: &[pitch::PitchEvent],
) -> Result<(Vec<Change>, Cursor), AppError> {
    let event = &ctx.events[index];
    if !event.pitched() {
        return overwrite(
            ctx,
            index,
            value.length(),
            format!("r{}", value.text()),
            vec![],
        );
    }
    let heads = head_pitches(pitches, event)?;
    // Rewrite the note with its own heads, keep its marks, and let `overwrite` fit the length.
    let body = match event.kind {
        EventKind::Chord => {
            let close = event.end - written(ctx, event).len();
            ctx.source[event.start..close].to_owned()
        }
        _ => {
            let (start, pitch_end, _) = event.members[0];
            ctx.source[start..pitch_end].to_owned()
        }
    };
    let text = format!("{body}{}", value.text());
    let marks_kept: Vec<(usize, Spelled)> = event
        .members
        .iter()
        .zip(heads)
        .map(|((start, _, _), pitch)| (start - event.start, pitch))
        .collect();
    let post = ctx.post_end(event.full_end);
    let (mut changes, _) = overwrite(ctx, index, value.length(), text, marks_kept)?;
    // Keep ties, slurs and dynamics on the note: move the remainder rests after them.
    if post > event.full_end {
        let main = &mut changes[0];
        if let Some(split) = main.text.find(" r").or_else(|| main.text.find(" s")) {
            let rests = main.text.split_off(split);
            let consumed = main.end > event.full_end;
            let attached = ctx.source[event.full_end..post].to_owned();
            if consumed {
                main.text.push_str(&attached);
                main.text.push_str(&rests);
            } else {
                main.end = event.full_end;
                changes.push(Change {
                    start: post,
                    end: post,
                    text: rests,
                    heads: vec![],
                });
            }
        } else if main.end > event.full_end {
            let attached = ctx.source[event.full_end..post].to_owned();
            main.text.push_str(&attached);
        }
    }
    Ok((changes, Cursor::At(cursor_of(event))))
}

pub(super) fn toggle_slur(ctx: &Context, index: usize) -> Result<Vec<Change>, AppError> {
    let event = &ctx.events[index];
    if !event.pitched() {
        return Err(invalid("select a note to start a slur"));
    }
    let open = ctx
        .post_tokens(event)
        .into_iter()
        .find(|t| t.kind == TokenKind::Raw && t.raw == "(");
    if let Some(open) = open {
        let close = ctx.events[index + 1..]
            .iter()
            .flat_map(|e| ctx.post_tokens(e))
            .find(|t| t.kind == TokenKind::Raw && t.raw == ")")
            .ok_or_else(|| invalid("the slur has no end"))?;
        return Ok(vec![
            Change {
                start: open.byte_start,
                end: open.byte_end,
                text: String::new(),
                heads: vec![],
            },
            Change {
                start: close.byte_start,
                end: close.byte_end,
                text: String::new(),
                heads: vec![],
            },
        ]);
    }
    let next = ctx.events[index + 1..]
        .iter()
        .find(|e| e.pitched())
        .ok_or_else(|| invalid("a slur needs a following note"))?;
    Ok(vec![
        Change {
            start: event.end,
            end: event.end,
            text: "(".into(),
            heads: vec![],
        },
        Change {
            start: next.end,
            end: next.end,
            text: ")".into(),
            heads: vec![],
        },
    ])
}
