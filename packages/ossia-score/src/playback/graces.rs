use super::engraved::moment_ticks;
use super::matching::{Attack, HeadAttack};
use super::timing::seconds_at;
use crate::{MidiFile, ScoreLayout, SvgObject};
use ossia_types::PlaybackNoteGroup;
use std::collections::{BTreeMap, HashMap, HashSet};

const GRACE_PLAYHEAD_WINDOW_SECONDS: f64 = 0.065;

pub(super) fn is_marked(object: &SvgObject) -> bool {
    grace_kind(object).is_some()
}

fn grace_kind(object: &SvgObject) -> Option<&str> {
    object.note.filter(|note| note.grace_numerator < 0)?;
    object
        .ornament
        .as_deref()
        .filter(|kind| matches!(*kind, "appoggiatura" | "acciaccatura"))
}

struct Appoggiatura<'a> {
    grace: Vec<(&'a HeadAttack<'a>, f64, f64)>,
    principal: Vec<&'a HeadAttack<'a>>,
    anticipation: f64,
    beat: f64,
    resolution: f64,
}

impl Appoggiatura<'_> {
    fn attacks(&self) -> HashSet<usize> {
        self.principal
            .iter()
            .map(|head| head.attack)
            .chain(self.grace.iter().map(|(head, ..)| head.attack))
            .collect()
    }
}

