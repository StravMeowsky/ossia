use super::{SourcePlaybackEvent, SourcePlaybackMember};
use crate::{resolve_pitches, PitchEventKind, ScoreIndex, SourceMap};
use ossia_types::SourceSpan;
use std::collections::HashSet;
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
    let mut events = Vec::new();
    let mut seen = HashSet::new();
    for staff in &index.staves {
        for name in &staff.voices {
            let Some(voice) = index.voice(name) else {
                continue;
            };
            let Ok(resolved) = resolve_pitches(index.source(), voice.music_span) else {
                continue;
            };
            for event in resolved.events {
                let members = source_members(index.source(), &event);
                let pitches: Vec<_> = event
                    .pitches
                    .into_iter()
                    .filter_map(|pitch| u8::try_from(pitch).ok())
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
