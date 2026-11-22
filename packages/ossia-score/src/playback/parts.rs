use crate::{MidiFile, ScoreIndex};
use ossia_types::{ContentHash, PlaybackNote, PlaybackNoteGroup, PlaybackPart};
use std::collections::{BTreeMap, BTreeSet};

type TimedNote = (PlaybackNote, u64, usize, u8);

fn score_instrument(source: &str) -> Option<ossia_types::SampleInstrument> {
    let syntax = crate::index::Syntax::new(source);
    let tokens = &syntax.tokens;
    let mut found = Vec::new();
    let mut unsupported_name = false;
    for (i, token) in tokens.iter().enumerate() {
        if token.command() == Some("header") {
            if let Some((open, end)) = syntax.expression(i + 1, tokens.len()) {
                for j in open + 1..end.saturating_sub(2) {
                    if syntax.depth[j] == syntax.depth[open] + 1
                        && tokens[j].raw == "instrument"
                        && tokens[j + 1].raw == "="
                        && tokens[j + 2].kind == crate::TokenKind::String
                    {
                        if let Some(instrument) =
                            crate::named_sample_instrument(tokens[j + 2].raw.trim_matches('"'))
                        {
                            found.push(instrument);
                        } else {
                            unsupported_name = true;
                        }
                    }
                }
            }
        }
        if token.raw == "stringTunings"
            && tokens.get(i + 1).is_some_and(|t| t.raw == "=")
            && tokens.get(i + 2..i + 5).is_some_and(|t| {
                t[0].raw == "#ukulele"
                    && t[1].raw == "-"
                    && t[2].raw == "tuning"
                    && t[0].byte_end == t[1].byte_start
                    && t[1].byte_end == t[2].byte_start
            })
        {
            found.push(ossia_types::SampleInstrument::Ukulele);
        }
    }
    if unsupported_name {
        return None;
    }
    let first = *found.first()?;
    found.iter().all(|value| *value == first).then_some(first)
}

#[cfg(test)]
#[path = "../../tests/unit/playback_parts.rs"]
mod tests;

