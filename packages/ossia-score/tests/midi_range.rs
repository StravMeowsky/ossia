use ossia_score::{parse_midi, slice_midi_measures};
use ossia_types::{ErrorCode, MeasureRange};

fn vlq(mut value: u64) -> Vec<u8> {
    let mut bytes = [0u8; 4];
    let mut index = bytes.len() - 1;
    bytes[index] = (value & 0x7f) as u8;
    value >>= 7;
    while value > 0 {
        index -= 1;
        bytes[index] = ((value & 0x7f) as u8) | 0x80;
        value >>= 7;
    }
    bytes[index..].to_vec()
}

fn smf(events: &[(u64, Vec<u8>)], end_tick: u64) -> Vec<u8> {
    let mut body = Vec::new();
    let mut previous = 0;
    for (tick, event) in events {
        body.extend(vlq(tick - previous));
        body.extend(event);
        previous = *tick;
    }
    body.extend(vlq(end_tick - previous));
    body.extend([0xff, 0x2f, 0]);

    let mut midi = b"MThd".to_vec();
    midi.extend(6u32.to_be_bytes());
    midi.extend(1u16.to_be_bytes());
    midi.extend(1u16.to_be_bytes());
    midi.extend(96u16.to_be_bytes());
    midi.extend(b"MTrk");
    midi.extend((body.len() as u32).to_be_bytes());
    midi.extend(body);
    midi
}

fn note_on(tick: u64, pitch: u8, velocity: u8) -> (u64, Vec<u8>) {
    (tick, vec![0x90, pitch, velocity])
}

fn note_off(tick: u64, pitch: u8) -> (u64, Vec<u8>) {
    (tick, vec![0x80, pitch, 0])
}

fn fixture() -> Vec<u8> {
    let mut events = vec![
        (0, vec![0xff, 0x58, 0x04, 0x04, 0x02, 0x18, 0x08]),
        (0, vec![0xff, 0x51, 0x03, 0x07, 0xa1, 0x20]),
        (0, vec![0xc0, 5]),
        (0, vec![0xb0, 7, 100]),
        (0, vec![0xf0, 3, 1, 2, 3]),
    ];
    events.extend([
        note_on(100, 67, 91),
        note_off(200, 67),
        note_on(300, 62, 82),
        note_off(500, 62),
        note_on(400, 64, 73),
        note_off(600, 64),
        (400, vec![0xe0, 0, 64]),
        note_on(1100, 65, 64),
        note_off(1300, 65),
        note_on(1200, 69, 70),
        note_off(1300, 69),
    ]);
    events.sort_by_key(|(tick, _)| *tick);
    smf(&events, 1536)
}

#[test]
fn slices_measures_and_preserves_state_and_boundary_notes() {
    let output = slice_midi_measures(&fixture(), MeasureRange { start: 2, end: 3 }).unwrap();
    assert!(output.starts_with(b"MThd"));
    let midi = parse_midi(&output).unwrap();
    assert_eq!(midi.time_signatures[0].tick, 0);
    assert_eq!(midi.tempos[0].tick, 0);
    let notes = midi.tracks[0]
        .notes
        .iter()
        .map(|note| (note.note, note.time, note.duration))
        .collect::<Vec<_>>();
    assert_eq!(
        notes,
        vec![(62, 0, Some(116)), (64, 16, Some(200)), (65, 716, Some(52))]
    );
    assert!(output.windows(2).any(|window| window == [0xc0, 5]));
    assert!(output.windows(3).any(|window| window == [0xb0, 7, 100]));
    assert!(output.windows(3).any(|window| window == [0xe0, 0, 64]));
    assert!(output.windows(4).any(|window| window == [0xf0, 3, 1, 2]));
}

#[test]
fn follows_time_signature_changes_at_measure_boundaries() {
    let mut events = vec![
        (0, vec![0xff, 0x58, 0x04, 0x04, 0x02, 0x18, 0x08]),
        (1536, vec![0xff, 0x58, 0x04, 0x03, 0x02, 0x18, 0x08]),
        note_on(1536, 60, 80),
        note_off(1824, 60),
        note_on(1824, 62, 80),
        note_off(2112, 62),
    ];
    events.sort_by_key(|(tick, _)| *tick);
    let input = smf(&events, 2112);
    let output = slice_midi_measures(&input, MeasureRange { start: 5, end: 5 }).unwrap();
    let midi = parse_midi(&output).unwrap();
    let notes = &midi.tracks[0].notes;
    assert_eq!(notes.len(), 1);
    assert_eq!(notes[0].note, 60);
    assert_eq!(notes[0].time, 0);
    assert_eq!(notes[0].duration, Some(288));
    assert_eq!(midi.time_signatures[0].numerator, 3);
}

#[test]
fn rejects_invalid_and_out_of_range_measures() {
    assert_eq!(
        slice_midi_measures(&fixture(), MeasureRange { start: 0, end: 1 })
            .unwrap_err()
            .code,
        ErrorCode::InvalidInput
    );
    assert_eq!(
        slice_midi_measures(&fixture(), MeasureRange { start: 3, end: 2 })
            .unwrap_err()
            .code,
        ErrorCode::InvalidInput
    );
    assert_eq!(
        slice_midi_measures(&fixture(), MeasureRange { start: 1, end: 5 })
            .unwrap_err()
            .code,
        ErrorCode::InvalidInput
    );
}
