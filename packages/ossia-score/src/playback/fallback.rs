use super::timing::{measure_times, midi_clusters, seconds_at};
use super::SourcePlaybackEvent;
use crate::{PitchEventKind, ScoreLayout};
use ossia_types::{GeometryElement, PlaybackNoteGroup, PlaybackSystemSpan, SelectionTarget};
use std::collections::{BTreeMap, HashMap, HashSet};

const FALLBACK_ONSET_SECONDS: f64 = 0.02;

pub(super) struct ElementHints<'a> {
    pub pitches: &'a HashMap<String, u8>,
    pub staves: &'a HashMap<String, usize>,
}

pub(super) fn fallback_note_groups(
    layout: &ScoreLayout,
    pages: &[ossia_types::GeometryPage],
    midi: &crate::MidiFile,
    timing: (u64, Option<u64>),
    groups: &mut Vec<PlaybackNoteGroup>,
    covered: &HashSet<String>,
    element_hints: &ElementHints<'_>,
) {
    let (ppqn, pickup) = timing;
    let last_tick = midi
        .tracks
        .iter()
        .flat_map(|track| &track.notes)
        .map(|note| note.time + note.duration.unwrap_or(0))
        .max()
        .unwrap_or(0);
    let Some(measure_times) = measure_times(midi, ppqn, last_tick, pickup) else {
        return;
    };
    let measures: HashMap<_, _> = measure_times
        .iter()
        .map(|time| (time.measure, *time))
        .collect();
    let clusters = midi_clusters(midi, ppqn, pickup).ok();
    let Some(clusters) = clusters else {
        return;
    };
    let mut buckets = BTreeMap::<
        (u32, usize, u32, i64, Option<usize>, Option<String>),
        (f64, Vec<&GeometryElement>),
    >::new();
    for page in pages {
        for element in &page.elements {
            if !matches!(
                element.region.element_kind.as_deref(),
                Some("NoteHead" | "TabNoteHead")
            ) {
                continue;
            }
            let SelectionTarget::Element { element_id } = &element.region.target else {
                continue;
            };
            if covered.contains(element_id) {
                continue;
            }
            let x = (element.bounds.x0 + element.bounds.x1) / 2.;
            let Some(position) = super::geometry::measure_position(layout, page, element) else {
                continue;
            };
            let Some(time) = measures.get(&position.measure) else {
                continue;
            };
            let expected =
                time.start_seconds + position.within * (time.end_seconds - time.start_seconds);
            let bucket_x = (x * 2.).round() as i64;
            buckets
                .entry((
                    page.page,
                    position.system_index,
                    position.measure,
                    bucket_x,
                    element.region.staff_index,
                    element.region.voice.clone(),
                ))
                .or_insert_with(|| (expected, Vec::new()))
                .1
                .push(element);
        }
    }
    for ((_, _, measure, _, staff_index, voice), (expected, elements)) in buckets {
        let Some(nearest) = clusters
            .iter()
            .filter(|cluster| cluster.measure == Some(measure))
            .map(|cluster| (seconds_at(cluster.start_tick, ppqn, midi), cluster))
            .min_by(|a, b| (a.0 - expected).abs().total_cmp(&(b.0 - expected).abs()))
        else {
            continue;
        };
        let start = nearest.0;
        let mut chosen = Vec::new();
        for cluster in clusters
            .iter()
            .filter(|cluster| cluster.measure == Some(measure))
        {
            let cluster_start = seconds_at(cluster.start_tick, ppqn, midi);
            if (cluster_start - start).abs() <= FALLBACK_ONSET_SECONDS {
                chosen.push(cluster);
            }
        }
        let Some(first) = chosen.first() else {
            continue;
        };
        let start_seconds = seconds_at(first.start_tick, ppqn, midi);
        let end_seconds = chosen
            .iter()
            .map(|cluster| seconds_at(cluster.start_tick + cluster.duration_tick, ppqn, midi))
            .fold(start_seconds, f64::max);
        let mut pitches = chosen
            .iter()
            .flat_map(|cluster| cluster.pitches.iter().copied())
            .collect::<Vec<_>>();
        pitches.sort_unstable();
        pitches.dedup();
        // Known source pitches keep noteheads separate and preserve their source staff.
        let mut source_groups = BTreeMap::<u8, Vec<String>>::new();
        for element in &elements {
            let SelectionTarget::Element { element_id } = &element.region.target else {
                continue;
            };
            if let Some(pitch) = element_hints.pitches.get(element_id) {
                source_groups
                    .entry(*pitch)
                    .or_default()
                    .push(element_id.clone());
            }
        }
        if source_groups.values().map(Vec::len).sum::<usize>() == elements.len() {
            for (pitch, element_ids) in source_groups {
                let target = clusters
                    .iter()
                    .filter(|cluster| {
                        cluster.measure == Some(measure) && cluster.pitches.contains(&pitch)
                    })
                    .map(|cluster| (seconds_at(cluster.start_tick, ppqn, midi), cluster))
                    .min_by(|a, b| (a.0 - expected).abs().total_cmp(&(b.0 - expected).abs()));
                let (group_start, group_end) =
                    target.map_or((start_seconds, end_seconds), |(start, cluster)| {
                        (
                            start,
                            seconds_at(cluster.start_tick + cluster.duration_tick, ppqn, midi),
                        )
                    });
                let source_staff = element_ids
                    .iter()
                    .find_map(|element_id| element_hints.staves.get(element_id).copied())
                    .or(staff_index);
                push_group(
                    groups,
                    element_ids,
                    vec![pitch],
                    source_staff,
                    voice.clone(),
                    group_start,
                    group_end,
                );
            }
            continue;
        }
        let element_ids = elements
            .into_iter()
            .filter_map(|element| match &element.region.target {
                SelectionTarget::Element { element_id } => Some(element_id.clone()),
                SelectionTarget::Measure { .. } => None,
            })
            .collect::<Vec<_>>();
        if element_ids.is_empty() || pitches.is_empty() {
            continue;
        }
        push_group(
            groups,
            element_ids,
            pitches,
            staff_index,
            voice,
            start_seconds,
            end_seconds,
        );
    }
}