/// Connect MIDI performers to source voices only where the existing note mapping agrees.
pub(super) fn assign_parts(
    index: &ScoreIndex,
    midi: &MidiFile,
    programs: &crate::instruments::ProgramTimeline,
    notes: &mut [TimedNote],
    groups: &mut [PlaybackNoteGroup],
) -> Vec<PlaybackPart> {
    let mut performers = BTreeMap::<String, (usize, u8)>::new();
    for (note, _, track, channel) in notes.iter() {
        performers.insert(note.part_id.clone(), (*track, *channel));
    }
    let score_instrument = (performers.len() == 1)
        .then(|| score_instrument(index.source()))
        .flatten();
    let mut candidates = BTreeMap::<String, (String, String, Option<usize>, Option<String>)>::new();
    for (temporary, (track, channel)) in &performers {
        let matched = groups
            .iter()
            .filter(|group| group.part_id.as_ref() == Some(temporary));
        let mut staves = BTreeSet::new();
        let mut voices = BTreeSet::new();
        for group in matched {
            if let Some(staff) = group.staff_index {
                staves.insert(staff);
            }
            if let Some(voice) = &group.voice {
                if !voice.starts_with('@') {
                    voices.insert(voice.clone());
                }
            }
        }
        let label = if staves.len() == 1 {
            staves
                .iter()
                .next()
                .and_then(|i| index.staves.get(*i))
                .and_then(|staff| staff.label.clone())
        } else {
            None
        };
        let instrument_name = if staves.len() == 1 {
            staves
                .iter()
                .next()
                .and_then(|i| index.staves.get(*i))
                .and_then(|staff| staff.instrument_name.clone())
        } else {
            None
        };
        let midi_instrument_name = if staves.len() == 1 {
            staves
                .iter()
                .next()
                .and_then(|i| index.staves.get(*i))
                .and_then(|staff| staff.midi_instrument_name.as_deref())
        } else {
            None
        };
        let piano_group = if staves.len() == 1 {
            staves
                .iter()
                .next()
                .and_then(|i| index.staves.get(*i))
                .and_then(|staff| staff.piano_group)
        } else {
            None
        };
        let track_name = midi
            .tracks
            .get(*track)
            .and_then(|track| track.name.as_ref())
            .filter(|name| !name.trim().is_empty());
        let automatic_name = instrument_name
            .clone()
            .or_else(|| piano_group.map(|_| "Piano".to_owned()))
            .or_else(|| {
                track_name
                    .filter(|name| crate::named_sample_instrument(name).is_some())
                    .cloned()
            })
            .or_else(|| score_instrument.map(|instrument| instrument.catalog_key().to_owned()));
        let name = instrument_name
            .or_else(|| piano_group.map(|_| "Piano".to_owned()))
            .or_else(|| {
                track_name
                    .filter(|name| crate::named_sample_instrument(name).is_some())
                    .cloned()
            })
            .or_else(|| score_instrument.map(|instrument| instrument.catalog_key().to_owned()))
            .unwrap_or_default();
        let identity = if let Some(group) = piano_group {
            let mut members = index
                .staves
                .iter()
                .filter(|staff| staff.piano_group == Some(group))
                .map(|staff| {
                    if let Some(label) = &staff.label {
                        return format!("staff:{label}");
                    }
                    let mut voices = staff
                        .voices
                        .iter()
                        .map(|name| {
                            if name.starts_with('@') {
                                "inline".into()
                            } else {
                                format!("voice:{name}")
                            }
                        })
                        .collect::<Vec<_>>();
                    voices.sort();
                    voices.join("|")
                })
                .collect::<Vec<_>>();
            members.sort();
            format!("piano:{}", ContentHash::of(members.join(";").as_bytes()))
        } else if let Some(label) = label {
            format!(
                "staff:{label}:voices:{}",
                voices.iter().cloned().collect::<Vec<_>>().join("\u{1f}")
            )
        } else if !voices.is_empty() {
            format!(
                "voices:{}:midi:{}",
                voices.into_iter().collect::<Vec<_>>().join("\u{1f}"),
                midi_instrument_name.unwrap_or("")
            )
        } else if staves.len() == 1 && index.staves.len() == 1 {
            format!(
                "single-staff:{:?}:midi:{}",
                index.staves[0].kind,
                midi_instrument_name.unwrap_or("")
            )
        } else if let Some(name) = track_name {
            format!("track:{name}")
        } else {
            let fingerprint = midi.tracks[*track]
                .notes
                .iter()
                .filter(|note| note.channel == *channel)
                .map(|note| format!("{}:{}:{:?}", note.time, note.note, note.duration))
                .collect::<Vec<_>>()
                .join(";");
            format!("music:{}", ContentHash::of(fingerprint.as_bytes()))
        };
        let identity = format!("part:{}", ContentHash::of(identity.as_bytes()));
        candidates.insert(
            temporary.clone(),
            (identity, name, piano_group, automatic_name),
        );
    }
    // Ambiguous identities must not transfer an override to another part after edits.
    let mut owners = BTreeMap::<String, BTreeSet<String>>::new();
    for (temporary, (key, _, group, _)) in &candidates {
        owners.entry(key.clone()).or_default().insert(
            group
                .map(|group| format!("group:{group}"))
                .unwrap_or_else(|| temporary.clone()),
        );
    }
    let mut ids = BTreeMap::new();
    for (temporary, (identity, _, group, _)) in &candidates {
        let id = if owners[identity].len() == 1 {
            identity.clone()
        } else {
            format!(
                "ambiguous:{}:{}:{identity}",
                index.source_hash,
                group
                    .map(|group| format!("group:{group}"))
                    .unwrap_or_else(|| temporary.clone())
            )
        };
        ids.insert(temporary.clone(), id);
    }
    let mut parts = BTreeMap::<String, PlaybackPart>::new();
    for (note, tick, _, channel) in notes.iter_mut() {
        let temporary = note.part_id.clone();
        let (_, name, _, automatic_name) = &candidates[&temporary];
        let (program, changed) = if *channel == 9 {
            (None, false)
        } else {
            programs.at(*channel, *tick)
        };
        if !changed {
            if let Some(instrument) = automatic_name
                .as_deref()
                .and_then(crate::named_sample_instrument)
            {
                note.instrument = instrument;
            }
        } else {
            note.instrument = program
                .and_then(crate::program_sample_instrument)
                .unwrap_or_default();
            if automatic_name
                .as_deref()
                .and_then(crate::named_sample_instrument)
                == Some(ossia_types::SampleInstrument::Ukulele)
                && program.is_some_and(|program| matches!(program, 24 | 25))
            {
                note.instrument = ossia_types::SampleInstrument::Ukulele;
            }
        }
        note.part_id = ids[&temporary].clone();
        parts
            .entry(note.part_id.clone())
            .or_insert_with(|| PlaybackPart {
                id: note.part_id.clone(),
                name: name.clone(),
                instrument: note.instrument,
            });
    }
    for group in groups {
        group.part_id = group.part_id.as_ref().and_then(|id| ids.get(id)).cloned();
        if group.part_id.is_none() {
            let Some(start) = group.start_seconds else {
                continue;
            };
            let position = notes.partition_point(|(note, ..)| {
                note.start_seconds < start - super::ONSET_EPSILON_SECONDS
            });
            let matching = notes[position..]
                .iter()
                .take_while(|(note, ..)| note.start_seconds <= start + super::ONSET_EPSILON_SECONDS)
                .filter(|(note, ..)| group.pitches.contains(&note.pitch))
                .map(|(note, ..)| &note.part_id)
                .collect::<BTreeSet<_>>();
            if matching.len() == 1 {
                group.part_id = matching.into_iter().next().cloned();
            }
        }
    }
    parts.into_values().collect()
}