fn schedule_playhead(plans: &mut [&Appoggiatura<'_>], groups: &mut [PlaybackNoteGroup]) {
    if plans.is_empty() {
        return;
    }
    let appoggiatura_groups = plans
        .iter()
        .flat_map(|plan| {
            plan.principal
                .iter()
                .map(|head| head.group)
                .chain(plan.grace.iter().map(|(head, ..)| head.group))
        })
        .collect::<HashSet<_>>();
    let mut other_onsets = groups
        .iter()
        .enumerate()
        .filter(|(i, group)| !appoggiatura_groups.contains(i) && !group.element_ids.is_empty())
        .filter_map(|(i, group)| Some((group.start_seconds?, i)))
        .collect::<Vec<_>>();
    other_onsets.sort_by(|a, b| a.0.total_cmp(&b.0));
    plans.sort_by(|a, b| a.beat.total_cmp(&b.beat));
    let mut beats = plans.chunk_by(|a, b| a.beat == b.beat).peekable();
    while let Some(beat_plans) = beats.next() {
        let beat = beat_plans[0].beat;
        let first_at_beat = other_onsets.partition_point(|(start, _)| *start < beat);
        let first_after_beat = other_onsets.partition_point(|(start, _)| *start <= beat);
        let next = other_onsets
            .get(first_after_beat)
            .map_or(f64::INFINITY, |(start, _)| *start)
            .min(beats.peek().map_or(f64::INFINITY, |next| next[0].beat));
        let shared = &other_onsets[first_at_beat..first_after_beat];
        let first_resolution = beat_plans
            .iter()
            .map(|plan| plan.resolution)
            .fold(f64::INFINITY, f64::min);
        if next > first_resolution && shared.iter().all(|&(_, i)| groups[i].pitches.is_empty()) {
            continue;
        }

        // Give both printed columns a turn before the next beat in another voice.
        let window = GRACE_PLAYHEAD_WINDOW_SECONDS.min((next.min(first_resolution) - beat) / 2.);
        let column_time = beat + window;
        let last_resolution = beat_plans
            .iter()
            .map(|plan| plan.resolution)
            .fold(beat, f64::max);
        for plan in beat_plans {
            for &(head, start, _) in &plan.grace {
                let fraction = (start - beat) / (last_resolution - beat);
                let visual_start = beat + window * fraction;
                if visual_start != start {
                    groups[head.group].playhead_seconds = Some(visual_start);
                }
            }
            for head in &plan.principal {
                groups[head.group].playhead_seconds = Some(column_time);
            }
        }
        for &(_, i) in shared {
            if !groups[i].pitches.is_empty() {
                groups[i].playhead_seconds = Some(column_time);
            }
        }
    }
}

fn plan<'a>(
    grace: &[&'a HeadAttack<'a>],
    principal: Vec<&'a HeadAttack<'a>>,
    notes: &[Attack],
    midi: &MidiFile,
    ppqn: u64,
) -> Option<Appoggiatura<'a>> {
    let main = &notes[principal.first()?.attack];
    let timing = principal[0].head.timing?;
    let beat_tick = main.1;
    let same_performer = |attack: &Attack| {
        (attack.2, attack.3, &attack.0.part_id) == (main.2, main.3, &main.0.part_id)
    };
    if principal.iter().any(|head| {
        let attack = &notes[head.attack];
        !same_performer(attack)
            || attack.1 != beat_tick
            || head.head.timing.is_none_or(|other| {
                (
                    other.duration_numerator,
                    other.duration_denominator,
                    other.dots,
                ) != (
                    timing.duration_numerator,
                    timing.duration_denominator,
                    timing.dots,
                )
            })
    }) {
        return None;
    }
    let tick_length = |head: &HeadAttack<'_>| {
        let timing = head.head.timing?;
        moment_ticks(
            timing.duration_numerator.into(),
            timing.duration_denominator,
            ppqn,
        )
    };
    let length = tick_length(principal[0])? as f64;
    let mut onsets = BTreeMap::<_, (u64, Vec<_>)>::new();
    for &head in grace {
        let attack = &notes[head.attack];
        if grace_kind(head.head) != Some("appoggiatura")
            || !same_performer(attack)
            || attack.1 >= beat_tick
        {
            return None;
        }
        let written = tick_length(head)?;
        let (shared, members) = onsets
            .entry(attack.1)
            .or_insert_with(|| (written, Vec::new()));
        if written == 0 || *shared != written {
            return None;
        }
        members.push(head);
    }
    let anticipation = seconds_at(*onsets.first_key_value()?.0, ppqn, midi);
    let beat = main.0.start_seconds;
    let written = onsets.values().map(|(ticks, _)| *ticks as f64).sum::<f64>();
    // Respect written grace values within the customary share of the first principal note.
    let share = if timing.dots == 0 { 0.5 } else { 2. / 3. };
    let borrowed = written.min(share * length);
    let resolution_tick = beat_tick.checked_add(borrowed.round() as u64)?;
    let resolution = seconds_at(resolution_tick, ppqn, midi);
    if resolution <= beat
        || principal.iter().any(|head| {
            let note = &notes[head.attack].0;
            note.start_seconds + note.duration_seconds <= resolution
        })
    {
        return None;
    }
    let mut played = 0.;
    let mut start = beat;
    let mut sequence = Vec::new();
    for (ticks, members) in onsets.into_values() {
        played += ticks as f64;
        let end_tick = beat_tick + (borrowed * played / written).round() as u64;
        let end = seconds_at(end_tick, ppqn, midi);
        if start >= end {
            return None;
        }
        sequence.extend(members.into_iter().map(|head| (head, start, end)));
        start = end;
    }
    Some(Appoggiatura {
        grace: sequence,
        principal,
        anticipation,
        beat,
        resolution,
    })
}

