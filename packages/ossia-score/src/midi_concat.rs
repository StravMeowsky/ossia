use crate::midi_range::{parse_raw, write_midi, RawEventKind, SliceEvent};
use ossia_types::AppError;

/// Join the MIDI files of consecutive `\score` blocks into one timeline.
///
/// `lengths` holds each score's music length in whole notes as
/// `(numerator, denominator)`, in the same order as `parts`. Part `n` starts at
/// the summed length of the parts before it, so its ticks line up with the
/// engraved moments that carry the same offsets. Tracks are joined by index
/// and every part keeps its own meta events (tempo, time signature, names).
pub fn concat_midi(parts: &[Vec<u8>], lengths: &[(u64, u64)]) -> Result<Vec<u8>, AppError> {
    if parts.is_empty() || parts.len() != lengths.len() {
        return Err(AppError::invalid(
            "MIDI parts and score lengths do not correspond",
        ));
    }
    let mut division = None;
    let mut tracks: Vec<Vec<SliceEvent>> = Vec::new();
    let mut offset = 0u64;
    let mut last_tick = 0u64;
    let mut serial = 0usize;
    for (bytes, &(numerator, denominator)) in parts.iter().zip(lengths) {
        let raw = parse_raw(bytes)?;
        if raw.format != 1 {
            return Err(AppError::unsupported(
                "multi-score MIDI joining requires format 1 parts",
            ));
        }
        if raw.division & 0x8000 != 0 || raw.division == 0 {
            return Err(AppError::unsupported(
                "multi-score MIDI joining requires a PPQN division",
            ));
        }
        if *division.get_or_insert(raw.division) != raw.division {
            return Err(AppError::unsupported(
                "multi-score MIDI parts use different divisions",
            ));
        }
        // A whole note is four quarters; the score length must land on a tick.
        let whole = 4 * u64::from(raw.division);
        if denominator == 0 || (numerator * whole) % denominator != 0 {
            return Err(AppError::unsupported(
                "score length cannot be represented as whole MIDI ticks",
            ));
        }
        let length = numerator * whole / denominator;
        if raw.last_tick > length {
            return Err(AppError::invalid(
                "MIDI part extends beyond its score length",
            ));
        }
        if tracks.len() < raw.tracks.len() {
            tracks.resize_with(raw.tracks.len(), Vec::new);
        }
        for (index, events) in raw.tracks.into_iter().enumerate() {
            for event in events {
                if matches!(event.kind, RawEventKind::Meta(0x2f, _)) {
                    continue;
                }
                let tick = offset
                    .checked_add(event.tick)
                    .ok_or_else(|| AppError::invalid("MIDI time overflow"))?;
                last_tick = last_tick.max(tick);
                tracks[index].push(SliceEvent {
                    tick,
                    priority: 0,
                    serial,
                    kind: event.kind,
                });
                serial += 1;
            }
        }
        offset = offset
            .checked_add(length)
            .ok_or_else(|| AppError::invalid("MIDI time overflow"))?;
    }
    write_midi(
        1,
        division.expect("at least one part"),
        offset.max(last_tick),
        &tracks,
    )
}
