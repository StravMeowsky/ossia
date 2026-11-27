use super::{SourcePlaybackEvent, SourcePlaybackMember};
use crate::{resolve_pitches, PitchEventKind, ScoreIndex, SourceMap};
use ossia_types::SourceSpan;
use std::collections::{HashMap, HashSet};
use std::ops::Range;

pub(super) fn source_members(source: &str, event: &crate::PitchEvent) -> Vec<SourcePlaybackMember> {
    let map = SourceMap::new(source);
    let mut cursor = event.span.byte_start;
    event
        .members
        .iter()
        .filter_map(|member| {
            let start = source
                .get(cursor..event.span.byte_end)?
                .find(&member.raw)?
                .saturating_add(cursor);
            cursor = start.saturating_add(member.raw.len());
            Some(SourcePlaybackMember {
                span: map.span(start, cursor).ok()?,
                pitch: u8::try_from(member.midi).ok()?,
            })
        })
        .collect()
}

pub(super) fn source_events(index: &ScoreIndex) -> Vec<SourcePlaybackEvent> {
    let muted_ranges = muted_ranges(index.source());
    let transpositions = score_transpositions(index);
    let pitch_source = mask_metadata_references(index.source());
    let mut events = Vec::new();
    let mut seen = HashSet::new();
    for staff in &index.staves {
        for name in &staff.voices {
            let Some(voice) = index.voice(name) else {
                continue;
            };
            let Ok(resolved) = resolve_pitches(&pitch_source, voice.music_span) else {
                continue;
            };
            for event in resolved.events {
                let Some(offset) = transpositions.get(name).copied().unwrap_or(Some(0)) else {
                    continue;
                };
                let members = source_members(index.source(), &event)
                    .into_iter()
                    .filter_map(|mut member| {
                        member.pitch = u8::try_from(i32::from(member.pitch) + offset).ok()?;
                        Some(member)
                    })
                    .collect();
                let pitches: Vec<_> = event
                    .pitches
                    .into_iter()
                    .filter_map(|pitch| u8::try_from(pitch + offset).ok())
                    .collect();
                if matches!(event.kind, PitchEventKind::Note | PitchEventKind::Chord)
                    && pitches.is_empty()
                {
                    continue;
                }
                let measure = index.measure_at(event.span.byte_start);
                let muted = muted_ranges
                    .iter()
                    .any(|range| range.contains(&event.span.byte_start));
                let voice = event
                    .voice
                    .map_or_else(|| name.clone(), |branch| format!("{name}[{branch}]"));
                if !seen.insert((
                    event.span.byte_start,
                    event.span.byte_end,
                    staff.staff_index,
                    voice.clone(),
                    pitches.clone(),
                )) {
                    continue;
                }
                events.push(SourcePlaybackEvent {
                    span: event.span,
                    staff_index: staff.staff_index,
                    voice,
                    kind: event.kind,
                    pitches,
                    members,
                    measure_start: measure.map(|measure| measure.measure_start),
                    measure_end: measure.map(|measure| measure.measure_end),
                    muted,
                });
            }
        }
    }
    events.sort_by_key(|event| (event.measure_start.unwrap_or(0), event.span.byte_start));
    events
}

fn mask_metadata_references(source: &str) -> String {
    let syntax = crate::index::Syntax::new(source);
    let tokens = &syntax.tokens;
    let mut names = HashSet::new();
    for i in 0..tokens.len().saturating_sub(3) {
        if syntax.depth[i] != 0
            || tokens[i].kind != crate::TokenKind::Ident
            || tokens[i + 1].raw != "="
            || tokens[i + 2].raw != "{"
        {
            continue;
        }
        let Some(end) = syntax.close[i + 2] else {
            continue;
        };
        let mut cursor = i + 3;
        while cursor < end {
            match tokens[cursor].command() {
                Some("key")
                    if cursor + 2 < end
                        && matches!(tokens[cursor + 2].command(), Some("major" | "minor")) =>
                {
                    cursor += 3
                }
                Some("time") if cursor + 1 < end && tokens[cursor + 1].raw.contains('/') => {
                    cursor += 2
                }
                _ => break,
            }
        }
        if cursor == end
            && (0..tokens.len().saturating_sub(1))
                .filter(|&position| {
                    syntax.depth[position] == 0
                        && tokens[position].raw == tokens[i].raw
                        && tokens[position + 1].raw == "="
                })
                .count()
                == 1
        {
            names.insert(tokens[i].raw.as_str());
        }
    }
    let mut masked = source.to_owned();
    for token in tokens
        .iter()
        .filter(|token| token.command().is_some_and(|name| names.contains(name)))
    {
        masked.replace_range(
            token.byte_start..token.byte_end,
            &" ".repeat(token.byte_end - token.byte_start),
        );
    }
    masked
}

