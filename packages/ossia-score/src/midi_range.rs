use crate::{analysis::measure_ticks, midi::Reader, MidiDivision, MidiFile};
use ossia_types::{AppError, ErrorCode, MeasureRange};
use std::collections::{HashMap, VecDeque};

/// Tick and wall-clock bounds for an inclusive one-based measure range.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MidiMeasureBounds {
    pub start_tick: u64,
    pub end_tick: u64,
    pub start_seconds: f64,
    pub end_seconds: f64,
}

/// Resolve a measure range against the MIDI time-signature map.
pub fn midi_measure_bounds(
    midi: &MidiFile,
    range: MeasureRange,
) -> Result<MidiMeasureBounds, AppError> {
    range.validate()?;
    let ppqn = match midi.division {
        MidiDivision::Ppqn { ticks_per_quarter } if midi.format != 2 && ticks_per_quarter > 0 => {
            u64::from(ticks_per_quarter)
        }
        _ => {
            return Err(AppError::unsupported(
                "measure ranges require a shared PPQN MIDI timeline",
            ))
        }
    };
    if midi
        .time_signatures
        .first()
        .is_none_or(|signature| signature.tick != 0)
    {
        return Err(AppError::unsupported(
            "measure ranges require a time signature at tick 0",
        ));
    }
    let mut tick = 0u64;
    let mut measure = 1u32;
    let mut signature_index = 0usize;
    let mut start_tick = None;
    while measure <= range.end {
        while let Some(next) = midi.time_signatures.get(signature_index + 1) {
            if next.tick < tick {
                return Err(AppError::unsupported(
                    "MIDI time-signature ticks are not monotonic",
                ));
            }
            if next.tick == tick {
                signature_index += 1;
            } else {
                break;
            }
        }
        let signature = &midi.time_signatures[signature_index];
        let length = measure_ticks(ppqn, signature).ok_or_else(|| {
            AppError::unsupported("MIDI time signature cannot be represented as whole ticks")
        })?;
        let end = tick
            .checked_add(length)
            .ok_or_else(|| AppError::unsupported("MIDI measure range overflow"))?;
        if midi
            .time_signatures
            .get(signature_index + 1)
            .is_some_and(|next| next.tick > tick && next.tick < end)
        {
            return Err(AppError::unsupported(
                "MIDI time-signature change is not at a measure boundary",
            ));
        }
        if measure == range.start {
            start_tick = Some(tick);
        }
        if measure == range.end {
            let start_tick = start_tick.expect("range start resolved");
            return Ok(MidiMeasureBounds {
                start_tick,
                end_tick: end,
                start_seconds: seconds_at(start_tick, ppqn, midi),
                end_seconds: seconds_at(end, ppqn, midi),
            });
        }
        tick = end;
        measure = measure
            .checked_add(1)
            .ok_or_else(|| AppError::unsupported("MIDI measure range overflow"))?;
    }
    unreachable!("measure loop always returns")
}

