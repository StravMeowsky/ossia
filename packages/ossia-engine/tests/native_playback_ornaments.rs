use ossia_engine::*;
use ossia_score::{
    build_index, build_layout, build_playback, parse_midi, playback_measure_bounds, MidiFile,
    ScoreLayout, SvgPage,
};
use ossia_types::{MeasureRange, ScorePlayback};
use std::{fs, path::PathBuf};

fn compile_playback(source: &str) -> (ScoreLayout, MidiFile, ScorePlayback, ScorePlayback) {
    let root = PathBuf::from(std::env::var_os("OSSIA_TEST_LILYPOND_ROOT").unwrap());
    let work = tempfile::tempdir().unwrap();
    let context = RunContext::default();
    let engine = Engine::verify(&root, &work.path().join("scratch"), &context).unwrap();
    let assets = work.path().join("assets");
    let snapshot = SourceSnapshot::new(DocumentId::new(), source.as_bytes().to_vec()).unwrap();
    let render = engine
        .compile(
            &snapshot,
            &CompileOptions {
                formats: vec![RenderFormat::Svg, RenderFormat::Midi],
                point_and_click: true,
            },
            &assets,
            &context,
        )
        .unwrap();
    assert!(
        render.format_errors.is_empty(),
        "{:?}",
        render.format_errors
    );
    let svg = fs::read_to_string(assets.join(&render.pages[0].artifact.path)).unwrap();
    let midi = parse_midi(&fs::read(assets.join(render.midi.unwrap().path)).unwrap()).unwrap();
    let index = build_index(source);
    let mut layout = build_layout(&index, &[SvgPage { page: 1, svg: &svg }]).unwrap();
    let playback = build_playback(&index, &layout, &midi, snapshot.source_hash().clone()).unwrap();
    let original = layout.clone();
    for object in layout.pages.iter_mut().flat_map(|page| &mut page.objects) {
        object.ornament = None;
    }
    let baseline = build_playback(&index, &layout, &midi, snapshot.source_hash().clone()).unwrap();
    (original, midi, playback, baseline)
}

#[test]
#[ignore = "requires native LilyPond 2.26.0"]
fn native_fermatas_hold_the_release_once_and_shift_all_voices() {
    let source = r#"\version "2.26.0"
\score {
  \new PianoStaff <<
    \new Staff {
      \tempo 4 = 60
      c'4 d'4\fermata e'4 r4\fermata |
      f'4~ f'4\fermata <g' b'>2\fermata |
    }
    \new Staff { g,2\fermata a,2 | r2 c2 | }
  >>
  \layout {} \midi {}
}
"#;
    let (layout, midi, playback, baseline) = compile_playback(source);
    let timing = |pitch| {
        let note = playback
            .notes
            .iter()
            .find(|note| note.pitch == pitch)
            .unwrap();
        (note.start_seconds, note.duration_seconds)
    };
    assert_eq!(timing(60), (0., 1.));
    assert_eq!(timing(62), (1., 3.));
    assert_eq!(timing(43), (0., 4.));
    assert_eq!(timing(64), (4., 1.));
    assert_eq!(timing(45), (4., 3.));
    assert_eq!(timing(65), (7., 3.));
    for pitch in [48, 67, 71] {
        assert_eq!(timing(pitch), (10., 4.));
    }
    assert!(playback.groups.iter().any(|group| group.pitches.is_empty()
        && group.start_seconds == Some(5.)
        && group.duration_seconds == Some(2.)));
    assert_eq!(playback.duration_seconds, 14.);
    assert_eq!(baseline.duration_seconds, 8.);
    let index = build_index(source);
    let bounds = |measure| {
        playback_measure_bounds(
            &index,
            &layout,
            &midi,
            MeasureRange {
                start: measure,
                end: measure,
            },
        )
        .unwrap()
    };
    assert_eq!(bounds(1), 0. ..7.);
    assert_eq!(bounds(2), 7. ..14.);
}

#[test]
#[ignore = "requires native LilyPond 2.26.0"]
fn native_fermata_on_full_measure_rest_delays_the_next_attack() {
    let source = r#"\version "2.26.0"
\score {
  \new Staff { \time 4/4 \tempo 4 = 60 c'1 | R1\fermata | d'1 | R1\fermata | }
  \layout {} \midi {}
}
"#;
    let (layout, midi, playback, _) = compile_playback(source);
    assert_eq!(playback.notes.len(), 2);
    assert_eq!(playback.notes[0].duration_seconds, 4.);
    assert_eq!(playback.notes[1].start_seconds, 12.);
    assert_eq!(playback.duration_seconds, 24.);
    assert!(playback.systems.iter().any(|span| span.measure_end == 4));
    let final_measure = MeasureRange { start: 4, end: 4 };
    assert_eq!(
        playback_measure_bounds(&build_index(source), &layout, &midi, final_measure).unwrap(),
        16. ..24.
    );
}