pub(super) fn visual_note_groups(
    pages: &[ossia_types::GeometryPage],
    midi: &crate::MidiFile,
    ppqn: u64,
    groups: &mut Vec<PlaybackNoteGroup>,
    element_hints: &ElementHints<'_>,
    systems: &[PlaybackSystemSpan],
) {
    if systems.is_empty() {
        return;
    }
    let mut performance = performance_onsets(midi, ppqn);
    if performance.is_empty() {
        return;
    }
    performance.sort_by(|a, b| a.start_seconds.total_cmp(&b.start_seconds));
    let mut buckets = BTreeMap::<(u32, usize, Option<usize>, i64), VisualOnset>::new();
    for page in pages {
        for element in &page.elements {
            if !matches!(
                element.region.element_kind.as_deref(),
                Some("NoteHead" | "TabNoteHead")
            ) {
                continue;
            }
            let SelectionTarget::Element { element_id } = &element.region.target else {
                continue;
            };
            let x = (element.bounds.x0 + element.bounds.x1) / 2.;
            let y = (element.bounds.y0 + element.bounds.y1) / 2.;
            let Some(system) = systems
                .iter()
                .filter(|system| system.page == page.page)
                .min_by(|left, right| {
                    interval_distance(y, left.y0, left.y1)
                        .total_cmp(&interval_distance(y, right.y0, right.y1))
                        .then_with(|| {
                            interval_distance(x, left.x0, left.x1)
                                .total_cmp(&interval_distance(x, right.x0, right.x1))
                        })
                })
            else {
                continue;
            };
            let progress = ((x - system.x0) / (system.x1 - system.x0).max(0.001)).clamp(0., 1.);
            let expected =
                system.start_seconds + progress * (system.end_seconds - system.start_seconds);
            let staff_index = element.region.staff_index;
            let key = (
                page.page,
                system.system_index,
                staff_index,
                (x * 2.).round() as i64,
            );
            let onset = buckets.entry(key).or_insert_with(|| VisualOnset {
                page: page.page,
                system_index: system.system_index,
                x,
                expected,
                element_ids: Vec::new(),
                pitches: HashSet::new(),
                staff_index,
                voice: element.region.voice.clone(),
            });
            onset.element_ids.push(element_id.clone());
            if let Some(pitch) = element_hints.pitches.get(element_id) {
                onset.pitches.insert(*pitch);
            }
        }
    }
    let mut by_staff = BTreeMap::<Option<usize>, Vec<VisualOnset>>::new();
    for onset in buckets.into_values() {
        by_staff.entry(onset.staff_index).or_default().push(onset);
    }
    let mut staffs = by_staff.into_iter().collect::<Vec<_>>();
    let anchor = (0..staffs.len())
        .min_by_key(|index| staffs[*index].1.len().abs_diff(performance.len()))
        .unwrap();
    let anchor_mapping = align_onsets(&staffs[anchor].1, &performance);
    push_visual_groups(groups, &staffs[anchor].1, &performance, &anchor_mapping);
    if staffs.len() > 1 {
        let mut anchors = BTreeMap::<(u32, usize), Vec<(f64, usize)>>::new();
        for (index, onset) in staffs[anchor].1.iter().enumerate() {
            let Some(performance_index) = anchor_mapping[index] else {
                continue;
            };
            anchors
                .entry((onset.page, onset.system_index))
                .or_default()
                .push((onset.x, performance_index));
        }
        for (staff_index, (_, visual)) in staffs.drain(..).enumerate() {
            if staff_index == anchor {
                continue;
            }
            let mut by_system = BTreeMap::<(u32, usize), Vec<&VisualOnset>>::new();
            for onset in &visual {
                by_system
                    .entry((onset.page, onset.system_index))
                    .or_default()
                    .push(onset);
            }
            let mut mapped = BTreeMap::<usize, Option<&VisualOnset>>::new();
            for (key, onsets) in by_system {
                let Some(reference) = anchors.get(&key) else {
                    continue;
                };
                let min_x = onsets.iter().map(|onset| onset.x).fold(f64::MAX, f64::min);
                let max_x = onsets.iter().map(|onset| onset.x).fold(f64::MIN, f64::max);
                let ref_min = reference.iter().map(|(x, _)| *x).fold(f64::MAX, f64::min);
                let ref_max = reference.iter().map(|(x, _)| *x).fold(f64::MIN, f64::max);
                for onset in onsets {
                    let Some((_, performance_index)) = reference.iter().min_by(|left, right| {
                        let left = (normalized_x(onset.x, min_x, max_x)
                            - normalized_x(left.0, ref_min, ref_max))
                        .abs();
                        let right = (normalized_x(onset.x, min_x, max_x)
                            - normalized_x(right.0, ref_min, ref_max))
                        .abs();
                        left.total_cmp(&right)
                    }) else {
                        continue;
                    };
                    add_unique_onset(&mut mapped, *performance_index, onset);
                }
            }
            for (performance_index, onset) in mapped {
                let Some(onset) = onset else { continue };
                let mut element_ids = onset.element_ids.clone();
                element_ids.sort();
                element_ids.dedup();
                let performed = &performance[performance_index];
                groups.push(PlaybackNoteGroup {
                    element_ids,
                    pitches: performed.pitches.clone(),
                    staff_index: onset.staff_index,
                    voice: onset.voice.clone(),
                    start_seconds: Some(performed.start_seconds),
                    duration_seconds: Some(performed.end_seconds - performed.start_seconds),
                    part_id: None,
                });
            }
        }
    }
    for performed in performance {
        let missing = performed
            .pitches
            .iter()
            .copied()
            .filter(|pitch| {
                !groups.iter().any(|group| {
                    group.pitches.contains(pitch)
                        && group.start_seconds.is_some_and(|value| {
                            (value - performed.start_seconds).abs() <= FALLBACK_ONSET_SECONDS
                        })
                })
            })
            .collect::<Vec<_>>();
        if missing.is_empty() {
            continue;
        }
        groups.push(PlaybackNoteGroup {
            element_ids: Vec::new(),
            pitches: missing,
            staff_index: None,
            voice: None,
            start_seconds: Some(performed.start_seconds),
            duration_seconds: Some(performed.end_seconds - performed.start_seconds),
            part_id: None,
        });
    }
}