#[cfg(test)]
#[path = "../../tests/unit/playback_source.rs"]
mod tests;

fn score_transpositions(index: &ScoreIndex) -> HashMap<String, Option<i32>> {
    let syntax = crate::index::Syntax::new(index.source());
    let tokens = &syntax.tokens;
    if syntax.malformed {
        return HashMap::new();
    }
    let scores: Vec<_> = tokens
        .iter()
        .enumerate()
        .filter_map(|(i, token)| {
            if token.command() != Some("score") || tokens.get(i + 1)?.raw != "{" {
                return None;
            }
            let end = syntax.close[i + 1]?;
            let midi = tokens[i + 2..end]
                .iter()
                .any(|token| token.command() == Some("midi"));
            Some((i + 2, end, midi))
        })
        .collect();
    let midi_scores: Vec<_> = scores.iter().filter(|score| score.2).collect();
    let score = match midi_scores.as_slice() {
        [score] => **score,
        [] if scores.len() == 1 => scores[0],
        _ => return HashMap::new(),
    };
    let (start, end, _) = score;
    let Some(language) = crate::pitch::language_before(tokens, tokens[start].byte_start) else {
        return HashMap::new();
    };
    let mut scopes = Vec::new();
    for i in start..end {
        if tokens[i].command() != Some("transpose") {
            continue;
        }
        let Some((from, to, body)) = tokens
            .get(i + 1)
            .zip(tokens.get(i + 2))
            .zip(tokens.get(i + 3))
            .map(|((a, b), c)| (a, b, c))
        else {
            return HashMap::new();
        };
        let Some(offset) = crate::pitch::transpose_interval(&from.raw, &to.raw, language) else {
            return HashMap::new();
        };
        let body_start = i + 3;
        let body_end = if body.raw == "{" || body.raw == "<<" {
            let Some(close) = syntax.close[body_start] else {
                return HashMap::new();
            };
            close
        } else if body.command().is_some() {
            body_start + 1
        } else {
            return HashMap::new();
        };
        scopes.push((body_start, body_end, offset));
    }
    let transposition_commands: Vec<_> = (start..end)
        .filter(|&i| tokens[i].command() == Some("transposition"))
        .collect();
    let staff_count = (start..end)
        .filter(|&i| {
            matches!(tokens[i].command(), Some("new" | "context"))
                && tokens.get(i + 1).is_some_and(|token| token.raw == "Staff")
        })
        .count();
    let instrument_offset = match transposition_commands.as_slice() {
        [] => 0,
        [i] if staff_count == 1 => match tokens
            .get(i + 1)
            .and_then(|token| crate::pitch::transpose_interval("c'", &token.raw, language))
        {
            Some(value) => value,
            None => return HashMap::new(),
        },
        _ => return HashMap::new(),
    };
    let names: HashSet<_> = index
        .voices
        .iter()
        .map(|voice| voice.name.as_str())
        .collect();
    let mut values: HashMap<String, Option<i32>> = HashMap::new();
    for (i, token) in tokens.iter().enumerate().take(end).skip(start) {
        let Some(name) = token.command().filter(|name| names.contains(name)) else {
            continue;
        };
        let offset = instrument_offset
            + scopes
                .iter()
                .filter(|(begin, end, _)| i >= *begin && i < *end)
                .map(|(_, _, offset)| offset)
                .sum::<i32>();
        values
            .entry(name.to_owned())
            .and_modify(|previous| {
                if *previous != Some(offset) {
                    *previous = None;
                }
            })
            .or_insert(Some(offset));
    }
    values
}

fn muted_ranges(source: &str) -> Vec<Range<usize>> {
    let mut ranges = Vec::new();
    let mut start = None;
    for token in crate::tokenize(source) {
        match token.command() {
            Some("deadNotesOn") => start = Some(token.byte_end),
            Some("deadNotesOff") => {
                if let Some(from) = start.take() {
                    ranges.push(from..token.byte_start);
                }
            }
            _ => {}
        }
    }
    if let Some(from) = start {
        ranges.push(from..source.len());
    }
    ranges
}

pub(super) fn overlaps(left: SourceSpan, right: SourceSpan) -> bool {
    left.byte_start < right.byte_end && right.byte_start < left.byte_end
}

pub(super) fn event_element_kind(event: PitchEventKind, element_kind: &str) -> bool {
    match event {
        PitchEventKind::Note | PitchEventKind::Chord => {
            matches!(element_kind, "NoteHead" | "TabNoteHead")
        }
        PitchEventKind::Rest => matches!(element_kind, "Rest" | "MultiMeasureRest"),
        PitchEventKind::Skip => false,
    }
}
