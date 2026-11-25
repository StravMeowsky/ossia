use super::timing::seconds_at;
use crate::{EngravedNote, MidiFile, ScoreLayout};
use ossia_types::{GeometryPage, PlaybackNoteGroup, SelectionTarget};
use std::collections::{HashMap, HashSet};

struct Head<'a> {
    id: &'a str,
    note: EngravedNote,
    staff: Option<usize>,
    voice: Option<String>,
}

fn tick(note: EngravedNote, ppqn: u64) -> Option<u64> {
    if note.denominator == 0 {
        return None;
    }
    let numerator = u128::try_from(note.numerator)
        .ok()?
        .checked_mul(4 * u128::from(ppqn))?;
    let denominator = u128::from(note.denominator);
    (numerator % denominator == 0)
        .then(|| (numerator / denominator).try_into().ok())
        .flatten()
}

/// Replace positional guesses only when engraved moments prove one constant pitch offset per voice.
pub(super) fn note_groups(
    layout: &ScoreLayout,
    pages: &[GeometryPage],
    midi: &MidiFile,
    ppqn: u64,
    hints: &HashMap<String, u8>,
    groups: &mut Vec<PlaybackNoteGroup>,
) {
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
            if notes
                .iter()
                .any(|other| other.pitch != note.pitch || tick(**other, ppqn) != tick(note, ppqn))
            {
                return None;
            }
            Some(Head {
                id: element_id,
                note,
                staff: element.region.staff_index,
                voice: element.region.voice.clone(),
            })
        })
        .collect::<Vec<_>>();
    if heads.is_empty() {
        return;
    }
    let by_id = heads
        .iter()
        .map(|head| (head.id, head))
        .collect::<HashMap<_, _>>();
    let mut continuations = HashSet::new();
    for mut chain in super::groups::tied_note_chains(pages) {
        if chain.iter().any(|id| !by_id.contains_key(id.as_str())) {
            continue;
        }
        chain.sort_by_key(|id| tick(by_id[id.as_str()].note, ppqn));
        continuations.extend(chain.into_iter().skip(1));
    }
    let mut offsets = HashMap::<(Option<usize>, Option<String>), HashSet<i16>>::new();
    for head in &heads {
        if head.note.grace_numerator != 0 || continuations.contains(head.id) {
            continue;
        }
        if head.note.pitch.fract() != 0. {
            continue;
        }
        let Some(time) = tick(head.note, ppqn) else {
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
            .entry((head.staff, head.voice.clone()))
            .and_modify(|values| values.retain(|value| candidates.contains(value)))
            .or_insert(candidates);
    }
    let mut replacements = Vec::new();
    let mut replaced = HashSet::new();
    for head in &heads {
        let Some(offsets) = offsets
            .get(&(head.staff, head.voice.clone()))
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
        let Some(time) = tick(head.note, ppqn) else {
            continue;
        };
        let matching = midi
            .tracks
            .iter()
            .flat_map(|track| track.notes.iter().map(move |note| (track.track, note)))
            .filter(|(_, note)| {
                if note.note != pitch as u8 {
                    return false;
                }
                if head.note.grace_numerator == 0 {
                    return note.time == time;
                }
                let grace = head.note.grace_numerator as f64 / head.note.grace_denominator as f64
                    * 4.
                    * ppqn as f64;
                note.time < time && note.time as f64 >= time as f64 + grace
            })
            .collect::<Vec<_>>();
        let Some((track, note)) = matching.first().copied() else {
            continue;
        };
        if matching.iter().any(|(_, other)| other.time != note.time) {
            continue;
        }
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
            voice: head.voice.clone(),
            start_seconds: Some(start),
            duration_seconds,
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