fn add_unique_onset<'a>(
    mapped: &mut BTreeMap<usize, Option<&'a VisualOnset>>,
    performance_index: usize,
    onset: &'a VisualOnset,
) {
    use std::collections::btree_map::Entry;
    match mapped.entry(performance_index) {
        Entry::Vacant(entry) => {
            entry.insert(Some(onset));
        }
        Entry::Occupied(mut entry) => {
            entry.insert(None);
        }
    }
}

fn interval_distance(value: f64, start: f64, end: f64) -> f64 {
    if value < start {
        start - value
    } else if value > end {
        value - end
    } else {
        0.
    }
}

fn normalized_x(value: f64, min: f64, max: f64) -> f64 {
    if max > min {
        (value - min) / (max - min)
    } else {
        0.5
    }
}

fn push_visual_groups(
    groups: &mut Vec<PlaybackNoteGroup>,
    visual: &[VisualOnset],
    performance: &[PerformanceOnset],
    mapping: &[Option<usize>],
) {
    for (index, onset) in visual.iter().enumerate() {
        if let Some(performance_index) = mapping[index] {
            let performed = &performance[performance_index];
            let mut element_ids = onset.element_ids.clone();
            element_ids.sort();
            element_ids.dedup();
            groups.push(PlaybackNoteGroup {
                element_ids,
                pitches: performed.pitches.clone(),
                staff_index: onset.staff_index,
                voice: onset.voice.clone(),
                start_seconds: Some(performed.start_seconds),
                duration_seconds: Some(performed.end_seconds - performed.start_seconds),
                part_id: None,
            });
        }
    }
}