fn seconds_at(tick: u64, ppqn: u64, midi: &MidiFile) -> f64 {
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

struct RawMidi {
    format: u16,
    division: u16,
    tracks: Vec<Vec<RawEvent>>,
    last_tick: u64,
}

struct RawEvent {
    tick: u64,
    serial: usize,
    kind: RawEventKind,
}

enum RawEventKind {
    Channel(u8, u8, Option<u8>),
    Meta(u8, Vec<u8>),
    SysEx(u8, Vec<u8>),
}

struct NoteInterval {
    on_tick: u64,
    off_tick: u64,
    channel: u8,
    pitch: u8,
    velocity: u8,
    serial: usize,
}

struct ActiveNote {
    on_tick: u64,
    velocity: u8,
    serial: usize,
}

struct SliceEvent {
    tick: u64,
    priority: u8,
    serial: usize,
    kind: RawEventKind,
}

/// Slice a Standard MIDI File by inclusive one-based measures.
///
/// The output preserves channel, meta and SysEx events, rebases ticks to zero,
/// and carries notes that overlap the start and end boundaries.
pub fn slice_midi_measures(bytes: &[u8], range: MeasureRange) -> Result<Vec<u8>, AppError> {
    let midi = crate::parse_midi(bytes)?;
    let bounds = midi_measure_bounds(&midi, range)?;
    let raw = parse_raw(bytes)?;
    if bounds.end_tick > raw.last_tick {
        return Err(AppError::new(
            ErrorCode::InvalidInput,
            format!(
                "measure range exceeds the score's {} measures",
                measure_count(&midi, raw.last_tick)
            ),
        ));
    }
    let mut tracks = Vec::with_capacity(raw.tracks.len());
    for events in &raw.tracks {
        tracks.push(slice_track(events, bounds)?);
    }
    write_midi(
        raw.format,
        raw.division,
        bounds.end_tick - bounds.start_tick,
        &tracks,
    )
}

fn measure_count(midi: &MidiFile, last_tick: u64) -> usize {
    let Some(ppqn) = (match midi.division {
        MidiDivision::Ppqn { ticks_per_quarter } if midi.format != 2 => {
            Some(u64::from(ticks_per_quarter))
        }
        _ => None,
    }) else {
        return 0;
    };
    let mut count = 0usize;
    let mut start = 0u64;
    for (index, signature) in midi.time_signatures.iter().enumerate() {
        if signature.tick != start {
            break;
        }
        let Some(length) = measure_ticks(ppqn, signature) else {
            break;
        };
        let next = midi
            .time_signatures
            .get(index + 1)
            .map_or(last_tick, |change| change.tick);
        if next < start {
            break;
        }
        count = count.saturating_add(usize::try_from((next - start) / length).unwrap_or(0));
        if next >= last_tick {
            break;
        }
        start = next;
    }
    count
}

fn parse_raw(bytes: &[u8]) -> Result<RawMidi, AppError> {
    let mut reader = Reader::new(bytes);
    if reader.take(4)? != b"MThd" {
        return Err(AppError::invalid("not a Standard MIDI File"));
    }
    let header_len = reader.u32()? as usize;
    if header_len < 6 {
        return Err(AppError::invalid("short MIDI header"));
    }
    let mut header = Reader::new(reader.take(header_len)?);
    let format = header.u16()?;
    let track_count = header.u16()?;
    let division = header.u16()?;
    let mut tracks = Vec::with_capacity(usize::from(track_count));
    let mut last_tick = 0u64;
    let mut serial: usize = Default::default();
    for _ in 0..track_count {
        if reader.take(4)? != b"MTrk" {
            return Err(AppError::invalid("bad MIDI track header"));
        }
        let track_len = reader.u32()? as usize;
        let mut track = Reader::new(reader.take(track_len)?);
        let mut events = Vec::new();
        let mut tick = 0u64;
        let mut running = None;
        let mut ended = false;
        while track.pos < track.bytes.len() {
            tick = tick
                .checked_add(u64::from(track.vlq()?))
                .ok_or_else(|| AppError::invalid("MIDI time overflow"))?;
            let next = *track
                .bytes
                .get(track.pos)
                .ok_or_else(|| AppError::invalid("missing MIDI event"))?;
            let status = if next & 0x80 != 0 {
                track.pos += 1;
                next
            } else {
                running.ok_or_else(|| AppError::invalid("no MIDI running status"))?
            };
            let kind = match status {
                0x80..=0xef => {
                    running = Some(status);
                    let first = track.data()?;
                    let second = if status & 0xf0 != 0xc0 && status & 0xf0 != 0xd0 {
                        Some(track.data()?)
                    } else {
                        None
                    };
                    RawEventKind::Channel(status, first, second)
                }
                0xff => {
                    running = None;
                    let kind = track.data()?;
                    let len = track.vlq()? as usize;
                    let data = track.take(len)?.to_vec();
                    if kind == 0x2f {
                        if !data.is_empty() || track.pos != track.bytes.len() {
                            return Err(AppError::invalid("invalid end-of-track event"));
                        }
                        ended = true;
                    }
                    RawEventKind::Meta(kind, data)
                }
                0xf0 | 0xf7 => {
                    running = None;
                    let len = track.vlq()? as usize;
                    RawEventKind::SysEx(status, track.take(len)?.to_vec())
                }
                _ => return Err(AppError::invalid("unsupported status in MIDI track")),
            };
            last_tick = last_tick.max(tick);
            events.push(RawEvent { tick, serial, kind });
            serial += 1;
        }
        if !ended {
            return Err(AppError::invalid("MIDI track has no end-of-track event"));
        }
        tracks.push(events);
    }
    if reader.pos != bytes.len() {
        return Err(AppError::invalid("unexpected bytes after MIDI tracks"));
    }
    Ok(RawMidi {
        format,
        division,
        tracks,
        last_tick,
    })
}

fn slice_track(
    events: &[RawEvent],
    bounds: MidiMeasureBounds,
) -> Result<Vec<SliceEvent>, AppError> {
    let mut active: HashMap<(u8, u8), VecDeque<ActiveNote>> = HashMap::new();
    let mut notes = Vec::new();
    for event in events {
        let RawEventKind::Channel(status, first, Some(second)) = &event.kind else {
            continue;
        };
        let kind = status & 0xf0;
        let channel = status & 0x0f;
        if kind == 0x90 && *second > 0 {
            active
                .entry((channel, *first))
                .or_default()
                .push_back(ActiveNote {
                    on_tick: event.tick,
                    velocity: *second,
                    serial: event.serial,
                });
        } else if kind == 0x80 || (kind == 0x90 && *second == 0) {
            if let Some(pending) = active.get_mut(&(channel, *first)) {
                if let Some(on) = pending.pop_front() {
                    if event.tick < on.on_tick {
                        return Err(AppError::invalid("MIDI note-off precedes note-on"));
                    }
                    notes.push(NoteInterval {
                        on_tick: on.on_tick,
                        off_tick: event.tick,
                        channel,
                        pitch: *first,
                        velocity: on.velocity,
                        serial: on.serial,
                    });
                }
            }
        }
    }
    if active.values().any(|pending| !pending.is_empty()) {
        return Err(AppError::unsupported(
            "MIDI range selection requires complete note-off timing",
        ));
    }

    let mut latest_state_meta = HashMap::<u8, usize>::new();
    let mut state_meta_at_start = HashMap::<u8, bool>::new();
    for (index, event) in events.iter().enumerate() {
        let RawEventKind::Meta(kind @ (0x51 | 0x58), _) = &event.kind else {
            continue;
        };
        if event.tick < bounds.start_tick {
            latest_state_meta.insert(*kind, index);
        } else if event.tick == bounds.start_tick {
            state_meta_at_start.insert(*kind, true);
        }
    }

    let mut sliced = Vec::new();
    for (index, event) in events.iter().enumerate() {
        if matches!(
            &event.kind,
            RawEventKind::Channel(status, _, _)
                if matches!(status & 0xf0, 0x80 | 0x90)
        ) || matches!(&event.kind, RawEventKind::Meta(0x2f, _))
        {
            continue;
        }
        if let RawEventKind::Meta(kind @ (0x51 | 0x58), _) = &event.kind {
            if event.tick < bounds.start_tick
                && (state_meta_at_start.get(kind).copied().unwrap_or(false)
                    || latest_state_meta.get(kind) != Some(&index))
            {
                continue;
            }
        }
        let tick = if event.tick < bounds.start_tick {
            Some(0)
        } else if event.tick < bounds.end_tick {
            Some(event.tick - bounds.start_tick)
        } else {
            None
        };
        if let Some(tick) = tick {
            sliced.push(SliceEvent {
                tick,
                priority: 0,
                serial: event.serial,
                kind: clone_kind(&event.kind),
            });
        }
    }
    for note in notes {
        let start = note.on_tick.max(bounds.start_tick);
        let end = note.off_tick.min(bounds.end_tick);
        if start >= end {
            continue;
        }
        sliced.push(SliceEvent {
            tick: start - bounds.start_tick,
            priority: 2,
            serial: note.serial,
            kind: RawEventKind::Channel(0x90 | note.channel, note.pitch, Some(note.velocity)),
        });
        sliced.push(SliceEvent {
            tick: end - bounds.start_tick,
            priority: 1,
            serial: note.serial,
            kind: RawEventKind::Channel(0x80 | note.channel, note.pitch, Some(0)),
        });
    }
    sliced.sort_by_key(|event| (event.tick, event.priority, event.serial));
    Ok(sliced)
}

fn clone_kind(kind: &RawEventKind) -> RawEventKind {
    match kind {
        RawEventKind::Channel(status, first, second) => {
            RawEventKind::Channel(*status, *first, *second)
        }
        RawEventKind::Meta(kind, data) => RawEventKind::Meta(*kind, data.clone()),
        RawEventKind::SysEx(status, data) => RawEventKind::SysEx(*status, data.clone()),
    }
}

fn write_midi(
    format: u16,
    division: u16,
    length: u64,
    tracks: &[Vec<SliceEvent>],
) -> Result<Vec<u8>, AppError> {
    let mut output = Vec::new();
    output.extend_from_slice(b"MThd");
    output.extend_from_slice(&6u32.to_be_bytes());
    output.extend_from_slice(&format.to_be_bytes());
    output.extend_from_slice(
        &u16::try_from(tracks.len())
            .map_err(|_| AppError::unsupported("too many MIDI tracks"))?
            .to_be_bytes(),
    );
    output.extend_from_slice(&division.to_be_bytes());
    for track in tracks {
        let mut body = Vec::new();
        let mut previous = 0u64;
        for event in track {
            write_vlq(&mut body, event.tick - previous)?;
            previous = event.tick;
            match &event.kind {
                RawEventKind::Channel(status, first, second) => {
                    body.push(*status);
                    body.push(*first);
                    if let Some(second) = second {
                        body.push(*second);
                    }
                }
                RawEventKind::Meta(kind, data) => {
                    body.push(0xff);
                    body.push(*kind);
                    write_vlq(&mut body, data.len() as u64)?;
                    body.extend_from_slice(data);
                }
                RawEventKind::SysEx(status, data) => {
                    body.push(*status);
                    write_vlq(&mut body, data.len() as u64)?;
                    body.extend_from_slice(data);
                }
            }
        }
        write_vlq(&mut body, length - previous)?;
        body.extend_from_slice(&[0xff, 0x2f, 0]);
        output.extend_from_slice(b"MTrk");
        output.extend_from_slice(
            &u32::try_from(body.len())
                .map_err(|_| AppError::unsupported("MIDI track is too large"))?
                .to_be_bytes(),
        );
        output.extend_from_slice(&body);
    }
    Ok(output)
}

fn write_vlq(output: &mut Vec<u8>, value: u64) -> Result<(), AppError> {
    if value > 0x0fff_ffff {
        return Err(AppError::unsupported("MIDI delta time exceeds four bytes"));
    }
    let mut bytes = [0u8; 4];
    let mut index = bytes.len() - 1;
    bytes[index] = (value & 0x7f) as u8;
    let mut remaining = value >> 7;
    while remaining > 0 {
        index -= 1;
        bytes[index] = ((remaining & 0x7f) as u8) | 0x80;
        remaining >>= 7;
    }
    output.extend_from_slice(&bytes[index..]);
    Ok(())
}
