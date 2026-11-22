use crate::MidiFile;
pub use ossia_types::SampleInstrument;

/// Resolve an explicit instrument name without guessing from musical content.
pub fn named_sample_instrument(name: &str) -> Option<SampleInstrument> {
    match name.trim().to_lowercase().as_str() {
        "ukulele" | "uke" | "尤克里里" => Some(SampleInstrument::Ukulele),
        "guitar"
        | "acoustic guitar"
        | "acoustic guitar (nylon)"
        | "acoustic guitar (steel)"
        | "吉他"
        | "古典吉他"
        | "木吉他" => Some(SampleInstrument::Guitar),
        "violin" | "小提琴" => Some(SampleInstrument::Violin),
        "piano" | "acoustic grand" | "acoustic grand piano" | "钢琴" => {
            Some(SampleInstrument::Piano)
        }
        _ => None,
    }
}

/// Resolve a General MIDI program to a bundled sample bank.
pub fn program_sample_instrument(program: u8) -> Option<SampleInstrument> {
    match program {
        0..=7 => Some(SampleInstrument::Piano),
        24 | 25 => Some(SampleInstrument::Guitar),
        40 => Some(SampleInstrument::Violin),
        _ => None,
    }
}

/// Select a sample bank from explicit metadata, defaulting to piano.
pub fn select_sample_instrument(name: Option<&str>, program: Option<u8>) -> SampleInstrument {
    name.and_then(named_sample_instrument)
        .or_else(|| program.and_then(program_sample_instrument))
        .unwrap_or(SampleInstrument::Piano)
}

/// Indexed MIDI program changes shared by all notes in a playback build.
pub(crate) struct ProgramTimeline {
    channels: [Vec<(u64, Option<u8>, bool)>; 16],
}

impl ProgramTimeline {
    pub(crate) fn new(midi: &MidiFile) -> Self {
        let mut channels: [Vec<_>; 16] = std::array::from_fn(|_| Vec::new());
        for change in midi.tracks.iter().flat_map(|track| &track.programs) {
            if let Some(channel) = channels.get_mut(change.channel as usize) {
                channel.push((change.time, change.program));
            }
        }
        Self {
            channels: channels.map(|mut changes| {
                changes.sort_unstable();
                let mut result = Vec::new();
                let mut first = changes.first().filter(|(tick, _)| *tick > 0).map(|_| 0);
                let mut changed = false;
                let mut i = 0;
                while i < changes.len() {
                    let (tick, program) = changes[i];
                    let mut end = i + 1;
                    while end < changes.len() && changes[end].0 == tick {
                        end += 1;
                    }
                    let value = changes[i..end]
                        .iter()
                        .all(|entry| entry.1 == program)
                        .then_some(program);
                    if let Some(value) = value {
                        if let Some(first) = first {
                            changed |= first != value;
                        } else {
                            first = Some(value);
                        }
                    }
                    result.push((tick, value, changed));
                    i = end;
                }
                result
            }),
        }
    }

    pub(crate) fn at(&self, channel: u8, tick: u64) -> (Option<u8>, bool) {
        let Some(changes) = self.channels.get(channel as usize) else {
            return (None, false);
        };
        let pos = changes.partition_point(|change| change.0 <= tick);
        if pos == 0 {
            (None, false)
        } else {
            (changes[pos - 1].1, changes[pos - 1].2)
        }
    }
}

/// Resolve a channel's program at a tick, rejecting conflicting simultaneous events.
pub fn midi_program_at(midi: &MidiFile, channel: u8, tick: u64) -> Option<u8> {
    ProgramTimeline::new(midi).at(channel, tick).0
}