struct VisualOnset {
    page: u32,
    system_index: usize,
    x: f64,
    expected: f64,
    element_ids: Vec<String>,
    pitches: HashSet<u8>,
    staff_index: Option<usize>,
    voice: Option<String>,
}

#[cfg(test)]
#[path = "../../tests/unit/playback_fallback.rs"]
mod tests;

struct PerformanceOnset {
    start_seconds: f64,
    end_seconds: f64,
    pitches: Vec<u8>,
}

fn performance_onsets(midi: &crate::MidiFile, ppqn: u64) -> Vec<PerformanceOnset> {
    let mut by_tick = BTreeMap::<u64, (u64, Vec<u8>)>::new();
    for track in &midi.tracks {
        for note in &track.notes {
            let end = note.time + note.duration.unwrap_or(0);
            let entry = by_tick.entry(note.time).or_insert((end, Vec::new()));
            entry.0 = entry.0.max(end);
            entry.1.push(note.note);
        }
    }
    by_tick
        .into_iter()
        .map(|(start, (end, mut pitches))| {
            pitches.sort_unstable();
            pitches.dedup();
            PerformanceOnset {
                start_seconds: seconds_at(start, ppqn, midi),
                end_seconds: seconds_at(end, ppqn, midi),
                pitches,
            }
        })
        .collect()
}

fn align_onsets(visual: &[VisualOnset], performance: &[PerformanceOnset]) -> Vec<Option<usize>> {
    let width = performance.len() + 1;
    let mut cost = vec![f64::INFINITY; (visual.len() + 1) * width];
    let mut steps = vec![0u8; cost.len()];
    let at = |row: usize, column: usize| row * width + column;
    cost[0] = 0.;
    for row in 0..=visual.len() {
        for column in 0..=performance.len() {
            let current = cost[at(row, column)];
            if !current.is_finite() {
                continue;
            }
            if row < visual.len() && column < performance.len() {
                let onset = &visual[row];
                let performed = &performance[column];
                let time = (onset.expected - performed.start_seconds).abs().min(5.0) * 0.02;
                let pitch = if onset.pitches.is_empty()
                    || performed
                        .pitches
                        .iter()
                        .any(|pitch| onset.pitches.contains(pitch))
                {
                    0.
                } else {
                    0.03
                };
                let next = current + time + pitch;
                let index = at(row + 1, column + 1);
                if next < cost[index] {
                    cost[index] = next;
                    steps[index] = 1;
                }
            }
            if row < visual.len() {
                let index = at(row + 1, column);
                if current + 1.0 < cost[index] {
                    cost[index] = current + 1.0;
                    steps[index] = 2;
                }
            }
            if column < performance.len() {
                let index = at(row, column + 1);
                if current + 1.0 < cost[index] {
                    cost[index] = current + 1.0;
                    steps[index] = 3;
                }
            }
        }
    }
    let mut mapping = vec![None; visual.len()];
    let (mut row, mut column) = (visual.len(), performance.len());
    while row > 0 || column > 0 {
        match steps[at(row, column)] {
            1 => {
                mapping[row - 1] = Some(column - 1);
                row -= 1;
                column -= 1;
            }
            2 => row -= 1,
            3 => column -= 1,
            _ => break,
        }
    }
    mapping
}

