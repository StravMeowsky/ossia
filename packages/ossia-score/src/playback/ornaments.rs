use super::engraved::moment_ticks;
use super::matching::{head_attacks, Attack, HeadAttack};
use super::timing::seconds_at;
use super::{MidiFile, ScoreIndex, ScoreLayout, ONSET_EPSILON_SECONDS};
use crate::{SvgObject, TokenKind};
use ossia_types::{GeometryPage, PlaybackNote, PlaybackNoteGroup};
use std::collections::{HashMap, HashSet};

const PULSE_SECONDS: f64 = 0.065;
const MAX_ARPEGGIO_STEP: f64 = 0.025;

pub(super) fn midi_is_articulated(index: &ScoreIndex) -> bool {
    crate::tokenize(index.source())
        .iter()
        .any(|token| token.kind == TokenKind::Cmd && token.raw == "\\articulate")
}

fn owners<'a, 'b>(
    ornament: &SvgObject,
    attacks: &'b HashMap<&str, HeadAttack<'a>>,
) -> Option<Vec<&'b HeadAttack<'a>>> {
    let mut found = Vec::new();
    let mut used = HashSet::new();
    for graphic in &ornament.owners {
        let owner = attacks.get(graphic.as_str())?;
        if owner.head.note?.grace_numerator != 0 {
            return None;
        }
        if used.insert(owner.attack) {
            found.push(owner);
        }
    }
    Some(found)
}

fn script(
    name: &str,
    owner: &HeadAttack<'_>,
    original: &PlaybackNote,
    window: f64,
) -> Option<Vec<PlaybackNote>> {
    let (lower, upper) = owner.head.ornament.as_deref()?.split_once(',')?;
    let neighbor = |delta: &str| {
        let pitch = f64::from(original.pitch) + delta.parse::<f64>().ok()?;
        (pitch.fract() == 0. && (0. ..=127.).contains(&pitch)).then_some(pitch as u8)
    };

    let trill;
    let (pattern, lengthened) = match name {
        "prallmordent" => (b"UPUPLP".as_slice(), None),
        "prallprall" => (b"UPUPUP".as_slice(), None),
        "downprall" => (b"UPUP".as_slice(), Some(0)),
        "prallup" => (b"UPUPUP".as_slice(), Some(4)),
        "turn" => (b"UPLP".as_slice(), None),
        "reverseturn" => (b"LPUP".as_slice(), None),
        "mordent" => (b"PLP".as_slice(), None),
        "prall" => (b"UPUP".as_slice(), None),
        "trill" => {
            let pairs = (window / (2. * PULSE_SECONDS)).floor().clamp(1., 512.) as usize;
            trill = b"UP".repeat(pairs);
            (trill.as_slice(), None)
        }
        _ => return None,
    };
    let pulse =
        PULSE_SECONDS.min(window / (pattern.len() + usize::from(lengthened.is_some())) as f64);
    if pulse <= ONSET_EPSILON_SECONDS {
        return None;
    }
    let mut offset = 0.;
    pattern
        .iter()
        .enumerate()
        .map(|(i, step)| {
            let pitch = match step {
                b'U' => neighbor(upper)?,
                b'L' => neighbor(lower)?,
                _ => original.pitch,
            };
            let duration = if i + 1 == pattern.len() {
                original.duration_seconds - offset
            } else {
                pulse * if lengthened == Some(i) { 2. } else { 1. }
            };
            let note = PlaybackNote {
                pitch,
                start_seconds: original.start_seconds + offset,
                duration_seconds: duration,
                ornament_anchor: Some(owner.anchor.to_owned()),
                ..original.clone()
            };
            offset += duration;
            Some(note)
        })
        .collect()
}