#[test]
#[ignore = "requires native LilyPond 2.26.0"]
fn native_graces_distinguish_long_and_slashed_notation() {
    let source = r#"\version "2.26.0"
\score {
  \new Staff {
    \time 4/4 \tempo 4 = 60
    c'4 \appoggiatura d'16 e'4 \acciaccatura f'16 g'4 \slashedGrace a'16 b'4 |
    \appoggiatura b'4 c''4. r8 \appoggiatura { d''16 e''16 } f''4~ f''8 \afterGrace g''8 { a''16 } |
    \grace b''16 c'''1 |
  }
  \layout {} \midi {}
}
"#;
    let (layout, _, playback, baseline) = compile_playback(source);
    let grace_kinds = layout.pages[0]
        .objects
        .iter()
        .filter(|object| object.note.is_some() && !object.hit_only)
        .filter_map(|object| object.ornament.as_deref())
        .collect::<std::collections::HashSet<_>>();
    assert_eq!(grace_kinds, ["appoggiatura", "acciaccatura"].into());
    let actual_note = |pitch| {
        playback
            .notes
            .iter()
            .find(|note| note.pitch == pitch)
            .unwrap()
    };
    for (pitch, start, duration) in [
        (60, 0., 1.),
        (62, 1., 0.25),
        (72, 5., 0.5),
        (74, 6., 0.25),
        (76, 6.25, 0.25),
        (77, 6.5, 1.),
    ] {
        let note = actual_note(pitch);
        assert_eq!(
            (note.start_seconds, note.duration_seconds),
            (start, duration)
        );
    }
    for pitch in [65, 69, 81, 83, 84] {
        let note = actual_note(pitch);
        let original = baseline
            .notes
            .iter()
            .find(|note| note.pitch == pitch)
            .unwrap();
        assert_eq!(
            (note.start_seconds, note.duration_seconds),
            (original.start_seconds, original.duration_seconds)
        );
        assert_eq!(note.ornament_anchor.is_some(), [65, 69].contains(&pitch));
    }
    for note in playback
        .notes
        .iter()
        .filter(|note| note.ornament_anchor.is_some())
    {
        assert!(playback.groups.iter().any(|group| {
            group
                .element_ids
                .contains(note.ornament_anchor.as_ref().unwrap())
                && group.pitches.contains(&note.pitch)
                && group.start_seconds == Some(note.start_seconds)
                && group.duration_seconds == Some(note.duration_seconds)
        }));
    }
}

#[test]
#[ignore = "requires native LilyPond 2.26.0"]
fn native_leading_appoggiatura_and_parallel_voice_share_the_beat() {
    let source = r#"\version "2.26.0"
\score {
  \new PianoStaff <<
    \new Staff { \tempo 4 = 60 \appoggiatura d'16 e'4 c' \appoggiatura d'16 e' e' }
    \new Staff { b4 b b b }
  >>
  \layout {} \midi {}
}
"#;
    let (_, _, playback, baseline) = compile_playback(source);
    let bass = |notes: &[ossia_types::PlaybackNote]| {
        notes
            .iter()
            .filter(|note| note.pitch == 59)
            .cloned()
            .collect::<Vec<_>>()
    };
    assert_eq!(bass(&playback.notes), bass(&baseline.notes));
    let first_beat = bass(&playback.notes)[0].start_seconds;
    let grace = playback.notes.iter().find(|note| note.pitch == 62).unwrap();
    let principal = playback.notes.iter().find(|note| note.pitch == 64).unwrap();
    assert!((grace.duration_seconds - 0.25).abs() < 1e-9);
    assert_eq!(grace.start_seconds, first_beat);
    assert_eq!(principal.start_seconds, first_beat + 0.25);
    let first_group = playback
        .groups
        .iter()
        .find(|group| group.start_seconds == Some(first_beat))
        .unwrap();
    assert_eq!(first_group.pitches, [62]);
}

#[test]
#[ignore = "requires native LilyPond 2.26.0"]
fn native_ornaments_follow_accidentals_ties_and_arpeggio_direction() {
    let source = r#"\version "2.26.0"
\score {
  \new Staff {
    \key g \major \time 4/4 \tempo 4 = 60
    e'4\prall f'4 e'4\prall g'4 |
    e'4\prall~ e'4\mordent \arpeggioArrowDown <g' b' d''>4\arpeggio c''4 |
    g'16\prallmordent r16 r8 r2. |
  }
  \layout {} \midi {}
}
"#;
    let (_, _, playback, baseline) = compile_playback(source);
    assert_eq!(playback.groups, baseline.groups);
    assert_eq!(playback.systems, baseline.systems);
    assert_eq!(playback.duration_seconds, baseline.duration_seconds);

    let attacks = |start: f64, end: f64| {
        playback.notes.iter().filter(move |note| {
            note.ornament_anchor.is_some() && (start..end).contains(&note.start_seconds)
        })
    };
    let pitches = |start, end| {
        attacks(start, end)
            .map(|note| note.pitch)
            .collect::<Vec<_>>()
    };
    assert_eq!(pitches(0., 1.), [66, 64, 66, 64]);
    assert_eq!(pitches(2., 3.), [65, 64, 65, 64]);
    assert_eq!(pitches(4., 6.), [66, 64, 66, 64]);
    let held = attacks(4., 6.).next_back().unwrap();
    assert_eq!(held.start_seconds + held.duration_seconds, 6.);
    assert_eq!(pitches(6., 7.), [74, 71, 67]);
    assert_eq!(pitches(8., 9.), [69, 67, 69, 67, 66, 67]);
    assert!(attacks(8., 9.).all(|note| (note.duration_seconds - 0.25 / 6.).abs() < 1e-6));
    let untouched = [1., 3., 7.];
    for onset in untouched {
        assert_eq!(
            playback
                .notes
                .iter()
                .find(|note| note.start_seconds == onset),
            baseline
                .notes
                .iter()
                .find(|note| note.start_seconds == onset),
        );
    }
}