fn push_group(
    groups: &mut Vec<PlaybackNoteGroup>,
    element_ids: Vec<String>,
    pitches: Vec<u8>,
    staff_index: Option<usize>,
    voice: Option<String>,
    start_seconds: f64,
    end_seconds: f64,
) {
    if let Some(existing) = groups.iter_mut().find(|group| {
        group.pitches == pitches
            && group.staff_index == staff_index
            && group.voice == voice
            && group
                .start_seconds
                .is_some_and(|value| (value - start_seconds).abs() <= FALLBACK_ONSET_SECONDS)
    }) {
        existing.element_ids.extend(element_ids);
        existing.element_ids.sort();
        existing.element_ids.dedup();
    } else {
        groups.push(PlaybackNoteGroup {
            element_ids,
            pitches,
            staff_index,
            voice,
            start_seconds: Some(start_seconds),
            duration_seconds: Some(end_seconds - start_seconds),
            part_id: None,
        });
    }
}

pub(super) fn fill_rest_groups(
    events: &[SourcePlaybackEvent],
    event_groups: &[Option<usize>],
    groups: &mut [PlaybackNoteGroup],
    midi: &crate::MidiFile,
    ppqn: u64,
    pickup: Option<u64>,
) {
    let last_tick = midi
        .tracks
        .iter()
        .flat_map(|track| &track.notes)
        .map(|note| note.time + note.duration.unwrap_or(0))
        .max()
        .unwrap_or(0);
    let Some(measure_times) = measure_times(midi, ppqn, last_tick, pickup) else {
        return;
    };
    let by_measure: HashMap<_, _> = measure_times
        .iter()
        .map(|time| (time.measure, *time))
        .collect();
    let mut voices = HashMap::<(usize, String), Vec<(usize, usize)>>::new();
    for (event_index, event) in events.iter().enumerate() {
        let Some(group_index) = event_groups[event_index] else {
            continue;
        };
        voices
            .entry((event.staff_index, event.voice.clone()))
            .or_default()
            .push((event_index, group_index));
    }
    for sequence in voices.values() {
        let mut cursor = 0;
        while cursor < sequence.len() {
            if events[sequence[cursor].0].kind != PitchEventKind::Rest {
                cursor += 1;
                continue;
            }
            let start = cursor;
            while cursor < sequence.len() && events[sequence[cursor].0].kind == PitchEventKind::Rest
            {
                cursor += 1;
            }
            let end = cursor;
            let previous = sequence[..start].iter().rev().find_map(|(_, group_index)| {
                let group = &groups[*group_index];
                group
                    .start_seconds
                    .map(|value| value + group.duration_seconds.unwrap_or(0.0))
            });
            let next = sequence[end..]
                .iter()
                .find_map(|(_, group_index)| groups[*group_index].start_seconds);
            let first_event = &events[sequence[start].0];
            let last_event = &events[sequence[end - 1].0];
            let measure_start = first_event
                .measure_start
                .and_then(|measure| by_measure.get(&measure))
                .map_or(0.0, |time| time.start_seconds);
            let measure_end = last_event
                .measure_end
                .and_then(|measure| by_measure.get(&measure))
                .map_or(measure_start, |time| time.end_seconds);
            let gap_start = previous.unwrap_or(measure_start).max(measure_start);
            let gap_end = next.unwrap_or(measure_end).max(gap_start + 0.01);
            let step = (gap_end - gap_start) / (end - start) as f64;
            for (offset, (_, group_index)) in sequence[start..end].iter().enumerate() {
                groups[*group_index].start_seconds = Some(gap_start + step * offset as f64);
                groups[*group_index].duration_seconds = Some(step.max(0.01));
            }
        }
    }
}
