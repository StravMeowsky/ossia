/// Canonical scientific spelling for a MIDI pitch. MIDI has no enharmonic intent,
/// so this intentionally uses sharps; source tokens remain authoritative.
pub fn midi_pitch_name(pitch: u8) -> String {
    const NAMES: [&str; 12] = [
        "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
    ];
    let octave = i16::from(pitch) / 12 - 1;
    format!("{}{}", NAMES[usize::from(pitch % 12)], octave)
}
