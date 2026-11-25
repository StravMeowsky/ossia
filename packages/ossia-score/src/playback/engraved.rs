use super::timing::seconds_at;
use crate::{EngravedNote, MidiFile, ScoreIndex, ScoreLayout};
use ossia_types::{GeometryPage, PlaybackNoteGroup, SelectionTarget};
use std::collections::{HashMap, HashSet};

struct Head<'a> {
    id: &'a str,
    note: EngravedNote,
    staff: Option<usize>,
    voice: Option<&'a str>,
}

pub(super) fn tick(note: EngravedNote, ppqn: u64, midi_origin: u64) -> Option<u64> {
    if note.grace_numerator == 0 {
        return moment_ticks(i128::from(note.numerator), note.denominator, ppqn)?
            .checked_add(midi_origin);
    }
    (grace_ticks(note, ppqn)? + i128::from(midi_origin))
        .try_into()
        .ok()
}

fn grace_ticks(note: EngravedNote, ppqn: u64) -> Option<i128> {
    if note.denominator == 0 || note.grace_denominator == 0 {
        return None;
    }
    // LilyPond's MIDI writer scales grace time by 9/40 and truncates the full moment.
    let main = i128::from(note.numerator) * 40;
    let grace = i128::from(note.grace_numerator) * 9;
    let numerator = (main.checked_mul(i128::from(note.grace_denominator))?)
        .checked_add(grace.checked_mul(i128::from(note.denominator))?)?
        .checked_mul(4 * i128::from(ppqn))?;
    let denominator = i128::from(note.denominator)
        .checked_mul(i128::from(note.grace_denominator))?
        .checked_mul(40)?;
    Some(numerator / denominator)
}

/// LilyPond offsets the full MIDI score when the first grace begins before tick zero.
pub(super) fn midi_origin(index: &ScoreIndex, layout: &ScoreLayout, ppqn: u64) -> u64 {
    let leading = layout
        .pages
        .iter()
        .flat_map(|page| &page.objects)
        .filter(|object| !object.hit_only)
        .filter_map(|object| {
            let note = object.note?;
            (note.numerator == 0 && note.grace_numerator < 0).then_some((object, note))
        });
    if !leading
        .clone()
        .any(|(object, _)| super::graces::is_marked(object))
        || super::ornaments::midi_is_articulated(index)
    {
        return 0;
    }
    leading
        .filter_map(|(_, note)| grace_ticks(note, ppqn))
        .min()
        .and_then(|tick| (-tick).try_into().ok())
        .unwrap_or(0)
}

pub(super) fn moment_ticks(numerator: i128, denominator: u64, ppqn: u64) -> Option<u64> {
    if denominator == 0 {
        return None;
    }
    let numerator = u128::try_from(numerator)
        .ok()?
        .checked_mul(4 * u128::from(ppqn))?;
    let denominator = u128::from(denominator);
    (numerator % denominator == 0)
        .then(|| (numerator / denominator).try_into().ok())
        .flatten()
}

