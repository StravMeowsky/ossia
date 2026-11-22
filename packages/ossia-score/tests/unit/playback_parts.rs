use super::*;
use crate::{build_index, MidiDivision, MidiNote, MidiProgram, MidiTrack};
use ossia_types::SampleInstrument;

#[test]
fn score_level_instrument_metadata_is_explicit_and_unambiguous() {
    assert_eq!(
        score_instrument(r#"\header { instrument = "Violin" } \score { \new Staff { c'4 } }"#),
        Some(SampleInstrument::Violin)
    );
    assert_eq!(
        score_instrument(
            r#"\score { \new TabStaff \with { stringTunings = #ukulele-tuning } { c'4 } }"#
        ),
        Some(SampleInstrument::Ukulele)
    );
    assert_eq!(
        score_instrument(
            r#"% instrument = "Violin"
      \header { title = "Violin concerto" } \score { \new Staff { c'4 } }"#
        ),
        None
    );
    assert_eq!(
        score_instrument(r#"\header { instrument = "Violin" } \header { instrument = "Guitar" }"#),
        None
    );
    assert_eq!(
        score_instrument(
            r#"\header { instrument = "Flute" } \new TabStaff \with { stringTunings = #ukulele-tuning } { c'4 }"#
        ),
        None
    );
}

#[test]
fn piano_staves_share_a_setting_without_merging_other_instruments() {
    let index = build_index(
        r#"\score { \new PianoStaff <<
      \new Staff = "right" { c'4 }
      \new Staff = "left" { c4 }
    >> }"#,
    );
    assert_eq!(index.staves.len(), 2);
    assert!(index.staves[0].piano_group.is_some());
    assert_eq!(index.staves[0].piano_group, index.staves[1].piano_group);
    let midi = MidiFile {
        format: 1,
        division: MidiDivision::Ppqn {
            ticks_per_quarter: 96,
        },
        tracks: (0..2)
            .map(|track| MidiTrack {
                track,
                name: None,
                programs: vec![],
                notes: vec![MidiNote {
                    note: 60 + track as u8,
                    time: 0,
                    channel: track as u8,
                    duration: Some(96),
                    duration_reason: None,
                }],
            })
            .collect(),
        time_signatures: vec![],
        tempos: vec![],
    };
    let mut notes = (0..2)
        .map(|track| {
            (
                PlaybackNote {
                    pitch: 60 + track as u8,
                    start_seconds: 0.,
                    duration_seconds: 0.5,
                    part_id: format!("{track}:{track}"),
                    instrument: SampleInstrument::Piano,
                },
                0,
                track,
                track as u8,
            )
        })
        .collect::<Vec<_>>();
    let mut groups = (0..2)
        .map(|staff| PlaybackNoteGroup {
            element_ids: vec![],
            pitches: vec![60 + staff as u8],
            staff_index: Some(staff),
            voice: None,
            start_seconds: Some(0.),
            duration_seconds: Some(0.5),
            part_id: Some(format!("{staff}:{staff}")),
        })
        .collect::<Vec<_>>();
    let programs = crate::instruments::ProgramTimeline::new(&midi);
    let parts = assign_parts(&index, &midi, &programs, &mut notes, &mut groups);
    assert_eq!(parts.len(), 1);
    assert_eq!(notes[0].0.part_id, notes[1].0.part_id);
    assert_eq!(groups[0].part_id, groups[1].part_id);

    let inline =
        build_index(r#"\score { \new PianoStaff << \new Staff { c'4 } \new Staff { c4 } >> }"#);
    let edited =
        build_index(r#"\score { \new PianoStaff << \new Staff { d'4 } \new Staff { e4 } >> }"#);
    let mut identities = Vec::new();
    for index in [&inline, &edited] {
        for (track, (note, _, _, _)) in notes.iter_mut().enumerate() {
            note.part_id = format!("{track}:{track}");
        }
        for (staff, group) in groups.iter_mut().enumerate() {
            group.part_id = Some(format!("{staff}:{staff}"));
        }
        identities.push(
            assign_parts(index, &midi, &programs, &mut notes, &mut groups)[0]
                .id
                .clone(),
        );
    }
    assert_eq!(identities[0], identities[1]);
}

#[test]
fn program_change_is_applied_to_later_notes_even_with_a_named_staff() {
    let index = build_index(
        r#"\score { \new Staff = "solo" \with { instrumentName = "Violin" } { c'4 d'4 } }"#,
    );
    let midi = MidiFile {
        format: 1,
        division: MidiDivision::Ppqn {
            ticks_per_quarter: 96,
        },
        tracks: vec![MidiTrack {
            track: 0,
            name: None,
            programs: vec![
                MidiProgram {
                    time: 0,
                    channel: 0,
                    program: 40,
                },
                MidiProgram {
                    time: 96,
                    channel: 0,
                    program: 24,
                },
            ],
            notes: vec![],
        }],
        time_signatures: vec![],
        tempos: vec![],
    };
    let programs = crate::instruments::ProgramTimeline::new(&midi);
    let mut notes = vec![
        (
            PlaybackNote {
                pitch: 60,
                start_seconds: 0.,
                duration_seconds: 0.5,
                part_id: "0:0".into(),
                instrument: SampleInstrument::Violin,
            },
            0,
            0,
            0,
        ),
        (
            PlaybackNote {
                pitch: 62,
                start_seconds: 0.5,
                duration_seconds: 0.5,
                part_id: "0:0".into(),
                instrument: SampleInstrument::Guitar,
            },
            96,
            0,
            0,
        ),
    ];
    let mut groups = vec![PlaybackNoteGroup {
        element_ids: vec![],
        pitches: vec![60],
        staff_index: Some(0),
        voice: None,
        start_seconds: Some(0.),
        duration_seconds: Some(0.5),
        part_id: Some("0:0".into()),
    }];
    let parts = assign_parts(&index, &midi, &programs, &mut notes, &mut groups);
    assert_eq!(parts.len(), 1);
    assert_eq!(notes[0].0.instrument, SampleInstrument::Violin);
    assert_eq!(notes[1].0.instrument, SampleInstrument::Guitar);

    let mut without_initial_program = midi.clone();
    without_initial_program.tracks[0].programs.remove(0);
    let programs = crate::instruments::ProgramTimeline::new(&without_initial_program);
    notes[0].0.part_id = "0:0".into();
    notes[1].0.part_id = "0:0".into();
    groups[0].part_id = Some("0:0".into());
    assign_parts(
        &index,
        &without_initial_program,
        &programs,
        &mut notes,
        &mut groups,
    );
    assert_eq!(notes[0].0.instrument, SampleInstrument::Violin);
    assert_eq!(notes[1].0.instrument, SampleInstrument::Guitar);
}

#[test]
fn context_identifier_does_not_override_explicit_midi_instrument() {
    let index = build_index(r#"\score { \new Staff = "violin" { c'4 } }"#);
    let midi = MidiFile {
        format: 1,
        division: MidiDivision::Ppqn {
            ticks_per_quarter: 96,
        },
        tracks: vec![MidiTrack {
            track: 0,
            name: None,
            programs: vec![MidiProgram {
                time: 0,
                channel: 0,
                program: 24,
            }],
            notes: vec![],
        }],
        time_signatures: vec![],
        tempos: vec![],
    };
    let programs = crate::instruments::ProgramTimeline::new(&midi);
    let mut notes = vec![(
        PlaybackNote {
            pitch: 60,
            start_seconds: 0.,
            duration_seconds: 0.5,
            part_id: "0:0".into(),
            instrument: SampleInstrument::Guitar,
        },
        0,
        0,
        0,
    )];
    let mut groups = vec![PlaybackNoteGroup {
        element_ids: vec![],
        pitches: vec![60],
        staff_index: Some(0),
        voice: None,
        start_seconds: Some(0.),
        duration_seconds: Some(0.5),
        part_id: Some("0:0".into()),
    }];
    assign_parts(&index, &midi, &programs, &mut notes, &mut groups);
    assert_eq!(notes[0].0.instrument, SampleInstrument::Guitar);
}

#[test]
fn unique_staff_identity_survives_reordering_and_ambiguous_identity_does_not() {
    fn ids(source: &str) -> Vec<(String, String)> {
        let index = build_index(source);
        let midi = MidiFile {
            format: 1,
            division: MidiDivision::Ppqn {
                ticks_per_quarter: 96,
            },
            tracks: (0..index.staves.len())
                .map(|track| MidiTrack {
                    track,
                    name: None,
                    programs: vec![],
                    notes: vec![MidiNote {
                        note: 60 + track as u8,
                        time: 0,
                        channel: track as u8,
                        duration: Some(96),
                        duration_reason: None,
                    }],
                })
                .collect(),
            time_signatures: vec![],
            tempos: vec![],
        };
        let mut notes = (0..index.staves.len())
            .map(|track| {
                (
                    PlaybackNote {
                        pitch: 60 + track as u8,
                        start_seconds: 0.,
                        duration_seconds: 0.5,
                        part_id: format!("{track}:{track}"),
                        instrument: SampleInstrument::Piano,
                    },
                    0,
                    track,
                    track as u8,
                )
            })
            .collect::<Vec<_>>();
        let mut groups = (0..index.staves.len())
            .map(|staff| PlaybackNoteGroup {
                element_ids: vec![],
                pitches: vec![60 + staff as u8],
                staff_index: Some(staff),
                voice: None,
                start_seconds: Some(0.),
                duration_seconds: Some(0.5),
                part_id: Some(format!("{staff}:{staff}")),
            })
            .collect::<Vec<_>>();
        let programs = crate::instruments::ProgramTimeline::new(&midi);
        let mut result = assign_parts(&index, &midi, &programs, &mut notes, &mut groups)
            .into_iter()
            .map(|part| (part.name, part.id))
            .collect::<Vec<_>>();
        result.sort();
        result
    }
    let first =
        ids(r#"\score { << \new Staff = "guitar" { c'4 } \new Staff = "violin" { d'4 } >> }"#);
    let reordered =
        ids(r#"\score { << \new Staff = "violin" { d'4 } \new Staff = "guitar" { c'4 } >> }"#);
    assert_eq!(first, reordered);
    assert_eq!(
        ids(r#"\score { \new Staff { c'4 } }"#),
        ids(r#"\score { \new Staff { d'4 } }"#)
    );
    let ambiguous =
        ids(r#"\score { << \new Staff = "same" { c'4 } \new Staff = "same" { d'4 } >> }"#);
    let changed =
        ids(r#"\score { << \new Staff = "same" { d'4 } \new Staff = "same" { c'4 } >> }"#);
    assert!(ambiguous
        .iter()
        .all(|(_, id)| !changed.iter().any(|(_, other)| other == id)));
}