pub(super) fn realize(
    layout: &ScoreLayout,
    attacks: &HashMap<&str, HeadAttack<'_>>,
    midi: &MidiFile,
    ppqn: u64,
    groups: &mut [PlaybackNoteGroup],
    notes: &mut [Attack],
) {
    let mut principals = HashMap::<_, Vec<_>>::new();
    let mut seen = HashSet::new();
    for object in layout.pages.iter().flat_map(|page| &page.objects) {
        if !object.hit_only
            && object.note.is_some_and(|note| note.grace_numerator == 0)
            && !object.related.is_empty()
            && object
                .graphic_id
                .as_deref()
                .is_some_and(|id| seen.insert(id))
        {
            principals
                .entry(object.related.as_slice())
                .or_default()
                .push(object);
        }
    }
    let mut claims = vec![0; notes.len()];
    let plans = principals
        .into_iter()
        .filter_map(|(related, principal)| {
            let grace = related
                .iter()
                .map(|id| attacks.get(id.as_str()))
                .collect::<Option<Vec<_>>>()?;
            let principal = principal
                .iter()
                .map(|head| attacks.get(head.graphic_id.as_deref()?))
                .collect::<Option<Vec<_>>>()?;
            let plan = plan(&grace, principal, notes, midi, ppqn)?;
            let attacks = plan.attacks();
            for &attack in &attacks {
                claims[attack] += 1;
            }
            Some((plan, attacks))
        })
        .collect::<Vec<_>>();

    let mut releases = attacks
        .values()
        .filter(|head| head.head.note.is_some_and(|note| note.grace_numerator == 0))
        .map(|head| {
            let note = &notes[head.attack].0;
            (note.start_seconds + note.duration_seconds, head)
        })
        .collect::<Vec<_>>();
    releases.sort_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.1.anchor.cmp(b.1.anchor)));
    let mut restored = HashMap::new();
    let mut edits = HashMap::new();
    let mut grace_anchors = HashSet::new();
    let mut delayed_anchors = HashSet::new();
    let mut applied = Vec::new();
    for (plan, _) in plans
        .iter()
        .filter(|(_, attacks)| attacks.iter().all(|&i| claims[i] == 1))
    {
        applied.push(plan);
        let main = plan.principal[0];
        let (_, _, track, channel) = &notes[main.attack];
        // LilyPond clips the preceding note at the first MIDI grace attack.
        let first = releases.partition_point(|(end, _)| *end < plan.anticipation - 1e-9);
        for &(_, previous) in releases[first..]
            .iter()
            .take_while(|(end, _)| *end <= plan.anticipation + 1e-9)
        {
            let (note, _, prev_track, prev_channel) = &notes[previous.attack];
            if (prev_track, prev_channel) == (track, channel)
                && groups[previous.group].voice == groups[main.group].voice
                && note.start_seconds < plan.anticipation
            {
                restored.insert(previous.attack, plan.beat);
                groups[previous.group].duration_seconds = Some(plan.beat - note.start_seconds);
            }
        }
        for &(head, start, end) in &plan.grace {
            edits.insert(head.attack, (head, start, Some(end)));
            grace_anchors.insert(head.anchor);
        }
        for &head in &plan.principal {
            edits.insert(head.attack, (head, plan.resolution, None));
            delayed_anchors.insert(head.anchor);
        }
    }
    for (i, end) in restored {
        let note = &mut notes[i].0;
        note.duration_seconds = end - note.start_seconds;
    }
    for (i, (head, start, end)) in edits {
        let note = &mut notes[i].0;
        let end = end.unwrap_or(note.start_seconds + note.duration_seconds);
        note.start_seconds = start;
        note.duration_seconds = end - start;
        note.ornament_anchor = Some(head.anchor.to_owned());
        groups[head.group].start_seconds = Some(start);
        groups[head.group].duration_seconds = Some(note.duration_seconds);
    }
    schedule_playhead(&mut applied, groups);
    for head in attacks
        .values()
        .filter(|head| grace_kind(head.head) == Some("acciaccatura"))
    {
        notes[head.attack].0.ornament_anchor = Some(head.anchor.to_owned());
    }
    // A grace head is printed left of the beat. Its delayed resolution stays on
    // the original beat's column, behind other voices starting at the new beat.
    let playhead_priority = |group: &PlaybackNoteGroup| {
        let contains = |anchors: &HashSet<&str>| {
            group
                .element_ids
                .iter()
                .any(|id| anchors.contains(id.as_str()))
        };
        if contains(&grace_anchors) {
            0
        } else if contains(&delayed_anchors) {
            2
        } else {
            1
        }
    };
    groups.sort_by(|a, b| {
        a.start_seconds
            .unwrap_or(f64::INFINITY)
            .total_cmp(&b.start_seconds.unwrap_or(f64::INFINITY))
            .then_with(|| playhead_priority(a).cmp(&playhead_priority(b)))
            .then_with(|| a.element_ids.cmp(&b.element_ids))
    });
}