fn rest_groups(
    layout: &ScoreLayout,
    pages: &[GeometryPage],
    midi: &MidiFile,
    ppqn: u64,
    midi_origin: u64,
    groups: &mut Vec<PlaybackNoteGroup>,
) {
    let timings = layout
        .pages
        .iter()
        .flat_map(|page| &page.objects)
        .filter(|object| !object.hit_only)
        .filter_map(|object| Some((object.graphic_id.as_deref()?, object.timing?)))
        .collect::<HashMap<_, _>>();
    let mut replacements = HashMap::<String, PlaybackNoteGroup>::new();
    for element in pages.iter().flat_map(|page| &page.elements) {
        if !matches!(
            element.region.element_kind.as_deref(),
            Some("Rest" | "MultiMeasureRest")
        ) {
            continue;
        }
        let SelectionTarget::Element { element_id } = &element.region.target else {
            continue;
        };
        let mut values = element
            .graphic_ids
            .iter()
            .filter_map(|id| timings.get(id.as_str()));
        let Some(timing) = values.next() else {
            continue;
        };
        if values.any(|other| {
            other.numerator != timing.numerator
                || other.denominator != timing.denominator
                || other.duration_numerator != timing.duration_numerator
                || other.duration_denominator != timing.duration_denominator
        }) {
            continue;
        }
        let (Some(start), Some(length)) = (
            moment_ticks(i128::from(timing.numerator), timing.denominator, ppqn)
                .and_then(|time| time.checked_add(midi_origin)),
            moment_ticks(
                i128::from(timing.duration_numerator),
                timing.duration_denominator,
                ppqn,
            ),
        ) else {
            continue;
        };
        let Some(end) = start.checked_add(length).filter(|_| length > 0) else {
            continue;
        };
        let start_seconds = seconds_at(start, ppqn, midi);
        // A multi-measure rest can emit several grobs for one source event.
        // Its duration belongs to the earliest occurrence, not to each copy.
        if replacements.get(element_id).is_some_and(|group| {
            group
                .start_seconds
                .is_some_and(|time| time <= start_seconds)
        }) {
            continue;
        }
        replacements.insert(
            element_id.clone(),
            PlaybackNoteGroup {
                element_ids: vec![element_id.clone()],
                pitches: Vec::new(),
                staff_index: element.region.staff_index,
                voice: element.region.voice.clone(),
                start_seconds: Some(start_seconds),
                duration_seconds: Some(seconds_at(end, ppqn, midi) - start_seconds),
                playhead_seconds: None,
                part_id: None,
            },
        );
    }
    groups.retain_mut(|group| {
        if group.element_ids.is_empty() {
            return true;
        }
        group
            .element_ids
            .retain(|id| !replacements.contains_key(id));
        !group.element_ids.is_empty()
    });
    groups.extend(replacements.into_values());
}

