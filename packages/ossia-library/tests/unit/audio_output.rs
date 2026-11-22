use super::clip_playback;
use ossia_score::{MidiDivision, MidiFile, MidiTimeSignature};
use ossia_types::*;

#[test]
fn clips_playback_to_overlapping_measure_range() {
    let hash = ContentHash::of(b"clip");
    let playback = ScorePlayback {
        render_key: hash.clone(),
        source_hash: hash,
        duration_seconds: 8.0,
        parts: Vec::new(),
        notes: vec![
            PlaybackNote {
                part_id: "a".into(),
                instrument: SampleInstrument::Guitar,
                pitch: 60,
                start_seconds: 1.0,
                duration_seconds: 1.5,
            },
            PlaybackNote {
                part_id: "a".into(),
                instrument: SampleInstrument::Guitar,
                pitch: 62,
                start_seconds: 1.75,
                duration_seconds: 0.5,
            },
            PlaybackNote {
                part_id: "b".into(),
                instrument: SampleInstrument::Violin,
                pitch: 64,
                start_seconds: 2.5,
                duration_seconds: 1.0,
            },
            PlaybackNote {
                part_id: "a".into(),
                instrument: SampleInstrument::Guitar,
                pitch: 65,
                start_seconds: 6.5,
                duration_seconds: 1.0,
            },
        ],
        groups: Vec::new(),
        systems: vec![PlaybackSystemSpan {
            page: 1,
            system_index: 0,
            measure_start: 1,
            measure_end: 4,
            start_seconds: 0.0,
            end_seconds: 8.0,
            x0: 0.0,
            x1: 1.0,
            y0: 0.0,
            y1: 1.0,
        }],
    };
    let midi = MidiFile {
        format: 1,
        division: MidiDivision::Ppqn {
            ticks_per_quarter: 96,
        },
        tracks: Vec::new(),
        time_signatures: vec![MidiTimeSignature {
            tick: 0,
            numerator: 4,
            denominator: 4,
        }],
        tempos: Vec::new(),
    };
    let clipped = clip_playback(playback, &midi, MeasureRange { start: 2, end: 3 }).unwrap();
    assert_eq!(clipped.notes[2].part_id, "b");
    assert_eq!(clipped.notes[2].instrument, SampleInstrument::Violin);
    assert_eq!(clipped.duration_seconds, 4.0);
    assert_eq!(
        clipped
            .notes
            .iter()
            .map(|note| (note.pitch, note.start_seconds, note.duration_seconds))
            .collect::<Vec<_>>(),
        vec![(60, 0.0, 0.5), (62, 0.0, 0.25), (64, 0.5, 1.0)]
    );
}