fn arpeggio(
    direction: &str,
    mut owners: Vec<&HeadAttack<'_>>,
    notes: &[Attack],
) -> Option<Vec<(usize, Vec<PlaybackNote>)>> {
    let first = owners.first()?.attack;
    if owners.len() < 2
        || owners
            .iter()
            .any(|owner| notes[owner.attack].1 != notes[first].1)
    {
        return None;
    }
    owners.sort_by_key(|owner| notes[owner.attack].0.pitch);
    if direction == "down" {
        owners.reverse();
    }
    let shortest = owners
        .iter()
        .map(|owner| notes[owner.attack].0.duration_seconds)
        .min_by(f64::total_cmp)?;
    let step = MAX_ARPEGGIO_STEP.min(shortest / (4 * (owners.len() - 1)) as f64);
    if step <= ONSET_EPSILON_SECONDS {
        return None;
    }
    Some(
        owners
            .into_iter()
            .enumerate()
            .map(|(i, owner)| {
                let original = &notes[owner.attack].0;
                let offset = i as f64 * step;
                (
                    owner.attack,
                    vec![PlaybackNote {
                        start_seconds: original.start_seconds + offset,
                        duration_seconds: original.duration_seconds - offset,
                        ornament_anchor: Some(owner.anchor.to_owned()),
                        ..original.clone()
                    }],
                )
            })
            .collect(),
    )
}

pub(super) fn expand(
    index: &ScoreIndex,
    layout: &ScoreLayout,
    midi: &MidiFile,
    ppqn: u64,
    pages: &[GeometryPage],
    groups: &mut [PlaybackNoteGroup],
    mut notes: Vec<Attack>,
) -> Vec<PlaybackNote> {
    let objects = layout.pages.iter().flat_map(|page| &page.objects);
    let mut seen = HashSet::new();
    let ornaments = objects
        .clone()
        .filter(|object| {
            !object.hit_only
                && matches!(object.kind.as_str(), "Script" | "Arpeggio")
                && object.ornament.is_some()
                && !super::fermatas::is_fermata(object)
                && object
                    .graphic_id
                    .as_deref()
                    .is_some_and(|id| seen.insert(id))
        })
        .collect::<Vec<_>>();
    let has_graces = objects.clone().any(super::graces::is_marked);
    if (ornaments.is_empty() && !has_graces) || midi_is_articulated(index) {
        return notes.into_iter().map(|(note, ..)| note).collect();
    }

    let midi_origin = super::engraved::midi_origin(index, layout, ppqn);
    let attacks = head_attacks(layout, pages, groups, &notes, ppqn, midi_origin);
    if has_graces {
        super::graces::realize(layout, &attacks, midi, ppqn, groups, &mut notes);
    }
    let mut claims = vec![0; notes.len()];
    let mut ornaments_to_apply = Vec::new();
    for ornament in ornaments {
        let Some(owners) = owners(ornament, &attacks) else {
            continue;
        };
        let name = ornament.ornament.as_deref().unwrap();
        let changes: Option<Vec<_>> = if ornament.kind == "Arpeggio" {
            arpeggio(name, owners, &notes)
        } else {
            owners
                .iter()
                .map(|owner| {
                    let (original, tick, ..) = &notes[owner.attack];
                    let timing = owner.head.timing?;
                    let length = moment_ticks(
                        timing.duration_numerator.into(),
                        timing.duration_denominator,
                        ppqn,
                    )?;
                    let written = seconds_at(tick + length, ppqn, midi) - original.start_seconds;
                    let sequence = script(
                        name,
                        owner,
                        original,
                        written.min(original.duration_seconds),
                    )?;
                    Some((owner.attack, sequence))
                })
                .collect()
        };
        if let Some(changes) = changes {
            for (index, _) in &changes {
                claims[*index] += 1;
            }
            ornaments_to_apply.push(changes);
        }
    }

    let mut changes = ornaments_to_apply
        .into_iter()
        .filter(|edits| edits.iter().all(|(i, _)| claims[*i] == 1))
        .flatten()
        .collect::<HashMap<_, _>>();
    let mut expanded = Vec::new();
    for (index, (note, ..)) in notes.into_iter().enumerate() {
        if let Some(sequence) = changes.remove(&index) {
            expanded.extend(sequence);
        } else {
            expanded.push(note);
        }
    }
    expanded
}