/// Replace positional guesses only when engraved moments prove one constant pitch offset per voice.
pub(super) fn note_groups(
    index: &ScoreIndex,
    layout: &ScoreLayout,
    pages: &[GeometryPage],
    midi: &MidiFile,
    ppqn: u64,
    hints: &HashMap<String, u8>,
    groups: &mut Vec<PlaybackNoteGroup>,
) {
    let midi_origin = midi_origin(index, layout, ppqn);
    rest_groups(layout, pages, midi, ppqn, midi_origin, groups);
    let muted = groups
        .iter()
        .filter(|group| group.pitches.is_empty())
        .flat_map(|group| group.element_ids.iter().map(String::as_str))
        .collect::<HashSet<_>>();
    let objects = layout
        .pages
        .iter()
        .flat_map(|page| &page.objects)
        .filter(|object| !object.hit_only)
        .filter_map(|object| Some((object.graphic_id.as_deref()?, object.note?)))
        .collect::<HashMap<_, _>>();
    let heads = pages
        .iter()
        .flat_map(|page| &page.elements)
        .filter_map(|element| {
            if !matches!(
                element.region.element_kind.as_deref(),
                Some("NoteHead" | "TabNoteHead")
            ) {
                return None;
            }
            let SelectionTarget::Element { element_id } = &element.region.target else {
                return None;
            };
            if muted.contains(element_id.as_str()) {
                return None;
            }
            let notes = element
                .graphic_ids
                .iter()
                .filter_map(|id| objects.get(id.as_str()))
                .collect::<Vec<_>>();
            let note = **notes.first()?;
            if notes.iter().any(|other| {
                other.pitch != note.pitch
                    || tick(**other, ppqn, midi_origin) != tick(note, ppqn, midi_origin)
            }) {
                return None;
            }
            Some(Head {
                id: element_id,
                note,
                staff: element.region.staff_index,
                voice: element.region.voice.as_deref(),
            })
        })
        .collect::<Vec<_>>();
    if heads.is_empty() {
        return;
    }
    let mut voice_staves = HashMap::<_, Option<usize>>::new();
    for staff in &index.staves {
        for voice in &staff.voices {
            voice_staves
                .entry(voice.as_str())
                .and_modify(|owner| *owner = owner.filter(|i| *i == staff.staff_index))
                .or_insert(Some(staff.staff_index));
        }
    }
    // Staff changes move the printed head while keeping the same source voice.
    let source_staff = |head: &Head<'_>| {
        head.voice
            .and_then(|voice| voice_staves.get(voice).copied().flatten())
            .or(head.staff)
    };
    let by_id = heads
        .iter()
        .map(|head| (head.id, head))
        .collect::<HashMap<_, _>>();
    let mut continuations = HashSet::new();
    for mut chain in super::groups::tied_note_chains(pages) {
        if chain.iter().any(|id| !by_id.contains_key(id.as_str())) {
            continue;
        }
        chain.sort_by_key(|id| tick(by_id[id.as_str()].note, ppqn, midi_origin));
        continuations.extend(chain.into_iter().skip(1));
    }
    let mut offsets = HashMap::<(Option<usize>, Option<&str>), HashSet<i16>>::new();
    for head in &heads {
        if head.note.grace_numerator != 0 || continuations.contains(head.id) {
            continue;
        }
        if head.note.pitch.fract() != 0. {
            continue;
        }
        let Some(time) = tick(head.note, ppqn, midi_origin) else {
            continue;
        };
        let candidates = midi
            .tracks
            .iter()
            .flat_map(|track| &track.notes)
            .filter(|note| {
                note.time == time && hints.get(head.id).is_none_or(|pitch| *pitch == note.note)
            })
            .map(|note| i16::from(note.note) - head.note.pitch as i16)
            .collect::<HashSet<_>>();
        offsets
            .entry((source_staff(head), head.voice))
            .and_modify(|values| values.retain(|value| candidates.contains(value)))
            .or_insert(candidates);
    }
    let mut replacements = Vec::new();
    let mut replaced = HashSet::new();
    for head in &heads {
        let Some(offsets) = offsets
            .get(&(source_staff(head), head.voice))
            .filter(|set| set.len() == 1)
        else {
            continue;
        };
        let pitch = head.note.pitch + f64::from(*offsets.iter().next().unwrap());
        if pitch.fract() != 0. || !(0. ..=127.).contains(&pitch) {
            continue;
        }
        if continuations.contains(head.id) {
            replaced.insert(head.id);
            continue;
        }
        let Some(time) = tick(head.note, ppqn, midi_origin) else {
            continue;
        };
        let matching = midi
            .tracks
            .iter()
            .flat_map(|track| track.notes.iter().map(move |note| (track.track, note)))
            .filter(|(_, note)| note.note == pitch as u8 && note.time == time)
            .collect::<Vec<_>>();
        let Some((track, note)) = matching.first().copied() else {
            continue;
        };
        let start = seconds_at(note.time, ppqn, midi);
        // Unison performers can have different releases; their shared pitch and
        // attack are certain even when this head's performer is unresolved.
        let duration_seconds = note
            .duration
            .filter(|duration| {
                matching
                    .iter()
                    .all(|(_, other)| other.duration == Some(*duration))
            })
            .map(|duration| seconds_at(note.time + duration, ppqn, midi) - start);
        // Accurate groups can already join notation and TAB for one source note.
        if groups.iter().any(|group| {
            group.element_ids.iter().any(|id| id == head.id)
                && group.element_ids.iter().all(|id| {
                    by_id
                        .get(id.as_str())
                        .is_none_or(|other| other.voice == head.voice)
                })
                && group.pitches == [pitch as u8]
                && group
                    .start_seconds
                    .is_some_and(|time| (time - start).abs() <= super::ONSET_EPSILON_SECONDS)
                && duration_seconds.is_none_or(|duration| {
                    group.duration_seconds.is_some_and(|length| {
                        (length - duration).abs() <= super::ONSET_EPSILON_SECONDS
                    })
                })
        }) {
            continue;
        }
        replacements.push(PlaybackNoteGroup {
            element_ids: vec![head.id.to_owned()],
            pitches: vec![pitch as u8],
            staff_index: head.staff,
            voice: head.voice.map(str::to_owned),
            start_seconds: Some(start),
            duration_seconds,
            playhead_seconds: None,
            part_id: (matching.len() == 1).then(|| format!("{track}:{}", note.channel)),
        });
        replaced.insert(head.id);
    }
    groups.retain_mut(|group| {
        if group.element_ids.is_empty() {
            return true;
        }
        group
            .element_ids
            .retain(|id| !replaced.contains(id.as_str()));
        !group.element_ids.is_empty()
    });
    groups.extend(replacements);
}

#[cfg(test)]
#[path = "../../tests/unit/playback_engraved.rs"]
mod tests;
