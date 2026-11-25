use super::fallback::ElementHints;
use super::geometry::measure_position;
use super::source::{event_element_kind, overlaps, source_events};
use super::timing::{measure_times, midi_clusters, seconds_at};
use super::SourcePlaybackEvent;
use crate::{selection_geometry, ScoreIndex, ScoreLayout};
use ossia_types::{AppError, GeometryPage, PlaybackNoteGroup, PlaybackSystemSpan, SelectionTarget};
use std::collections::{BTreeMap, HashMap, HashSet};

#[derive(Debug)]
struct PlayableElement {
    element_id: String,
    pitch: Option<u8>,
    measure: Option<u32>,
    within: Option<f64>,
    staff_index: usize,
}

struct PlayableEvents {
    pages: Vec<GeometryPage>,
    elements: BTreeMap<usize, Vec<PlayableElement>>,
}

pub(super) struct MutedNote {
    pub(super) pitch: u8,
    pub(super) tick: u64,
    pub(super) track: usize,
    pub(super) channel: u8,
}

pub(super) struct NoteGroupBuild {
    pub(super) groups: Vec<PlaybackNoteGroup>,
    pub(super) muted_notes: Vec<MutedNote>,
    pub(super) element_positions: HashMap<String, ElementPosition>,
}

pub(super) struct ElementPosition {
    pub(super) page: u32,
    pub(super) system_index: usize,
    pub(super) x: f64,
}

fn playable_events(
    index: &ScoreIndex,
    layout: &ScoreLayout,
    events: &[SourcePlaybackEvent],
) -> Result<PlayableEvents, AppError> {
    let pages = selection_geometry(index, layout)?.pages;
    let tie_ids = pages
        .iter()
        .flat_map(|page| &page.elements)
        .filter(|element| element.region.element_kind.as_deref() == Some("Tie"))
        .filter_map(|element| match &element.region.target {
            SelectionTarget::Element { element_id } => Some(element_id.clone()),
            SelectionTarget::Measure { .. } => None,
        })
        .collect::<std::collections::HashSet<_>>();
    let mut parent = (0..events.len()).collect::<Vec<_>>();
    fn root(parent: &mut [usize], mut index: usize) -> usize {
        while parent[index] != index {
            parent[index] = parent[parent[index]];
            index = parent[index];
        }
        index
    }
    fn unite(parent: &mut [usize], left: usize, right: usize) {
        let left = root(parent, left);
        let right = root(parent, right);
        if left != right {
            let (root, child) = if left < right {
                (left, right)
            } else {
                (right, left)
            };
            parent[child] = root;
        }
    }
    let mut tied_events = HashMap::<Vec<String>, usize>::new();
    let mut result: BTreeMap<usize, Vec<PlayableElement>> = BTreeMap::new();
    for page in &pages {
        for element in &page.elements {
            let Some(element_kind) = element.region.element_kind.as_deref() else {
                continue;
            };
            if !matches!(
                element_kind,
                "NoteHead" | "TabNoteHead" | "Rest" | "MultiMeasureRest"
            ) {
                continue;
            }
            let Some(span) = element.region.spans.first().copied() else {
                continue;
            };
            let direct = events
                .iter()
                .enumerate()
                .filter(|(_, event)| {
                    event_element_kind(event.kind, element_kind) && overlaps(event.span, span)
                })
                .min_by_key(|(_, event)| {
                    (
                        usize::from(
                            element
                                .region
                                .staff_index
                                .is_some_and(|staff| staff != event.staff_index),
                        ),
                        event.span.byte_end - event.span.byte_start,
                    )
                });
            let next = (direct.is_none()
                && index
                    .source()
                    .get(span.byte_start..span.byte_end)
                    .is_some_and(|raw| raw.trim().starts_with('\\')))
            .then(|| {
                let measure = index
                    .measure_at(span.byte_start)
                    .map(|measure| measure.measure_start);
                let voice = element.region.voice.as_deref();
                events
                    .iter()
                    .enumerate()
                    .filter(|(_, event)| {
                        event.span.byte_start >= span.byte_end
                            && event.measure_start == measure
                            && event_element_kind(event.kind, element_kind)
                            && voice.is_none_or(|voice| {
                                event.voice == voice
                                    || event
                                        .voice
                                        .strip_prefix(voice)
                                        .is_some_and(|tail| tail.starts_with('['))
                            })
                    })
                    .min_by_key(|(_, event)| event.span.byte_start - span.byte_end)
            })
            .flatten();
            let Some((index, _)) = direct.or(next) else {
                continue;
            };
            if matches!(element_kind, "NoteHead" | "TabNoteHead") {
                let mut ties = element
                    .linked_targets
                    .iter()
                    .filter_map(|target| match target {
                        SelectionTarget::Element { element_id } if tie_ids.contains(element_id) => {
                            Some(element_id.clone())
                        }
                        _ => None,
                    })
                    .collect::<Vec<_>>();
                if !ties.is_empty() {
                    ties.sort();
                    if let Some(owner) = tied_events.get(&ties).copied() {
                        unite(&mut parent, index, owner);
                    } else {
                        tied_events.insert(ties, index);
                    }
                }
            }
            let SelectionTarget::Element { element_id } = &element.region.target else {
                continue;
            };
            let pitch = events[index]
                .members
                .iter()
                .min_by_key(|member| {
                    let distance = if span.byte_end <= member.span.byte_start {
                        member.span.byte_start - span.byte_end
                    } else {
                        span.byte_start.saturating_sub(member.span.byte_end)
                    };
                    let overlap = span
                        .byte_end
                        .min(member.span.byte_end)
                        .saturating_sub(span.byte_start.max(member.span.byte_start));
                    (distance, std::cmp::Reverse(overlap))
                })
                .map(|member| member.pitch);
            let position = measure_position(layout, page, element);
            result.entry(index).or_default().push(PlayableElement {
                element_id: element_id.clone(),
                pitch,
                measure: position.map(|position| position.measure),
                within: position.map(|position| position.within),
                staff_index: events[index].staff_index,
            });
        }
    }
    for elements in result.values_mut() {
        elements.sort_by(|a, b| a.element_id.cmp(&b.element_id));
        elements.dedup_by(|a, b| a.element_id == b.element_id);
    }
    let mut merged = BTreeMap::<usize, Vec<PlayableElement>>::new();
    for (index, elements) in result {
        merged
            .entry(root(&mut parent, index))
            .or_default()
            .extend(elements);
    }
    for elements in merged.values_mut() {
        elements.sort_by(|a, b| a.element_id.cmp(&b.element_id));
        elements.dedup_by(|a, b| a.element_id == b.element_id);
    }
    Ok(PlayableEvents {
        pages,
        elements: merged,
    })
}

