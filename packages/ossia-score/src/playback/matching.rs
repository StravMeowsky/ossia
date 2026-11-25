use super::{engraved::tick, ONSET_EPSILON_SECONDS};
use crate::{ScoreLayout, SvgObject};
use ossia_types::{GeometryPage, PlaybackNote, PlaybackNoteGroup, SelectionTarget};
use std::collections::HashMap;

pub(super) type Attack = (PlaybackNote, u64, usize, u8);

pub(super) struct HeadAttack<'a> {
    pub anchor: &'a str,
    pub head: &'a SvgObject,
    pub attack: usize,
    pub group: usize,
}

/// Index engraved heads only when their group and MIDI attack agree.
pub(super) fn head_attacks<'a>(
    layout: &'a ScoreLayout,
    pages: &'a [GeometryPage],
    groups: &[PlaybackNoteGroup],
    notes: &[Attack],
    ppqn: u64,
    midi_origin: u64,
) -> HashMap<&'a str, HeadAttack<'a>> {
    let heads = layout
        .pages
        .iter()
        .flat_map(|page| &page.objects)
        .filter(|object| !object.hit_only && object.note.is_some())
        .filter_map(|object| Some((object.graphic_id.as_deref()?, object)))
        .collect::<HashMap<_, _>>();
    let mut groups_by_anchor = HashMap::<_, Vec<_>>::new();
    for (i, group) in groups.iter().enumerate() {
        for anchor in &group.element_ids {
            groups_by_anchor.entry(anchor.as_str()).or_default().push(i);
        }
    }
    let mut by_attack = HashMap::<_, Vec<_>>::new();
    for (i, (note, tick, ..)) in notes.iter().enumerate() {
        by_attack.entry((*tick, note.pitch)).or_default().push(i);
    }
    pages
        .iter()
        .flat_map(|page| &page.elements)
        .filter_map(|element| {
            let SelectionTarget::Element { element_id } = &element.region.target else {
                return None;
            };
            let groups_for_head = groups_by_anchor.get(element_id.as_str())?;
            Some(
                element
                    .graphic_ids
                    .iter()
                    .map(move |id| (id.as_str(), element_id.as_str(), groups_for_head)),
            )
        })
        .flatten()
        .filter_map(|(id, anchor, groups_for_head)| {
            let &head = heads.get(id)?;
            let engraved = head.note?;
            if engraved.pitch.fract() != 0. || !(0. ..=127.).contains(&engraved.pitch) {
                return None;
            }
            let time = tick(engraved, ppqn, midi_origin)?;
            let mut matching = groups_for_head.iter().flat_map(|&i| {
                let group = &groups[i];
                let pitch = match group.pitches.as_slice() {
                    [pitch] => Some(*pitch),
                    pitches => pitches
                        .contains(&(engraved.pitch as u8))
                        .then_some(engraved.pitch as u8),
                };
                pitch
                    .and_then(|pitch| by_attack.get(&(time, pitch)))
                    .into_iter()
                    .flatten()
                    .copied()
                    .filter(move |&attack| {
                        let (note, _, _, channel) = &notes[attack];
                        *channel != 9
                            && group.start_seconds.is_some_and(|start| {
                                (note.start_seconds - start).abs() <= ONSET_EPSILON_SECONDS
                            })
                            && group.part_id.as_ref() == Some(&note.part_id)
                    })
                    .map(move |attack| (attack, i))
            });
            let (attack, group) = matching.next()?;
            if matching.any(|(other, _)| attack != other) {
                return None;
            }
            Some((
                id,
                HeadAttack {
                    anchor,
                    head,
                    attack,
                    group,
                },
            ))
        })
        .collect()
}
