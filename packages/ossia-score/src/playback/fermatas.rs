use super::engraved::{midi_origin, moment_ticks};
use super::timing::seconds_at;
use crate::{MidiFile, ScoreIndex, ScoreLayout, SvgObject};
use ossia_types::{PlaybackNote, PlaybackNoteGroup, PlaybackSystemSpan};
use std::collections::{BTreeMap, HashMap};
use std::iter;

const EXTRA_DURATION_RATIO: f64 = 1.0;
// Start plus duration may round differently from the original MIDI release.
const ROUNDING_EPSILON_SECONDS: f64 = 1e-9;

pub(super) fn is_fermata(object: &SvgObject) -> bool {
    matches!(object.kind.as_str(), "Script" | "MultiMeasureRestScript")
        && object.ornament.as_deref() == Some("fermata")
}

/// Cumulative time added at each original release point.
#[derive(Default)]
pub(super) struct Fermatas {
    offsets: Vec<(f64, f64)>,
    pub(super) last_release_tick: u64,
    pub(super) origin_tick: u64,
}

impl Fermatas {
    pub(super) fn from_layout(
        index: &ScoreIndex,
        layout: &ScoreLayout,
        midi: &MidiFile,
        ppqn: u64,
    ) -> Self {
        let objects = layout
            .pages
            .iter()
            .flat_map(|page| &page.objects)
            .filter(|object| !object.hit_only);
        let marks = objects.clone().filter(|object| is_fermata(object));
        if marks.clone().next().is_none() {
            return Self::default();
        }
        let rhythmic = objects
            .filter_map(|object| {
                if object.note.is_some_and(|note| note.grace_numerator != 0) {
                    return None;
                }
                let id = object.graphic_id.as_deref()?;
                let timing = object.timing?;
                // Rest scripts reference the original spanner even when only its printed piece remains.
                let aliases = if object.kind == "MultiMeasureRest" {
                    object.related.as_slice()
                } else {
                    &[]
                };
                Some(
                    iter::once(id)
                        .chain(aliases.iter().map(String::as_str))
                        .map(move |id| (id, timing)),
                )
            })
            .flatten()
            .collect::<HashMap<_, _>>();
        let origin = midi_origin(index, layout, ppqn);
        let mut holds = BTreeMap::<u64, f64>::new();
        for timing in marks
            .flat_map(|mark| &mark.owners)
            .filter_map(|owner| rhythmic.get(owner.as_str()))
        {
            let (Some(start), Some(length)) = (
                moment_ticks(timing.numerator.into(), timing.denominator, ppqn)
                    .and_then(|tick| tick.checked_add(origin)),
                moment_ticks(
                    timing.duration_numerator.into(),
                    timing.duration_denominator,
                    ppqn,
                ),
            ) else {
                continue;
            };
            let Some(end) = start.checked_add(length).filter(|_| length > 0) else {
                continue;
            };
            let extra = (seconds_at(end, ppqn, midi) - seconds_at(start, ppqn, midi))
                * EXTRA_DURATION_RATIO;
            // Multiple voices marking the same release share a single hold.
            holds
                .entry(end)
                .and_modify(|hold| *hold = hold.max(extra))
                .or_insert(extra);
        }
        let mut offset = 0.;
        Self {
            last_release_tick: holds.last_key_value().map_or(0, |(&tick, _)| tick),
            origin_tick: if holds.is_empty() { 0 } else { origin },
            offsets: holds
                .into_iter()
                .map(|(end, hold)| {
                    offset += hold;
                    (seconds_at(end, ppqn, midi), offset)
                })
                .collect(),
        }
    }

    fn offset_at(&self, seconds: f64) -> f64 {
        let index = self
            .offsets
            .partition_point(|(release, _)| *release <= seconds + ROUNDING_EPSILON_SECONDS);
        self.offsets[..index]
            .last()
            .map_or(0., |&(_, offset)| offset)
    }

    pub(super) fn time(&self, seconds: f64) -> f64 {
        seconds + self.offset_at(seconds)
    }

    pub(super) fn apply(
        &self,
        notes: &mut [PlaybackNote],
        groups: &mut [PlaybackNoteGroup],
        systems: &mut [PlaybackSystemSpan],
    ) {
        if self.offsets.is_empty() {
            return;
        }
        let apply_interval = |start: &mut f64, duration: Option<&mut f64>| {
            let offset = self.offset_at(*start);
            if let Some(duration) = duration {
                *duration += self.offset_at(*start + *duration) - offset;
            }
            *start += offset;
        };
        for note in notes {
            apply_interval(&mut note.start_seconds, Some(&mut note.duration_seconds));
        }
        for group in groups {
            if let Some(start) = group.start_seconds.as_mut() {
                apply_interval(start, group.duration_seconds.as_mut());
            }
            if let Some(playhead) = group.playhead_seconds.as_mut() {
                *playhead = self.time(*playhead);
            }
        }
        for span in systems {
            span.start_seconds = self.time(span.start_seconds);
            span.end_seconds = self.time(span.end_seconds);
        }
    }

    pub(super) fn end(&self) -> Option<f64> {
        self.offsets
            .last()
            .map(|(release, offset)| release + offset)
    }
}
