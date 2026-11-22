use ossia_score::{
    midi_program_at, named_sample_instrument, parse_midi, program_sample_instrument,
    select_sample_instrument, SampleInstrument,
};

#[test]
fn recognizes_only_supported_names_and_programs() {
    assert_eq!(
        named_sample_instrument("尤克里里"),
        Some(SampleInstrument::Ukulele)
    );
    assert_eq!(
        named_sample_instrument("acoustic guitar (nylon)"),
        Some(SampleInstrument::Guitar)
    );
    assert_eq!(
        named_sample_instrument("Violin"),
        Some(SampleInstrument::Violin)
    );
    assert_eq!(
        named_sample_instrument("钢琴"),
        Some(SampleInstrument::Piano)
    );
    assert_eq!(named_sample_instrument("electric guitar"), None);
    assert_eq!(
        program_sample_instrument(24),
        Some(SampleInstrument::Guitar)
    );
    assert_eq!(
        program_sample_instrument(40),
        Some(SampleInstrument::Violin)
    );
    assert_eq!(program_sample_instrument(73), None);
    assert_eq!(
        select_sample_instrument(Some("ukulele"), Some(24)),
        SampleInstrument::Ukulele
    );
    assert_eq!(
        select_sample_instrument(Some("flute"), Some(73)),
        SampleInstrument::Piano
    );
}

#[test]
fn program_lookup_observes_channel_and_time() {
    let track = [0, 0xc1, 24, 12, 0xc1, 40, 0, 0xc2, 0, 0, 0xff, 0x2f, 0];
    let mut bytes = b"MThd\0\0\0\x06\0\0\0\x01\0\x60MTrk".to_vec();
    bytes.extend_from_slice(&(track.len() as u32).to_be_bytes());
    bytes.extend_from_slice(&track);
    let midi = parse_midi(&bytes).unwrap();
    assert_eq!(midi_program_at(&midi, 1, 11), Some(24));
    assert_eq!(midi_program_at(&midi, 1, 12), Some(40));
    assert_eq!(midi_program_at(&midi, 2, 11), None);
    assert_eq!(midi_program_at(&midi, 2, 12), Some(0));
}