pub(super) fn tied_note_chains(pages: &[GeometryPage]) -> Vec<Vec<String>> {
    let tie_ids = pages
        .iter()
        .flat_map(|page| &page.elements)
        .filter(|element| element.region.element_kind.as_deref() == Some("Tie"))
        .filter_map(|element| match &element.region.target {
            SelectionTarget::Element { element_id } => Some(element_id.as_str()),
            SelectionTarget::Measure { .. } => None,
        })
        .collect::<HashSet<_>>();
    let mut note_links = HashMap::<String, HashSet<String>>::new();
    let mut note_ties = HashMap::<String, HashSet<String>>::new();
    for element in pages.iter().flat_map(|page| &page.elements) {
        if !matches!(
            element.region.element_kind.as_deref(),
            Some("NoteHead" | "TabNoteHead")
        ) {
            continue;
        }
        let SelectionTarget::Element { element_id } = &element.region.target else {
            continue;
        };
        for target in &element.linked_targets {
            let SelectionTarget::Element {
                element_id: linked_id,
            } = target
            else {
                continue;
            };
            if tie_ids.contains(linked_id.as_str()) {
                note_ties
                    .entry(element_id.clone())
                    .or_default()
                    .insert(linked_id.clone());
            } else if linked_id != element_id {
                note_links
                    .entry(element_id.clone())
                    .or_default()
                    .insert(linked_id.clone());
            }
        }
    }
    let mut edges = HashMap::<String, HashSet<String>>::new();
    for (note, linked) in &note_links {
        for other in linked {
            let reciprocal = note_links
                .get(other)
                .is_some_and(|links| links.contains(note));
            let shared_tie = note_ties
                .get(note)
                .zip(note_ties.get(other))
                .is_some_and(|(left, right)| !left.is_disjoint(right));
            if reciprocal && shared_tie {
                edges.entry(note.clone()).or_default().insert(other.clone());
                edges.entry(other.clone()).or_default().insert(note.clone());
            }
        }
    }
    let mut seen = HashSet::new();
    let mut chains = Vec::new();
    for start in edges.keys() {
        if seen.contains(start) {
            continue;
        }
        let mut chain = Vec::new();
        let mut pending = vec![start.clone()];
        while let Some(id) = pending.pop() {
            if !seen.insert(id.clone()) {
                continue;
            }
            chain.push(id.clone());
            pending.extend(edges.get(&id).into_iter().flatten().cloned());
        }
        if chain.len() > 1 {
            chain.sort();
            chains.push(chain);
        }
    }
    chains
}

