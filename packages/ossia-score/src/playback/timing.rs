use super::{unsupported, MeasureTime, MidiCluster};
use crate::{measure_ticks, MidiDivision, MidiFile};
use ossia_types::AppError;
use std::collections::BTreeMap;

pub(super) fn pickup_ticks(source: &str, ppqn: u64) -> Option<u64> {
    let tokens = crate::tokenize(source);
    let mut pickup = None;
    for (i, _) in tokens
        .iter()
        .enumerate()
        .filter(|(_, token)| token.command() == Some("partial"))
    {
        let raw = &tokens[i + 1..].iter().find(|token| !token.trivia())?.raw;
        let digits = raw.trim_end_matches('.');
        let denominator: u64 = digits.parse().ok()?;
        let dots = u32::try_from(raw.len().checked_sub(digits.len())?).ok()?;
        let factor = 1u64.checked_shl(dots)?;
        let numerator = ppqn
            .checked_mul(4)?
            .checked_mul(factor.checked_mul(2)?.checked_sub(1)?)?;
        let denominator = denominator.checked_mul(factor)?;
        if denominator == 0 || numerator % denominator != 0 {
            return None;
        }
        let ticks = numerator / denominator;
        if pickup.is_some_and(|previous| previous != ticks) {
            return None;
        }
        pickup = Some(ticks);
    }
    pickup
}

pub(super) fn ppqn(midi: &MidiFile) -> Result<u64, AppError> {
    match midi.division {
        MidiDivision::Ppqn { ticks_per_quarter } if midi.format != 2 && ticks_per_quarter > 0 => {
            Ok(u64::from(ticks_per_quarter))
        }
        _ => Err(unsupported("playback requires a shared PPQN MIDI timeline")),
    }
}

pub(super) fn seconds_at(tick: u64, ppqn: u64, midi: &MidiFile) -> f64 {
    let mut cursor = 0u64;
    let mut seconds = 0.0;
    let mut tempo = 500_000u32;
    for change in &midi.tempos {
        if change.tick > tick {
            break;
        }
        seconds += ((change.tick - cursor) as f64 * f64::from(tempo)) / (ppqn as f64 * 1_000_000.0);
        cursor = change.tick;
        tempo = change.microseconds_per_quarter;
    }
    seconds += ((tick - cursor) as f64 * f64::from(tempo)) / (ppqn as f64 * 1_000_000.0);
    seconds
}

pub(super) fn measure_times(
    midi: &MidiFile,
    ppqn: u64,
    last_tick: u64,
    pickup: Option<u64>,
) -> Option<Vec<MeasureTime>> {
    let signatures = &midi.time_signatures;
    if signatures
        .first()
        .is_none_or(|signature| signature.tick != 0)
    {
        return None;
    }
    let mut result = Vec::new();
    let mut signature_index = 0usize;
    let mut tick = 0u64;
    let mut measure = 1u32;
    while tick < last_tick {
        if let Some(next) = signatures.get(signature_index + 1) {
            if next.tick < tick {
                return None;
            }
            if next.tick == tick {
                signature_index += 1;
            }
        }
        let signature = signatures.get(signature_index)?;
        let length = if measure == 1 {
            pickup.unwrap_or(measure_ticks(ppqn, signature)?)
        } else {
            measure_ticks(ppqn, signature)?
        };
        let end = tick.checked_add(length)?;
        if signatures
            .get(signature_index + 1)
            .is_some_and(|next| end > next.tick)
        {
            return None;
        }
        result.push(MeasureTime {
            measure,
            start_seconds: seconds_at(tick, ppqn, midi),
            end_seconds: seconds_at(end, ppqn, midi),
        });
        measure = measure.checked_add(1)?;
        tick = end;
    }
    Some(result)
}

pub(super) fn midi_clusters(
    midi: &MidiFile,
    ppqn: u64,
    pickup: Option<u64>,
) -> Result<Vec<MidiCluster>, AppError> {
    let mut grouped = BTreeMap::<(u64, u64, usize, u8), Vec<u8>>::new();
    for track in &midi.tracks {
        for note in &track.notes {
            let duration = note
                .duration
                .ok_or_else(|| unsupported("playback requires note-off timing"))?;
            grouped
                .entry((note.time, duration, track.track, note.channel))
                .or_default()
                .push(note.note);
        }
    }
    let last_tick = midi
        .tracks
        .iter()
        .flat_map(|track| &track.notes)
        .map(|note| note.time.saturating_add(note.duration.unwrap_or(0)))
        .max()
        .unwrap_or(0);
    let measures = measure_times(midi, ppqn, last_tick, pickup);
    Ok(grouped
        .into_iter()
        .map(
            |((start_tick, duration_tick, track, channel), mut pitches)| {
                pitches.sort_unstable();
                pitches.dedup();
                MidiCluster {
                    start_tick,
                    duration_tick,
                    pitches,
                    measure: measures.as_ref().and_then(|times| {
                        let start = seconds_at(start_tick, ppqn, midi);
                        times
                            .iter()
                            .find(|time| start >= time.start_seconds && start < time.end_seconds)
                            .map(|time| time.measure)
                    }),
                    track,
                    channel,
                }
            },
        )
        .collect())
}
