use ossia_score::{concat_midi, parse_midi};
use ossia_types::ErrorCode;

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

fn track(events: &[(u64, Vec<u8>)], end_tick: u64) -> Vec<u8> {
    let mut body = Vec::new();
    let mut previous = 0;
    for (tick, event) in events {
        body.extend(vlq(tick - previous));
        body.extend(event);
        previous = *tick;
    }
    body.extend(vlq(end_tick - previous));
    body.extend([0xff, 0x2f, 0]);
    let mut chunk = b"MTrk".to_vec();
    chunk.extend((body.len() as u32).to_be_bytes());
    chunk.extend(body);
    chunk
}

fn smf(division: u16, tracks: &[Vec<u8>]) -> Vec<u8> {
    let mut midi = b"MThd".to_vec();
    midi.extend(6u32.to_be_bytes());
    midi.extend(1u16.to_be_bytes());
    midi.extend((tracks.len() as u16).to_be_bytes());
    midi.extend(division.to_be_bytes());
    for track in tracks {
        midi.extend(track);
    }
    midi
}

fn time_signature(numerator: u8, denominator_power: u8) -> Vec<u8> {
    vec![0xff, 0x58, 0x04, numerator, denominator_power, 0x18, 0x08]
}

/// Two movements as LilyPond writes them: a conductor track and one staff.
fn movements() -> (Vec<u8>, Vec<u8>) {
    let first = smf(
        96,
        &[
            track(&[(0, time_signature(3, 2))], 672),
            track(&[(0, vec![0x90, 67, 90]), (96, vec![0x80, 67, 0])], 672),
        ],
    );
    let second = smf(
        96,
        &[
            track(&[(0, time_signature(2, 2))], 384),
            track(&[(0, vec![0x90, 76, 90]), (96, vec![0x80, 76, 0])], 384),
        ],
    );
    (first, second)
}

#[test]
fn joins_movements_at_their_music_lengths() {
    let (first, second) = movements();
    // 7/4 whole notes at 96 PPQN = 672 ticks, the second score's offset.
    let joined = concat_midi(&[first, second], &[(7, 4), (1, 1)]).unwrap();
    let midi = parse_midi(&joined).unwrap();
    assert_eq!(midi.tracks.len(), 2);
    let notes: Vec<_> = midi.tracks[1]
        .notes
        .iter()
        .map(|note| (note.note, note.time, note.duration))
        .collect();
    assert_eq!(notes, vec![(67, 0, Some(96)), (76, 672, Some(96))]);
    let signatures: Vec<_> = midi
        .time_signatures
        .iter()
        .map(|signature| (signature.tick, signature.numerator))
        .collect();
    assert_eq!(signatures, vec![(0, 3), (672, 2)]);
}

#[test]
fn rejects_parts_that_outrun_their_score_length() {
    let (first, second) = movements();
    let error = concat_midi(&[first, second], &[(1, 4), (1, 1)]).unwrap_err();
    assert_eq!(error.code, ErrorCode::InvalidInput);
}

#[test]
fn rejects_mismatched_lengths() {
    let (first, _) = movements();
    let error = concat_midi(&[first], &[]).unwrap_err();
    assert_eq!(error.code, ErrorCode::InvalidInput);
}