/// Recompose tie chains after fallback mapping, without absorbing a following rearticulation.
fn merge_tied_groups(groups: &mut Vec<PlaybackNoteGroup>, pages: &[GeometryPage]) {
    let chains = tied_note_chains(pages);
    if chains.is_empty() {
        return;
    }
    let mut by_element = HashMap::<String, Vec<usize>>::new();
    for (index, group) in groups.iter().enumerate() {
        for id in &group.element_ids {
            by_element.entry(id.clone()).or_default().push(index);
        }
    }
    let mut removed = HashSet::new();
    for chain in chains {
        let members = chain.iter().cloned().collect::<HashSet<_>>();
        let mut involved = chain
            .iter()
            .filter_map(|id| by_element.get(id))
            .flatten()
            .copied()
            .collect::<Vec<_>>();
        involved.sort_unstable();
        involved.dedup();
        let Some(target) = involved
            .iter()
            .copied()
            .filter(|index| !removed.contains(index))
            .min_by(|left, right| {
                groups[*left]
                    .start_seconds
                    .unwrap_or(f64::INFINITY)
                    .total_cmp(&groups[*right].start_seconds.unwrap_or(f64::INFINITY))
                    .then_with(|| left.cmp(right))
            })
        else {
            continue;
        };
        if groups[target].start_seconds.is_none() {
            continue;
        }
        for index in involved {
            if index == target || removed.contains(&index) {
                continue;
            }
            groups[index].element_ids.retain(|id| !members.contains(id));
            if groups[index].element_ids.is_empty() {
                removed.insert(index);
            }
        }
        groups[target].element_ids.extend(chain.iter().cloned());
        groups[target].element_ids.sort();
        groups[target].element_ids.dedup();
    }
    if !removed.is_empty() {
        *groups = std::mem::take(groups)
            .into_iter()
            .enumerate()
            .filter_map(|(index, group)| (!removed.contains(&index)).then_some(group))
            .collect();
    }
}

pub(super) fn note_groups(
    index: &ScoreIndex,
    layout: &ScoreLayout,
    midi: &crate::MidiFile,
    ppqn: u64,
    systems: &[PlaybackSystemSpan],
) -> Result<NoteGroupBuild, AppError> {
    let events = source_events(index);
    let PlayableEvents { pages, elements } = playable_events(index, layout, &events)?;
    let pickup = super::timing::pickup_ticks(index.source(), ppqn);
    let clusters = midi_clusters(midi, ppqn, pickup)?;
    let measured_layout = !layout.columns.is_empty()
        && layout
            .columns
            .iter()
            .all(|column| column.measure_start.is_some() && column.measure_end.is_some());
    let last_tick = midi
        .tracks
        .iter()
        .flat_map(|track| &track.notes)
        .map(|note| note.time + note.duration.unwrap_or(0))
        .max()
        .unwrap_or(0);
    let measure_onsets = measure_times(midi, ppqn, last_tick, pickup)
        .unwrap_or_default()
        .into_iter()
        .map(|time| (time.measure, (time.start_seconds, time.end_seconds)))
        .collect::<HashMap<_, _>>();
    let mut used = vec![HashSet::<u8>::new(); clusters.len()];
    let mut last_tick = HashMap::<String, u64>::new();
    let mut groups = Vec::new();
    let mut muted_notes = Vec::new();
    let mut event_groups = vec![None; events.len()];
    let mut matched_sources = HashMap::<(usize, usize, String), Vec<(usize, usize, usize)>>::new();
    let mut reused_events = Vec::<(usize, usize)>::new();
    let programs = crate::instruments::ProgramTimeline::new(midi);
    let mut ambiguous_performers = std::collections::HashSet::new();
    for (event_index, event) in events.iter().enumerate() {
        let Some(element_notes) = elements.get(&event_index) else {
            continue;
        };
        if event.kind == crate::PitchEventKind::Rest {
            event_groups[event_index] = Some(groups.len());
            groups.push(PlaybackNoteGroup {
                element_ids: element_notes
                    .iter()
                    .map(|element| element.element_id.clone())
                    .collect(),
                pitches: Vec::new(),
                staff_index: Some(event.staff_index),
                voice: Some(event.voice.clone()),
                start_seconds: None,
                duration_seconds: None,
                part_id: None,
            });
            continue;
        }
        if !matches!(
            event.kind,
            crate::PitchEventKind::Note | crate::PitchEventKind::Chord
        ) {
            continue;
        }
        let rendered_measure = element_notes
            .iter()
            .find_map(|element| element.measure)
            .filter(|measure| {
                element_notes
                    .iter()
                    .all(|element| element.measure.is_none() || element.measure == Some(*measure))
            });
        let measure = rendered_measure
            .or_else(|| {
                event
                    .measure_start
                    .and_then(|measure| index.rendered_measure(&event.voice, measure, None))
            })
            .or(event.measure_start);
        // Tie chains contain several noteheads, but only their earliest onset has
        // a MIDI note-on. Element ids are stable identities, not chronological
        // order, so using the first one would bind the chain to a later repeat.
        let expected_seconds = element_notes
            .iter()
            .filter_map(|element| {
                let measure = element.measure?;
                let within = element.within?;
                let (start, end) = measure_onsets.get(&measure)?;
                Some(start + within * (end - start))
            })
            .min_by(f64::total_cmp);
        let mut pitches = event.pitches.clone();
        pitches.sort_unstable();
        pitches.dedup();
        let staff = index.staves.get(event.staff_index);
        let expected_instrument = staff
            .and_then(|staff| {
                staff
                    .midi_instrument_name
                    .as_deref()
                    .or(staff.instrument_name.as_deref())
            })
            .and_then(crate::named_sample_instrument)
            .map(|instrument| {
                if instrument == ossia_types::SampleInstrument::Ukulele {
                    ossia_types::SampleInstrument::Guitar
                } else {
                    instrument
                }
            });
        let affinity = |cluster: &super::MidiCluster| {
            if staff
                .and_then(|staff| staff.label.as_deref())
                .is_some_and(|label| {
                    midi.tracks
                        .get(cluster.track)
                        .and_then(|track| track.name.as_deref())
                        .is_some_and(|name| name.split(':').next() == Some(label))
                })
            {
                return 0;
            }
            match (
                expected_instrument,
                programs
                    .at(cluster.channel, cluster.start_tick)
                    .0
                    .and_then(crate::program_sample_instrument),
            ) {
                (Some(expected), Some(actual)) if expected == actual => 0,
                (Some(_), Some(_)) => 2,
                _ => 1,
            }
        };
        let candidate = clusters
            .iter()
            .enumerate()
            .filter(|(index, cluster)| {
                pitches
                    .iter()
                    .all(|pitch| cluster.pitches.contains(pitch) && !used[*index].contains(pitch))
                    && match measure {
                        Some(measure) => {
                            cluster.measure.is_none() || cluster.measure == Some(measure)
                        }
                        None => cluster.measure.is_none() || !measured_layout,
                    }
            })
            .filter(|(_, cluster)| {
                last_tick
                    .get(&event.voice)
                    .is_none_or(|tick| cluster.start_tick >= *tick)
            })
            .min_by(|(_, left), (_, right)| {
                let left_onset = seconds_at(left.start_tick, ppqn, midi);
                let right_onset = seconds_at(right.start_tick, ppqn, midi);
                // Source order is authoritative within an indexed voice. Geometry
                // estimates cannot skip a repeated pitch's earlier performance.
                event
                    .measure_start
                    .map_or(std::cmp::Ordering::Equal, |_| {
                        left.start_tick.cmp(&right.start_tick)
                    })
                    .then_with(|| {
                        expected_seconds
                            .map_or(0., |expected| (left_onset - expected).abs())
                            .total_cmp(
                                &expected_seconds
                                    .map_or(0., |expected| (right_onset - expected).abs()),
                            )
                    })
                    .then_with(|| {
                        (left.pitches.len() - pitches.len())
                            .cmp(&(right.pitches.len() - pitches.len()))
                    })
                    .then_with(|| left.start_tick.cmp(&right.start_tick))
                    .then_with(|| affinity(left).cmp(&affinity(right)))
                    .then_with(|| left.track.cmp(&right.track))
                    .then_with(|| left.channel.cmp(&right.channel))
                    .then_with(|| left.duration_tick.cmp(&right.duration_tick))
            });
        let source_key = (
            event.span.byte_start,
            event.span.byte_end,
            event.voice.clone(),
        );
        let previous = matched_sources.get(&source_key).and_then(|previous| {
            (previous.len() == 1)
                .then_some(previous[0])
                .filter(|(_, _, staff)| {
                    index
                        .staves
                        .get(event.staff_index)
                        .is_some_and(|staff| staff.kind == crate::StaffKind::Tablature)
                        || index
                            .staves
                            .get(*staff)
                            .is_some_and(|staff| staff.kind == crate::StaffKind::Tablature)
                })
        });
        let explicit = |staff: usize| {
            index.staves.get(staff).is_some_and(|staff| {
                staff.midi_instrument_name.is_some() || staff.instrument_name.is_some()
            })
        };
        let duplicate = previous.and_then(|(first, _, previous_staff)| {
            candidate.filter(|(_, other)| {
                let original = &clusters[first];
                let original_program = programs.at(original.channel, original.start_tick).0;
                original.start_tick == other.start_tick
                    && original.duration_tick == other.duration_tick
                    && ((original_program.is_some()
                        && original_program == programs.at(other.channel, other.start_tick).0)
                        || !(explicit(previous_staff) && explicit(event.staff_index)))
            })
        });
        let prefer_duplicate = previous.is_some_and(|(_, _, previous_staff)| {
            duplicate.is_some()
                && !explicit(previous_staff)
                && (explicit(event.staff_index)
                    || index
                        .staves
                        .get(event.staff_index)
                        .is_some_and(|staff| staff.kind == crate::StaffKind::Tablature))
        });
        if let (Some((first, previous_event, _)), Some((copy, cluster))) = (previous, duplicate) {
            used[copy].extend(pitches.iter().copied());
            let original = &clusters[first];
            ambiguous_performers.remove(&(original.track, original.channel));
            ambiguous_performers.remove(&(cluster.track, cluster.channel));
            let silent = if prefer_duplicate { original } else { cluster };
            muted_notes.extend(pitches.iter().map(|pitch| MutedNote {
                pitch: *pitch,
                tick: silent.start_tick,
                track: silent.track,
                channel: silent.channel,
            }));
            if prefer_duplicate {
                let ids = elements
                    .get(&previous_event)
                    .into_iter()
                    .flatten()
                    .map(|element| &element.element_id)
                    .collect::<std::collections::HashSet<_>>();
                for group in &mut groups {
                    if group.element_ids.iter().any(|id| ids.contains(id)) {
                        group.part_id = Some(format!("{}:{}", cluster.track, cluster.channel));
                    }
                }
            }
        }
        let reused = previous.filter(|(cluster, _, _)| {
            duplicate.is_some()
                || candidate
                    .is_none_or(|(_, other)| other.start_tick != clusters[*cluster].start_tick)
        });
        if let Some((_, previous_event, _)) = reused {
            reused_events.push((previous_event, event_index));
        }
        let candidate = if prefer_duplicate {
            candidate
        } else {
            reused
                .map(|(cluster, _, _)| (cluster, &clusters[cluster]))
                .or(candidate)
        };
        if let Some((selected, cluster)) = candidate {
            let competitors = clusters
                .iter()
                .enumerate()
                .filter(|(position, other)| {
                    *position != selected
                        && pitches.iter().all(|pitch| !used[*position].contains(pitch))
                        && other.start_tick == cluster.start_tick
                        && other.pitches == cluster.pitches
                        && affinity(other) == affinity(cluster)
                })
                .map(|(_, other)| (other.track, other.channel))
                .collect::<Vec<_>>();
            if !competitors.is_empty() {
                ambiguous_performers.insert((cluster.track, cluster.channel));
                ambiguous_performers.extend(competitors);
            }
        }
        let muted_key = candidate
            .as_ref()
            .map(|(_, cluster)| (cluster.start_tick, cluster.track, cluster.channel));
        let (start_seconds, duration_seconds) = if let Some((index, cluster)) = candidate {
            used[index].extend(pitches.iter().copied());
            if reused.is_none() {
                matched_sources.entry(source_key).or_default().push((
                    index,
                    event_index,
                    event.staff_index,
                ));
            }
            last_tick.insert(event.voice.clone(), cluster.start_tick);
            (
                Some(seconds_at(cluster.start_tick, ppqn, midi)),
                Some(
                    seconds_at(cluster.start_tick + cluster.duration_tick, ppqn, midi)
                        - seconds_at(cluster.start_tick, ppqn, midi),
                ),
            )
        } else {
            (None, None)
        };
        // One click auditions one sounding pitch; tie chains with the same pitch stay together.
        let mut by_pitch = BTreeMap::<u8, Vec<String>>::new();
        for element in element_notes {
            let Some(pitch) = element.pitch else {
                by_pitch.clear();
                break;
            };
            by_pitch
                .entry(pitch)
                .or_default()
                .push(element.element_id.clone());
        }
        if by_pitch.is_empty() {
            event_groups[event_index] = Some(groups.len());
            if event.muted {
                if let Some((tick, track, channel)) = muted_key {
                    muted_notes.extend(pitches.iter().map(|pitch| MutedNote {
                        pitch: *pitch,
                        tick,
                        track,
                        channel,
                    }));
                }
            }
            groups.push(PlaybackNoteGroup {
                element_ids: element_notes
                    .iter()
                    .map(|element| element.element_id.clone())
                    .collect(),
                pitches: if event.muted { Vec::new() } else { pitches },
                staff_index: Some(event.staff_index),
                voice: Some(event.voice.clone()),
                start_seconds,
                duration_seconds,
                part_id: muted_key.map(|(_, track, channel)| format!("{track}:{channel}")),
            });
            continue;
        }
        for (pitch, element_ids) in by_pitch {
            if event.muted {
                if let Some((tick, track, channel)) = muted_key {
                    muted_notes.push(MutedNote {
                        pitch,
                        tick,
                        track,
                        channel,
                    });
                }
            }
            if event_groups[event_index].is_none() {
                event_groups[event_index] = Some(groups.len());
            }
            groups.push(PlaybackNoteGroup {
                element_ids,
                pitches: if event.muted { Vec::new() } else { vec![pitch] },
                staff_index: Some(event.staff_index),
                voice: Some(event.voice.clone()),
                start_seconds,
                duration_seconds,
                part_id: muted_key.map(|(_, track, channel)| format!("{track}:{channel}")),
            });
        }
    }
    super::fallback::fill_rest_groups(&events, &event_groups, &mut groups, midi, ppqn, pickup);
    let mut groups_by_element = HashMap::<String, Vec<usize>>::new();
    for (position, group) in groups.iter().enumerate() {
        for id in &group.element_ids {
            groups_by_element
                .entry(id.clone())
                .or_default()
                .push(position);
        }
    }
    let mut removed = std::collections::HashSet::new();
    for (first, duplicate) in reused_events {
        let originals = elements
            .get(&first)
            .into_iter()
            .flatten()
            .flat_map(|element| {
                groups_by_element
                    .get(&element.element_id)
                    .into_iter()
                    .flatten()
                    .copied()
            })
            .collect::<std::collections::BTreeSet<_>>();
        let copies = elements
            .get(&duplicate)
            .into_iter()
            .flatten()
            .flat_map(|element| {
                groups_by_element
                    .get(&element.element_id)
                    .into_iter()
                    .flatten()
                    .copied()
            })
            .collect::<std::collections::BTreeSet<_>>();
        for copy in copies {
            if let Some(original) = originals.iter().copied().find(|original| {
                *original != copy
                    && groups[*original].pitches == groups[copy].pitches
                    && groups[*original].part_id == groups[copy].part_id
            }) {
                let ids = groups[copy].element_ids.clone();
                groups[original].element_ids.extend(ids);
                groups[original].element_ids.sort();
                groups[original].element_ids.dedup();
                removed.insert(copy);
            }
        }
    }
    let mut position = 0;
    groups.retain(|_| {
        let keep = !removed.contains(&position);
        position += 1;
        keep
    });
    let ambiguous_ids = ambiguous_performers
        .into_iter()
        .map(|(track, channel)| format!("{track}:{channel}"))
        .collect::<std::collections::HashSet<_>>();
    for group in &mut groups {
        if group
            .part_id
            .as_ref()
            .is_some_and(|temporary| ambiguous_ids.contains(temporary))
        {
            group.part_id = None;
        }
    }
    let covered = groups
        .iter()
        .filter(|group| group.start_seconds.is_some())
        .flat_map(|group| group.element_ids.iter().cloned())
        .collect::<std::collections::HashSet<_>>();
    let element_pitches = elements
        .values()
        .flatten()
        .filter_map(|element| {
            element
                .pitch
                .map(|pitch| (element.element_id.clone(), pitch))
        })
        .collect::<HashMap<_, _>>();
    let element_staves = elements
        .values()
        .flatten()
        .map(|element| (element.element_id.clone(), element.staff_index))
        .collect::<HashMap<_, _>>();
    let hints = ElementHints {
        pitches: &element_pitches,
        staves: &element_staves,
    };
    if measured_layout {
        super::fallback::fallback_note_groups(
            layout,
            &pages,
            midi,
            (ppqn, pickup),
            &mut groups,
            &covered,
            &hints,
        );
    } else {
        let mut visual_groups = Vec::new();
        super::fallback::visual_note_groups(
            &pages,
            midi,
            ppqn,
            &mut visual_groups,
            &hints,
            systems,
        );
        if visual_groups
            .iter()
            .any(|group| !group.element_ids.is_empty())
        {
            groups = visual_groups;
        }
    }
    super::engraved::note_groups(
        index,
        layout,
        &pages,
        midi,
        ppqn,
        &element_pitches,
        &mut groups,
    );
    merge_tied_groups(&mut groups, &pages);
    groups.retain(|group| group.start_seconds.is_some());
    groups.sort_by(|a, b| {
        a.start_seconds
            .unwrap_or(f64::INFINITY)
            .total_cmp(&b.start_seconds.unwrap_or(f64::INFINITY))
            .then_with(|| a.element_ids.cmp(&b.element_ids))
    });
    // The visual pass can leave MIDI-only placeholders that engraving has since resolved.
    for onset in groups
        .chunk_by_mut(|a, b| a.start_seconds == b.start_seconds)
        .filter(|onset| onset[0].element_ids.is_empty())
    {
        let covered = onset
            .iter()
            .filter(|group| !group.element_ids.is_empty())
            .flat_map(|group| group.pitches.iter().copied())
            .collect::<HashSet<_>>();
        for group in onset
            .iter_mut()
            .filter(|group| group.element_ids.is_empty())
        {
            group.pitches.retain(|pitch| !covered.contains(pitch));
        }
    }
    groups.retain(|group| !group.element_ids.is_empty() || !group.pitches.is_empty());
    let element_positions = pages
        .iter()
        .flat_map(|page| {
            let layout_page = layout.pages.iter().find(|item| item.page == page.page);
            page.elements
                .iter()
                .filter(|element| {
                    matches!(
                        element.region.element_kind.as_deref(),
                        Some("NoteHead" | "TabNoteHead")
                    )
                })
                .filter_map(move |element| {
                    let SelectionTarget::Element { element_id } = &element.region.target else {
                        return None;
                    };
                    let y = (element.bounds.y0 + element.bounds.y1) / 2.;
                    let system_index = layout_page?
                        .systems
                        .iter()
                        .enumerate()
                        .min_by(|(_, left), (_, right)| {
                            vertical_distance(y, left.top, left.bot)
                                .total_cmp(&vertical_distance(y, right.top, right.bot))
                        })
                        .map(|(index, _)| index)?;
                    Some((
                        element_id.clone(),
                        ElementPosition {
                            page: page.page,
                            system_index,
                            x: (element.bounds.x0 + element.bounds.x1) / 2.,
                        },
                    ))
                })
        })
        .collect();
    Ok(NoteGroupBuild {
        groups,
        muted_notes,
        element_positions,
    })
}

#[cfg(test)]
#[path = "../../tests/unit/playback_ties.rs"]
mod tests;

fn vertical_distance(value: f64, top: f64, bot: f64) -> f64 {
    if value < top {
        top - value
    } else if value > bot {
        value - bot
    } else {
        0.
    }
}
