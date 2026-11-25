use ossia_score::*;
use ossia_types::{ContentHash, QuarterTime, SelectionTarget};
use std::{fs, path::PathBuf};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures")
}

fn text(path: &str) -> String {
    fs::read_to_string(root().join(path)).unwrap()
}

fn smf(ppqn: u16, track: &[u8]) -> Vec<u8> {
    let mut file = b"MThd\0\0\0\x06\0\0\0\x01".to_vec();
    file.extend(ppqn.to_be_bytes());
    file.extend(b"MTrk");
    file.extend((track.len() as u32).to_be_bytes());
    file.extend(track);
    file
}

#[test]
fn tempo_meta_events_and_wall_clock_seconds_are_exact() {
    let track = [
        0x00, 0xff, 0x51, 0x03, 0x07, 0xa1, 0x20, // 500000 us at tick 0
        0x00, 0x90, 0x3c, 0x40, 0x83, 0x60, 0x80, 0x3c, 0x00, // 480 ticks
        0x00, 0xff, 0x51, 0x03, 0x03, 0xd0, 0x90, // 250000 us at tick 480
        0x00, 0x90, 0x3e, 0x40, 0x83, 0x60, 0x80, 0x3e, 0x00, // 480 more ticks
        0x00, 0xff, 0x2f, 0x00,
    ];
    let midi = parse_midi(&smf(480, &track)).unwrap();
    assert_eq!(
        midi.tempos,
        vec![
            MidiTempo {
                tick: 0,
                microseconds_per_quarter: 500_000,
            },
            MidiTempo {
                tick: 480,
                microseconds_per_quarter: 250_000,
            },
        ]
    );
    let report = analyze_performance(&midi);
    assert_eq!(report.notes.len(), 2);
    assert_eq!(report.notes[0].pitch, 60);
    assert_eq!(
        report.notes[0].start,
        Some(QuarterTime::new(0, 480).unwrap())
    );
    assert_eq!(
        report.notes[0].duration,
        Some(QuarterTime::new(480, 480).unwrap())
    );
    assert_eq!(report.notes[1].pitch, 62);
    assert_eq!(
        report.notes[1].start,
        Some(QuarterTime::new(480, 480).unwrap())
    );
}

#[test]
fn safe_stem_direction_aliases_do_not_discard_pitch_events() {
    let source = r#"
D = \tweak Stem.direction #DOWN \etc
U = \tweak Stem.direction #UP \etc
Music = \relative c' { \U c4 \D d4 \U <e g>4 }
\score { \new Staff \Music }
"#;
    let report = analyze_source(source);
    assert_ne!(report.status, ossia_types::AnalysisStatus::Unavailable);
    assert_eq!(
        report
            .events
            .iter()
            .flat_map(|event| event.pitches.clone())
            .collect::<Vec<_>>(),
        vec![60, 62, 64, 67]
    );
}

#[test]
fn inline_tweaks_do_not_discard_pitch_events() {
    let source = r#"
Music = \relative c' {
  \tweak Y-offset #1.0 c4
  \tweak direction #UP d4
  \tweak Stem.direction #DOWN <e g>4
}
\score { \new Staff \Music }
"#;
    let report = analyze_source(source);
    assert_ne!(report.status, ossia_types::AnalysisStatus::Unavailable);
    assert_eq!(
        report
            .events
            .iter()
            .flat_map(|event| event.pitches.clone())
            .collect::<Vec<_>>(),
        vec![60, 62, 64, 67]
    );
}

#[test]
fn unbraced_custom_markup_arguments_keep_following_relative_notes() {
    let source = r#"
D = \tweak Stem.direction #DOWN \etc
U = \tweak Stem.direction #UP \etc
Music = \relative a'' {
  \clef "treble" \numericTimeSignature \time 4/4 \key a \minor \break \mark
  \markup \square A a2.. -\markup "dolce, con pedale" -\markup "96 BPM" -\markup
  \bold Andante -\p d8 | % 1
  a2.. e'8 | % 2
}
\score { \new Staff \Music \layout {} \midi {} }
"#;
    let index = build_index(source);
    let voice = index.voice("Music").unwrap();
    let result = resolve_pitches(source, voice.music_span).unwrap();
    assert_eq!(result.unsupported, None);
    assert_eq!(
        result
            .events
            .iter()
            .map(|event| (event.raw.as_str(), event.pitches.clone()))
            .collect::<Vec<_>>(),
        vec![
            ("a2..", vec![81]),
            ("d8", vec![86]),
            ("a2..", vec![81]),
            ("e'8", vec![88]),
        ]
    );
}

#[test]
fn vowel_flat_note_names_lower_pitch_in_relative_music() {
    let source = r#"
Music = \relative as'' { as16 es16 bes'16 es,16 | }
\score { \new Staff \Music \layout {} \midi {} }
"#;
    let index = build_index(source);
    let voice = index.voice("Music").unwrap();
    let result = resolve_pitches(source, voice.music_span).unwrap();
    assert_eq!(result.unsupported, None);
    assert_eq!(
        result
            .events
            .iter()
            .map(|event| (event.raw.as_str(), event.pitches.clone()))
            .collect::<Vec<_>>(),
        vec![
            ("as16", vec![80]),
            ("es16", vec![75]),
            ("bes'16", vec![82]),
            ("es,16", vec![75]),
        ]
    );
}

#[test]
fn unreliable_voice_uses_rendered_measure_and_source_staff() {
    let source = r#"
one = { c'4 d'4 | e'4 f'4 | }
two = { s1*1 c'4 | }
\score { \new StaffGroup <<
  \new Staff = "one" \one
  \new Staff = "two" \two
>> \layout {} \midi {} }
"#;
    let first_c = source.find("c'4").unwrap();
    let first_d = source.find("d'4").unwrap();
    let first_e = source.find("e'4").unwrap();
    let first_f = source.find("f'4").unwrap();
    let later_c = source.rfind("c'4").unwrap();
    let head = |id: &str, start: usize, x: f64, y: f64, staff: usize| {
        format!(
            r#"<g data-ossia-kind="NoteHead" data-ossia-id="{id}" data-ossia-staff="{staff}"
                 data-ossia-bounds="{x} {y} {} {}"
                 data-ossia-source="{start}:{}">
                 <rect x="{x}" y="{y}" width="2" height="2"/>
               </g>"#,
            x + 2.,
            y + 2.,
            start + 3
        )
    };
    let svg = format!(
        r#"<svg viewBox="0 0 100 60">
          <g><line x1="10" x2="90" y1="20" y2="20"/><line x1="10" x2="90" y1="21" y2="21"/>
          <line x1="10" x2="90" y1="22" y2="22"/><line x1="10" x2="90" y1="23" y2="23"/>
          <line x1="10" x2="90" y1="24" y2="24"/></g>
          <g><line x1="10" x2="90" y1="40" y2="40"/><line x1="10" x2="90" y1="41" y2="41"/>
          <line x1="10" x2="90" y1="42" y2="42"/><line x1="10" x2="90" y1="43" y2="43"/>
          <line x1="10" x2="90" y1="44" y2="44"/></g>
          {}
          <g data-ossia-kind="BarLine" data-ossia-bounds="9.8 20 10 44"/>
          <g data-ossia-kind="BarLine" data-ossia-bounds="49.8 20 50 44"/>
          <g data-ossia-kind="BarLine" data-ossia-bounds="89.8 20 90 44"/>
        </svg>"#,
        [
            head("c1", first_c, 20., 21., 0),
            head("d1", first_d, 35., 19., 0),
            head("e1", first_e, 60., 21., 0),
            head("f1", first_f, 75., 19., 0),
            head("c2", later_c, 65., 41., 3),
        ]
        .concat()
    );
    let track = [
        0x00, 0xff, 0x58, 0x04, 0x04, 0x02, 0x18, 0x08, 0x00, 0xff, 0x51, 0x03, 0x07, 0xa1, 0x20,
        0x00, 0x90, 0x3c, 0x40, 0x83, 0x60, 0x80, 0x3c, 0x00, 0x00, 0x90, 0x3e, 0x40, 0x83, 0x60,
        0x80, 0x3e, 0x00, 0x87, 0x40, 0x90, 0x3c, 0x40, 0x83, 0x60, 0x80, 0x3c, 0x00, 0x00, 0x90,
        0x40, 0x40, 0x83, 0x60, 0x80, 0x40, 0x00, 0x00, 0x90, 0x41, 0x40, 0x83, 0x60, 0x80, 0x41,
        0x00, 0x00, 0xff, 0x2f, 0x00,
    ];
    let midi = parse_midi(&smf(480, &track)).unwrap();
    let index = build_index(source);
    let layout = build_layout(&index, &[SvgPage { page: 1, svg: &svg }]).unwrap();
    let playback = build_playback(&index, &layout, &midi, ContentHash::of(b"unreliable")).unwrap();
    let groups = selection_geometry(&index, &layout)
        .unwrap()
        .iter()
        .flat_map(|page| &page.elements)
        .filter_map(|element| {
            let SelectionTarget::Element { element_id } = &element.region.target else {
                return None;
            };
            playback
                .groups
                .iter()
                .find(|group| group.element_ids.contains(element_id))
        })
        .collect::<Vec<_>>();
    assert!(groups.iter().all(|group| group.pitches.len() == 1));
    let fallback = groups
        .iter()
        .find(|group| group.start_seconds == Some(2.0))
        .unwrap();
    assert_eq!(fallback.pitches, vec![60]);
    assert_eq!(fallback.staff_index, Some(1));
    assert_eq!(fallback.voice.as_deref(), Some("two"));
}

#[test]
fn unreliable_voice_matches_the_nearest_onset_inside_a_measure() {
    let source = r#"
one = { c'4 r4 | c'4 r4 | }
two = { s1*1 r4 c'4 | }
\score { \new Staff <<
  \context Voice = "one" \one
  \context Voice = "two" \two
>> \layout {} \midi {} }
"#;
    let first_c = source.find("c'4").unwrap();
    let second_c = source[first_c + 3..].find("c'4").unwrap() + first_c + 3;
    let third_c = source[second_c + 3..].find("c'4").unwrap() + second_c + 3;
    let head = |id: &str, start: usize, x: f64| {
        format!(
            r#"<g data-ossia-kind="NoteHead" data-ossia-id="{id}" data-ossia-staff="0"
                 data-ossia-bounds="{x} 21 {} 23" data-ossia-source="{start}:{}">
                 <rect x="{x}" y="21" width="2" height="2"/>
               </g>"#,
            x + 2.,
            start + 3
        )
    };
    let svg = format!(
        r#"<svg viewBox="0 0 100 60">
          <g><line x1="10" x2="90" y1="20" y2="20"/><line x1="10" x2="90" y1="21" y2="21"/>
          <line x1="10" x2="90" y1="22" y2="22"/><line x1="10" x2="90" y1="23" y2="23"/>
          <line x1="10" x2="90" y1="24" y2="24"/></g>
          {}
          <g data-ossia-kind="BarLine" data-ossia-bounds="9.8 20 10 24"/>
          <g data-ossia-kind="BarLine" data-ossia-bounds="49.8 20 50 24"/>
          <g data-ossia-kind="BarLine" data-ossia-bounds="89.8 20 90 24"/>
        </svg>"#,
        [
            head("c1", first_c, 20.),
            head("c2", second_c, 55.),
            head("c3", third_c, 65.),
        ]
        .concat()
    );
    let track = [
        0x00, 0xff, 0x58, 0x04, 0x04, 0x02, 0x18, 0x08, 0x00, 0xff, 0x51, 0x03, 0x07, 0xa1, 0x20,
        0x00, 0x90, 0x3c, 0x40, 0x83, 0x60, 0x80, 0x3c, 0x00, 0x8b, 0x20, 0x90, 0x3c, 0x40, 0x83,
        0x60, 0x80, 0x3c, 0x00, 0x00, 0x90, 0x3c, 0x40, 0x83, 0x60, 0x80, 0x3c, 0x00, 0x00, 0xff,
        0x2f, 0x00,
    ];
    let midi = parse_midi(&smf(480, &track)).unwrap();
    let index = build_index(source);
    let layout = build_layout(&index, &[SvgPage { page: 1, svg: &svg }]).unwrap();
    let playback = build_playback(&index, &layout, &midi, ContentHash::of(b"onset-hint")).unwrap();
    let pages = selection_geometry(&index, &layout).unwrap();
    let group_start = |start: usize| {
        let element_id = pages
            .iter()
            .flat_map(|page| &page.elements)
            .find_map(|element| {
                (element.region.spans.first()?.byte_start == start)
                    .then_some(&element.region.target)
                    .and_then(|target| match target {
                        SelectionTarget::Element { element_id } => Some(element_id),
                        SelectionTarget::Measure { .. } => None,
                    })
            })
            .unwrap();
        playback
            .groups
            .iter()
            .find(|group| group.element_ids.contains(element_id))
            .unwrap()
            .start_seconds
    };
    assert_eq!(group_start(second_c), Some(2.0));
    assert_eq!(group_start(third_c), Some(2.5));
}

#[test]
fn unmapped_midi_notes_are_not_discarded_when_noteheads_are_covered() {
    let source = r#"
upper = { c'1 | }
lower = { c1 | }
chords = \chordmode { c1 | }
\score { <<
  \new ChordNames \chords
  \new Staff = "upper" \upper
  \new Staff = "lower" \lower
>> \layout {} \midi {} }
"#;
    let upper = source.find("c'1").unwrap();
    let lower = source.find("c1").unwrap();
    let head = |id: &str, start: usize, y: f64, staff: usize| {
        format!(
            r#"<g data-ossia-kind="NoteHead" data-ossia-id="{id}" data-ossia-staff="{staff}"
                 data-ossia-bounds="30 {y} 32 {}"
                 data-ossia-source="{start}:{}">
                 <rect x="30" y="{y}" width="2" height="2"/>
               </g>"#,
            y + 2.,
            start + 3
        )
    };
    let svg = format!(
        r#"<svg viewBox="0 0 100 60">
          <g><line x1="10" x2="90" y1="20" y2="20"/><line x1="10" x2="90" y1="21" y2="21"/>
          <line x1="10" x2="90" y1="22" y2="22"/><line x1="10" x2="90" y1="23" y2="23"/>
          <line x1="10" x2="90" y1="24" y2="24"/></g>
          <g><line x1="10" x2="90" y1="40" y2="40"/><line x1="10" x2="90" y1="41" y2="41"/>
          <line x1="10" x2="90" y1="42" y2="42"/><line x1="10" x2="90" y1="43" y2="43"/>
          <line x1="10" x2="90" y1="44" y2="44"/></g>
          {}
          <g data-ossia-kind="BarLine" data-ossia-bounds="9.8 20 10 44"/>
          <g data-ossia-kind="BarLine" data-ossia-bounds="89.8 20 90 44"/>
        </svg>"#,
        [head("upper", upper, 21., 0), head("lower", lower, 41., 1)].concat()
    );
    let track = [
        0x00, 0xff, 0x58, 0x04, 0x04, 0x02, 0x18, 0x08, 0x00, 0xff, 0x51, 0x03, 0x07, 0xa1, 0x20,
        0x00, 0x90, 0x30, 0x40, 0x00, 0x90, 0x3c, 0x40, 0x00, 0x90, 0x3e, 0x40, 0x00, 0x90, 0x42,
        0x40, 0x00, 0x90, 0x45, 0x40, 0x8f, 0x00, 0x80, 0x30, 0x00, 0x00, 0x80, 0x3c, 0x00, 0x00,
        0x80, 0x3e, 0x00, 0x00, 0x80, 0x42, 0x00, 0x00, 0x80, 0x45, 0x00, 0x00, 0xff, 0x2f, 0x00,
    ];
    let midi = parse_midi(&smf(480, &track)).unwrap();
    let index = build_index(source);
    let layout = build_layout(&index, &[SvgPage { page: 1, svg: &svg }]).unwrap();
    let playback =
        build_playback(&index, &layout, &midi, ContentHash::of(b"hidden-chords")).unwrap();
    assert_eq!(
        playback
            .notes
            .iter()
            .map(|note| note.pitch)
            .collect::<Vec<_>>(),
        vec![48, 60, 62, 66, 69]
    );
}

#[test]
fn absolute_fixture_has_playback_groups_and_system_measure_spans() {
    let source = text("rendered/absolute/score.ly");
    let svg = text("rendered/absolute/score.svg");
    let midi = parse_midi(&fs::read(root().join("rendered/absolute/score.midi")).unwrap()).unwrap();
    let index = build_index(&source);
    let layout = build_layout(&index, &[SvgPage { page: 1, svg: &svg }]).unwrap();
    let playback =
        build_playback(&index, &layout, &midi, ContentHash::of(b"playback-test")).unwrap();
    assert_eq!(playback.source_hash, index.source_hash);
    assert_eq!(playback.notes.len(), 3);
    assert!(playback.duration_seconds > 0.0);
    assert!(playback.groups.is_empty());
    assert!(playback
        .systems
        .iter()
        .any(|span| span.measure_start == 1 && span.end_seconds > span.start_seconds));
}

#[test]
fn chord_noteheads_keep_one_midi_cluster_but_split_audition_groups_by_pitch() {
    let source = "Music = { <c' e'>1 | } \\score { \\new Staff \\Music \\layout {} \\midi {} }";
    let first_start = source.find("c'").unwrap();
    let first_end = first_start + "c'".len();
    let second_start = source.find("e'").unwrap();
    let second_end = second_start + "e'".len();
    let svg = format!(
        r#"<svg viewBox="0 0 100 60">
          <g><line x1="5" x2="90" y1="20" y2="20"/><line x1="5" x2="90" y1="21" y2="21"/>
          <line x1="5" x2="90" y1="22" y2="22"/><line x1="5" x2="90" y1="23" y2="23"/>
          <line x1="5" x2="90" y1="24" y2="24"/></g>
          <g data-ossia-kind="NoteHead" data-ossia-id="a" data-ossia-staff="0"
             data-ossia-bounds="30 21 32 23" data-ossia-source="{first_start}:{first_end}">
            <rect x="30" y="21" width="2" height="2"/>
          </g>
          <g data-ossia-kind="NoteHead" data-ossia-id="b" data-ossia-staff="0"
             data-ossia-bounds="34 18 36 20" data-ossia-source="{second_start}:{second_end}">
            <rect x="34" y="18" width="2" height="2"/>
          </g>
          <g data-ossia-kind="BarLine" data-ossia-bounds="89.8 20 90 24"/>
        </svg>"#
    );
    let track = [
        0x00, 0xff, 0x58, 0x04, 0x04, 0x02, 0x18, 0x08, 0x00, 0xff, 0x51, 0x03, 0x07, 0xa1, 0x20,
        0x00, 0x90, 0x3c, 0x40, 0x00, 0x90, 0x40, 0x40, 0x83, 0x60, 0x80, 0x3c, 0x00, 0x00, 0x80,
        0x40, 0x00, 0x00, 0xff, 0x2f, 0x00,
    ];
    let midi = parse_midi(&smf(480, &track)).unwrap();
    let index = build_index(source);
    let layout = build_layout(&index, &[SvgPage { page: 1, svg: &svg }]).unwrap();
    let playback = build_playback(&index, &layout, &midi, ContentHash::of(b"chord")).unwrap();
    assert_eq!(playback.groups.len(), 2);
    let mut pitches = playback
        .groups
        .iter()
        .map(|group| {
            assert_eq!(group.element_ids.len(), 1);
            assert_eq!(group.pitches.len(), 1);
            group.pitches[0]
        })
        .collect::<Vec<_>>();
    pitches.sort_unstable();
    assert_eq!(pitches, vec![60, 64]);
    assert_ne!(
        playback.groups[0].element_ids,
        playback.groups[1].element_ids
    );
    assert!(playback.groups.iter().all(|group| {
        group.staff_index == Some(0)
            && group.voice.as_deref() == Some("Music")
            && group.start_seconds == Some(0.0)
            && group.duration_seconds == Some(0.5)
    }));
}

#[test]
fn chord_notehead_fragments_still_split_into_single_pitch_groups() {
    let source = "Music = { <c' e' g'>1 | } \\score { \\new Staff \\Music \\layout {} \\midi {} }";
    let fragment = |token: &str| {
        let start = source.find(token).unwrap() + token.len();
        (start, start + 1)
    };
    let heads = ["c'", "e'", "g'"]
        .into_iter()
        .enumerate()
        .map(|(index, token)| {
            let (start, end) = fragment(token);
            format!(
                r#"<g data-ossia-kind="NoteHead" data-ossia-id="n{index}" data-ossia-staff="0"
                     data-ossia-bounds="30 {} 32 {}" data-ossia-source="{start}:{end}">
                     <rect x="30" y="{}" width="2" height="2"/>
                   </g>"#,
                21 + index,
                23 + index,
                21 + index
            )
        })
        .collect::<String>();
    let svg = format!(
        r#"<svg viewBox="0 0 100 60">
          <g><line x1="10" x2="90" y1="20" y2="20"/><line x1="10" x2="90" y1="21" y2="21"/>
          <line x1="10" x2="90" y1="22" y2="22"/><line x1="10" x2="90" y1="23" y2="23"/>
          <line x1="10" x2="90" y1="24" y2="24"/></g>
          {heads}
          <g data-ossia-kind="BarLine" data-ossia-bounds="89.8 20 90 24"/>
        </svg>"#
    );
    let track = [
        0x00, 0xff, 0x58, 0x04, 0x04, 0x02, 0x18, 0x08, 0x00, 0xff, 0x51, 0x03, 0x07, 0xa1, 0x20,
        0x00, 0x90, 0x3c, 0x40, 0x00, 0x90, 0x40, 0x40, 0x00, 0x90, 0x43, 0x40, 0x83, 0x60, 0x80,
        0x3c, 0x00, 0x00, 0x80, 0x40, 0x00, 0x00, 0x80, 0x43, 0x00, 0x00, 0xff, 0x2f, 0x00,
    ];
    let midi = parse_midi(&smf(480, &track)).unwrap();
    let index = build_index(source);
    let layout = build_layout(&index, &[SvgPage { page: 1, svg: &svg }]).unwrap();
    let playback = build_playback(&index, &layout, &midi, ContentHash::of(b"fragments")).unwrap();
    assert_eq!(playback.groups.len(), 3);
    let mut pitches = playback
        .groups
        .iter()
        .map(|group| {
            assert_eq!(group.element_ids.len(), 1);
            group.pitches[0]
        })
        .collect::<Vec<_>>();
    pitches.sort_unstable();
    assert_eq!(pitches, vec![60, 64, 67]);
}

#[test]
fn simultaneous_voice_groups_keep_exact_voice_identity() {
    let source = "Music = { << { c'4 r4 } \\\\ { g,2 } >> | } \\score { \\new Staff \\Music \\layout {} \\midi {} }";
    let first_start = source.find("c'").unwrap();
    let first_end = first_start + "c'".len();
    let second_start = source.find("g,").unwrap();
    let second_end = second_start + "g,".len();
    let svg = format!(
        r#"<svg viewBox="0 0 100 60">
          <g><line x1="5" x2="90" y1="20" y2="20"/><line x1="5" x2="90" y1="21" y2="21"/>
          <line x1="5" x2="90" y1="22" y2="22"/><line x1="5" x2="90" y1="23" y2="23"/>
          <line x1="5" x2="90" y1="24" y2="24"/></g>
          <g data-ossia-kind="NoteHead" data-ossia-id="a" data-ossia-staff="0"
             data-ossia-bounds="30 21 32 23" data-ossia-source="{first_start}:{first_end}">
            <rect x="30" y="21" width="2" height="2"/>
          </g>
          <g data-ossia-kind="NoteHead" data-ossia-id="b" data-ossia-staff="0"
             data-ossia-bounds="30 24 32 26" data-ossia-source="{second_start}:{second_end}">
            <rect x="30" y="24" width="2" height="2"/>
          </g>
          <g data-ossia-kind="BarLine" data-ossia-bounds="89.8 20 90 24"/>
        </svg>"#
    );
    let track = [
        0x00, 0xff, 0x58, 0x04, 0x04, 0x02, 0x18, 0x08, 0x00, 0xff, 0x51, 0x03, 0x07, 0xa1, 0x20,
        0x00, 0x90, 0x3c, 0x40, 0x00, 0x90, 0x2b, 0x40, 0x83, 0x60, 0x80, 0x3c, 0x00, 0x83, 0x60,
        0x80, 0x2b, 0x00, 0x00, 0xff, 0x2f, 0x00,
    ];
    let midi = parse_midi(&smf(480, &track)).unwrap();
    let index = build_index(source);
    let layout = build_layout(&index, &[SvgPage { page: 1, svg: &svg }]).unwrap();
    let playback = build_playback(&index, &layout, &midi, ContentHash::of(b"voices")).unwrap();
    assert_eq!(playback.groups.len(), 2);
    assert!(playback
        .groups
        .iter()
        .all(|group| group.staff_index == Some(0)));
    let voices = playback
        .groups
        .iter()
        .filter_map(|group| group.voice.as_deref())
        .collect::<Vec<_>>();
    assert!(voices.contains(&"Music[1]"));
    assert!(voices.contains(&"Music[2]"));
}

#[test]
fn note_and_rest_groups_include_silent_periods() {
    let source =
        "Music = \\absolute { c'4 r4 d'4 r4 | } \\score { \\new Staff \\Music \\layout {} \\midi {} }";
    let first_note = source.find("c'").unwrap();
    let first_note_end = first_note + "c'".len();
    let first_rest = source.find("r4").unwrap();
    let first_rest_end = first_rest + "r4".len();
    let second_note = source.find("d'").unwrap();
    let second_note_end = second_note + "d'".len();
    let second_rest = source[first_rest_end..].find("r4").unwrap() + first_rest_end;
    let second_rest_end = second_rest + "r4".len();
    let svg = format!(
        r#"<svg viewBox="0 0 100 60">
          <g><line x1="5" x2="90" y1="20" y2="20"/><line x1="5" x2="90" y1="21" y2="21"/>
          <line x1="5" x2="90" y1="22" y2="22"/><line x1="5" x2="90" y1="23" y2="23"/>
          <line x1="5" x2="90" y1="24" y2="24"/></g>
          <g data-ossia-kind="NoteHead" data-ossia-id="a" data-ossia-staff="0"
             data-ossia-bounds="20 21 22 23" data-ossia-source="{first_note}:{first_note_end}">
            <rect x="20" y="21" width="2" height="2"/>
          </g>
          <g data-ossia-kind="Rest" data-ossia-id="b" data-ossia-staff="0"
             data-ossia-bounds="35 20 37 22" data-ossia-source="{first_rest}:{first_rest_end}">
            <rect x="35" y="20" width="2" height="2"/>
          </g>
          <g data-ossia-kind="NoteHead" data-ossia-id="c" data-ossia-staff="0"
             data-ossia-bounds="50 21 52 23" data-ossia-source="{second_note}:{second_note_end}">
            <rect x="50" y="21" width="2" height="2"/>
          </g>
          <g data-ossia-kind="Rest" data-ossia-id="d" data-ossia-staff="0"
             data-ossia-bounds="65 20 67 22" data-ossia-source="{second_rest}:{second_rest_end}">
            <rect x="65" y="20" width="2" height="2"/>
          </g>
          <g data-ossia-kind="BarLine" data-ossia-bounds="89.8 20 90 24"/>
        </svg>"#
    );
    let track = [
        0x00, 0xff, 0x58, 0x04, 0x04, 0x02, 0x18, 0x08, 0x00, 0xff, 0x51, 0x03, 0x07, 0xa1, 0x20,
        0x00, 0x90, 0x3c, 0x40, 0x83, 0x60, 0x80, 0x3c, 0x00, 0x83, 0x60, 0x90, 0x3e, 0x40, 0x83,
        0x60, 0x80, 0x3e, 0x00, 0x00, 0xff, 0x2f, 0x00,
    ];
    let midi = parse_midi(&smf(480, &track)).unwrap();
    let index = build_index(source);
    let layout = build_layout(&index, &[SvgPage { page: 1, svg: &svg }]).unwrap();
    let playback = build_playback(&index, &layout, &midi, ContentHash::of(b"rests")).unwrap();
    assert_eq!(playback.groups.len(), 4);
    let rests = playback
        .groups
        .iter()
        .filter(|group| group.pitches.is_empty())
        .collect::<Vec<_>>();
    assert_eq!(rests.len(), 2);
    assert_eq!(
        rests
            .iter()
            .map(|group| group.start_seconds)
            .collect::<Vec<_>>(),
        vec![Some(0.5), Some(1.5)]
    );
    assert_eq!(
        rests
            .iter()
            .map(|group| group.duration_seconds)
            .collect::<Vec<_>>(),
        vec![Some(0.5), Some(0.5)]
    );
}

#[test]
fn dead_notes_are_silent_groups_and_do_not_reach_the_audio_timeline() {
    let source =
        "Music = { \\deadNotesOn c'4 \\deadNotesOff d'4 | } \\score { \\new Staff \\Music \\layout {} \\midi {} }";
    let first = source.find("c'").unwrap();
    let second = source.find("d'").unwrap();
    let svg = format!(
        r#"<svg viewBox="0 0 100 60">
          <g><line x1="10" x2="90" y1="20" y2="20"/><line x1="10" x2="90" y1="21" y2="21"/>
          <line x1="10" x2="90" y1="22" y2="22"/><line x1="10" x2="90" y1="23" y2="23"/>
          <line x1="10" x2="90" y1="24" y2="24"/></g>
          <g data-ossia-kind="NoteHead" data-ossia-id="dead" data-ossia-staff="0"
             data-ossia-bounds="20 21 22 23" data-ossia-source="{first}:{}">
            <rect x="20" y="21" width="2" height="2"/>
          </g>
          <g data-ossia-kind="NoteHead" data-ossia-id="live" data-ossia-staff="0"
             data-ossia-bounds="30 21 32 23" data-ossia-source="{second}:{}">
            <rect x="30" y="21" width="2" height="2"/>
          </g>
          <g data-ossia-kind="BarLine" data-ossia-bounds="89.8 20 90 24"/>
        </svg>"#,
        first + 2,
        second + 2
    );
    let track = [
        0x00, 0xff, 0x58, 0x04, 0x04, 0x02, 0x18, 0x08, 0x00, 0xff, 0x51, 0x03, 0x07, 0xa1, 0x20,
        0x00, 0x90, 0x3c, 0x40, 0x83, 0x60, 0x80, 0x3c, 0x00, 0x00, 0x90, 0x3e, 0x40, 0x83, 0x60,
        0x80, 0x3e, 0x00, 0x00, 0xff, 0x2f, 0x00,
    ];
    let midi = parse_midi(&smf(480, &track)).unwrap();
    let index = build_index(source);
    let layout = build_layout(&index, &[SvgPage { page: 1, svg: &svg }]).unwrap();
    let playback = build_playback(&index, &layout, &midi, ContentHash::of(b"dead")).unwrap();
    assert_eq!(playback.notes.len(), 1);
    assert_eq!(playback.notes[0].pitch, 62);
    assert_eq!(playback.notes[0].start_seconds, 0.5);
    let dead = playback
        .groups
        .iter()
        .find(|group| group.pitches.is_empty())
        .unwrap();
    assert_eq!(dead.start_seconds, Some(0.0));
    assert_eq!(dead.element_ids.len(), 1);
}

#[test]
fn dead_notes_do_not_silence_the_same_pitch_on_another_channel() {
    let source = r#"
one = { \deadNotesOn c'4 \deadNotesOff }
two = { c'4 }
\score { << \new Staff \one \new Staff \two >> \layout {} \midi {} }
"#;
    let dead = source.find("c'4").unwrap();
    let live = source.rfind("c'4").unwrap();
    let svg = format!(
        r#"<svg viewBox="0 0 100 60">
          <g><line x1="10" x2="90" y1="20" y2="20"/><line x1="10" x2="90" y1="21" y2="21"/>
          <line x1="10" x2="90" y1="22" y2="22"/><line x1="10" x2="90" y1="23" y2="23"/>
          <line x1="10" x2="90" y1="24" y2="24"/></g>
          <g><line x1="10" x2="90" y1="30" y2="30"/><line x1="10" x2="90" y1="31" y2="31"/>
          <line x1="10" x2="90" y1="32" y2="32"/><line x1="10" x2="90" y1="33" y2="33"/>
          <line x1="10" x2="90" y1="34" y2="34"/></g>
          <g data-ossia-kind="NoteHead" data-ossia-id="dead" data-ossia-staff="0"
             data-ossia-bounds="30 21 32 23" data-ossia-source="{dead}:{}">
            <rect x="30" y="21" width="2" height="2"/>
          </g>
          <g data-ossia-kind="NoteHead" data-ossia-id="live" data-ossia-staff="1"
             data-ossia-bounds="30 31 32 33" data-ossia-source="{live}:{}">
            <rect x="30" y="31" width="2" height="2"/>
          </g>
          <g data-ossia-kind="BarLine" data-ossia-bounds="89.8 20 90 34"/>
        </svg>"#,
        dead + 3,
        live + 3
    );
    let track = [
        0x00, 0xff, 0x58, 0x04, 0x04, 0x02, 0x18, 0x08, 0x00, 0xff, 0x51, 0x03, 0x07, 0xa1, 0x20,
        0x00, 0x90, 0x3c, 0x40, 0x00, 0x91, 0x3c, 0x40, 0x83, 0x60, 0x80, 0x3c, 0x00, 0x00, 0x81,
        0x3c, 0x00, 0x00, 0xff, 0x2f, 0x00,
    ];
    let midi = parse_midi(&smf(480, &track)).unwrap();
    let index = build_index(source);
    let layout = build_layout(&index, &[SvgPage { page: 1, svg: &svg }]).unwrap();
    let playback = build_playback(&index, &layout, &midi, ContentHash::of(b"channels")).unwrap();
    assert_eq!(playback.notes.len(), 1);
    assert_eq!(playback.notes[0].pitch, 60);
}

#[test]
fn guitar_fretboard_include_keeps_pitch_events() {
    let source = r#"
\include "predefined-guitar-fretboards.ly"
Music = { <c' e' g'>1 | }
\score { \new Staff \Music \layout {} \midi {} }
"#;
    let index = build_index(source);
    let voice = index.voice("Music").unwrap();
    let resolved = resolve_pitches(source, voice.music_span).unwrap();
    assert_eq!(resolved.unsupported, None);
    assert_eq!(resolved.events.len(), 1);
    assert_eq!(resolved.events[0].pitches, vec![60, 64, 67]);
}

#[test]
fn tied_noteheads_share_the_first_midi_cluster() {
    let source = "Music = { c'4 ~ c'4 | } \\score { \\new Staff \\Music \\layout {} \\midi {} }";
    let first = source.find("c'").unwrap();
    let second = source[first + 2..].find("c'").unwrap() + first + 2;
    let tie_start = source.find('~').unwrap();
    let svg = format!(
        r#"<svg viewBox="0 0 100 60">
          <g><line x1="5" x2="90" y1="20" y2="20"/><line x1="5" x2="90" y1="21" y2="21"/>
          <line x1="5" x2="90" y1="22" y2="22"/><line x1="5" x2="90" y1="23" y2="23"/>
          <line x1="5" x2="90" y1="24" y2="24"/></g>
          <g data-ossia-kind="NoteHead" data-ossia-id="n1" data-ossia-staff="0"
             data-ossia-bounds="30 21 32 23" data-ossia-source="{first}:{}">
            <rect x="30" y="21" width="2" height="2"/>
          </g>
          <g data-ossia-kind="NoteHead" data-ossia-id="n2" data-ossia-staff="0"
             data-ossia-bounds="50 21 52 23" data-ossia-source="{second}:{}">
            <rect x="50" y="21" width="2" height="2"/>
          </g>
          <g data-ossia-kind="Tie" data-ossia-id="tie" data-ossia-owners="n1,n2"
             data-ossia-bounds="32 22 50 24" data-ossia-source="{tie_start}:{}"/>
          <g data-ossia-kind="BarLine" data-ossia-bounds="89.8 20 90 24"/>
        </svg>"#,
        first + 2,
        second + 2,
        tie_start + 1,
    );
    let track = [
        0x00, 0xff, 0x58, 0x04, 0x04, 0x02, 0x18, 0x08, 0x00, 0xff, 0x51, 0x03, 0x07, 0xa1, 0x20,
        0x00, 0x90, 0x3c, 0x40, 0x83, 0x60, 0x80, 0x3c, 0x00, 0x00, 0xff, 0x2f, 0x00,
    ];
    let midi = parse_midi(&smf(480, &track)).unwrap();
    let index = build_index(source);
    let layout = build_layout(&index, &[SvgPage { page: 1, svg: &svg }]).unwrap();
    let playback = build_playback(&index, &layout, &midi, ContentHash::of(b"tie")).unwrap();
    assert_eq!(playback.groups.len(), 1);
    assert_eq!(playback.groups[0].element_ids.len(), 2);
    assert_eq!(playback.groups[0].start_seconds, Some(0.0));
}

#[test]
fn tied_chain_uses_its_earliest_notehead_instead_of_element_order() {
    let source =
        "Music = \\absolute { c'4 ~  c'8 c'8 r2 | } \\score { \\new Staff \\Music \\layout {} \\midi {} }";
    let token = |text: &str| {
        let start = source.find(text).unwrap();
        (start, start + text.len())
    };
    let first = token("c'4");
    let second = source[first.1..].find("c'8").unwrap() + first.1;
    let later = source[second + 3..].find("c'8").unwrap() + second + 3;
    let tie_start = source.find('~').unwrap();
    let head = |id: &str, start: usize, end: usize, x: f64| {
        format!(
            r#"<g data-ossia-kind="NoteHead" data-ossia-id="{id}" data-ossia-staff="0"
                 data-ossia-bounds="{x} 21 {} 23" data-ossia-source="{start}:{end}">
                 <rect x="{x}" y="21" width="2" height="2"/>
               </g>"#,
            x + 2.
        )
    };
    let svg = format!(
        r#"<svg viewBox="0 0 100 60">
          <g><line x1="10" x2="90" y1="20" y2="20"/><line x1="10" x2="90" y1="21" y2="21"/>
          <line x1="10" x2="90" y1="22" y2="22"/><line x1="10" x2="90" y1="23" y2="23"/>
          <line x1="10" x2="90" y1="24" y2="24"/></g>
          {}
          <g data-ossia-kind="Tie" data-ossia-id="tie" data-ossia-owners="z-first,a-cont"
             data-ossia-bounds="22 22 60 24" data-ossia-source="{tie_start}:{}"/>
          <g data-ossia-kind="BarLine" data-ossia-bounds="9.8 20 10 24"/>
          <g data-ossia-kind="BarLine" data-ossia-bounds="89.8 20 90 24"/>
        </svg>"#,
        [
            head("z-first", first.0, first.1, 20.),
            head("a-cont", second, second + 3, 60.),
            head("b-later", later, later + 3, 70.),
        ]
        .concat(),
        tie_start + 1
    );
    let track = [
        0x00, 0xff, 0x58, 0x04, 0x04, 0x02, 0x18, 0x08, 0x00, 0xff, 0x51, 0x03, 0x07, 0xa1, 0x20,
        0x00, 0x90, 0x3c, 0x40, 0x85, 0x50, 0x80, 0x3c, 0x00, 0x00, 0x90, 0x3c, 0x40, 0x81, 0x70,
        0x80, 0x3c, 0x00, 0x00, 0xff, 0x2f, 0x00,
    ];
    let midi = parse_midi(&smf(480, &track)).unwrap();
    let index = build_index(source);
    let layout = build_layout(&index, &[SvgPage { page: 1, svg: &svg }]).unwrap();
    let playback =
        build_playback(&index, &layout, &midi, ContentHash::of(b"tie-earliest")).unwrap();
    assert!(playback
        .groups
        .iter()
        .any(|group| group.start_seconds == Some(0.0) && group.element_ids.len() == 2));
    assert!(playback
        .groups
        .iter()
        .any(|group| group.start_seconds == Some(0.75) && group.element_ids.len() == 1));
    assert!(playback
        .notes
        .iter()
        .any(|note| note.pitch == 60 && note.start_seconds == 0.0));
}

#[test]
fn tablature_playhead_and_fallback_timing_start_at_the_first_note() {
    let source =
        "music = { c'4 d'4 e'4 f'4 | g'4 a'4 b'4 c''4 | } \\score { \\new TabStaff \\music \\layout {} \\midi {} }";
    let head = |id: &str, x: f64| {
        format!(
            r#"<g data-ossia-kind="TabNoteHead" data-ossia-id="{id}" data-ossia-staff="0"
                 data-ossia-bounds="{x} 21 {} 23" data-ossia-source="0:1">
                 <rect x="{x}" y="21" width="2" height="2"/>
               </g>"#,
            x + 2.
        )
    };
    let svg = format!(
        r#"<svg viewBox="0 0 100 60">
          <g><line x1="10" x2="90" y1="20" y2="20"/><line x1="10" x2="90" y1="21" y2="21"/>
          <line x1="10" x2="90" y1="22" y2="22"/><line x1="10" x2="90" y1="23" y2="23"/>
          <line x1="10" x2="90" y1="24" y2="24"/></g>
          {}
          <g data-ossia-kind="BarLine" data-ossia-bounds="9.8 20 10 24"/>
          <g data-ossia-kind="BarLine" data-ossia-bounds="49.8 20 50 24"/>
          <g data-ossia-kind="BarLine" data-ossia-bounds="89.8 20 90 24"/>
        </svg>"#,
        [
            ("n1", 30.),
            ("n2", 34.),
            ("n3", 38.),
            ("n4", 42.),
            ("n5", 52.),
            ("n6", 58.),
            ("n7", 64.),
            ("n8", 70.),
        ]
        .into_iter()
        .map(|(id, x)| head(id, x))
        .collect::<String>()
    );
    let track = [
        0x00, 0xff, 0x58, 0x04, 0x04, 0x02, 0x18, 0x08, 0x00, 0xff, 0x51, 0x03, 0x07, 0xa1, 0x20,
        0x00, 0x90, 0x3c, 0x40, 0x83, 0x60, 0x80, 0x3c, 0x00, 0x00, 0x90, 0x3e, 0x40, 0x83, 0x60,
        0x80, 0x3e, 0x00, 0x00, 0x90, 0x40, 0x40, 0x83, 0x60, 0x80, 0x40, 0x00, 0x00, 0x90, 0x41,
        0x40, 0x83, 0x60, 0x80, 0x41, 0x00, 0x00, 0x90, 0x43, 0x40, 0x83, 0x60, 0x80, 0x43, 0x00,
        0x00, 0x90, 0x45, 0x40, 0x83, 0x60, 0x80, 0x45, 0x00, 0x00, 0x90, 0x47, 0x40, 0x83, 0x60,
        0x80, 0x47, 0x00, 0x00, 0x90, 0x48, 0x40, 0x83, 0x60, 0x80, 0x48, 0x00, 0x00, 0xff, 0x2f,
        0x00,
    ];
    let midi = parse_midi(&smf(480, &track)).unwrap();
    let index = build_index(source);
    let layout = build_layout(&index, &[SvgPage { page: 1, svg: &svg }]).unwrap();
    let playback = build_playback(&index, &layout, &midi, ContentHash::of(b"tab-start")).unwrap();
    assert_eq!(playback.systems[0].x0, 31.0);
    assert_eq!(playback.systems[1].x0, 53.0);
    let first = playback.groups.first().unwrap();
    assert_eq!(first.pitches, [60]);
    assert_eq!(first.start_seconds, Some(0.0));
}

#[test]
fn complete_implicit_final_measure_emits_playhead_groups_and_spans() {
    let source = "music = { \\time 4/4 c'4 d'4 e'4 f'4 | g'4 a'4 b'4 c''4 } \\score { \\new Staff \\music \\layout {} \\midi {} }";
    let notes = [
        ("c1", 20.0, "c'4", 60),
        ("d1", 30.0, "d'4", 62),
        ("e1", 40.0, "e'4", 64),
        ("f1", 45.0, "f'4", 65),
        ("g1", 60.0, "g'4", 67),
        ("a1", 70.0, "a'4", 69),
        ("b1", 80.0, "b'4", 71),
        ("c2", 85.0, "c''4", 72),
    ];
    let heads = notes
        .iter()
        .map(|(id, x, token, _)| {
            let start = source.find(token).unwrap();
            format!(
                r#"<g data-ossia-kind="NoteHead" data-ossia-id="{id}" data-ossia-staff="0"
                     data-ossia-bounds="{x} 21 {} 23"
                     data-ossia-source="{start}:{}">
                     <rect x="{x}" y="21" width="2" height="2"/>
                   </g>"#,
                x + 2.,
                start + token.len()
            )
        })
        .collect::<String>();
    let svg = format!(
        r#"<svg viewBox="0 0 100 60">
          <g><line x1="10" x2="90" y1="20" y2="20"/><line x1="10" x2="90" y1="21" y2="21"/>
          <line x1="10" x2="90" y1="22" y2="22"/><line x1="10" x2="90" y1="23" y2="23"/>
          <line x1="10" x2="90" y1="24" y2="24"/></g>
          {heads}
          <g data-ossia-kind="BarLine" data-ossia-bounds="9.8 20 10 24"/>
          <g data-ossia-kind="BarLine" data-ossia-bounds="49.8 20 50 24"/>
          <g data-ossia-kind="BarLine" data-ossia-bounds="89.8 20 90 24"/>
        </svg>"#
    );
    let mut track = vec![
        0x00, 0xff, 0x58, 0x04, 0x04, 0x02, 0x18, 0x08, // 4/4
        0x00, 0xff, 0x51, 0x03, 0x07, 0xa1, 0x20, // 500000 us
    ];
    for (_, _, _, pitch) in notes {
        track.extend([0x00, 0x90, pitch, 0x40, 0x83, 0x60, 0x80, pitch, 0x00]);
    }
    track.extend([0x00, 0xff, 0x2f, 0x00]);
    let midi = parse_midi(&smf(480, &track)).unwrap();
    let index = build_index(source);
    let layout = build_layout(&index, &[SvgPage { page: 1, svg: &svg }]).unwrap();
    let playback =
        build_playback(&index, &layout, &midi, ContentHash::of(b"implicit-final")).unwrap();
    assert_eq!(layout.columns.len(), 2);
    assert_eq!(playback.groups.len(), 8);
    assert_eq!(playback.systems.len(), 2);
    assert_eq!(playback.groups[0].start_seconds, Some(0.0));
}

#[test]
fn incomplete_measure_index_keeps_playhead_spans_on_the_performance_timeline() {
    let source = r#"music = { \partial 4 c'4 | d'4 e'4 f'4 } \score { \new Staff \music \layout {} \midi {} }"#;
    let head = |id: &str, source_offset: usize, x: f64| {
        format!(
            r#"<g data-ossia-kind="NoteHead" data-ossia-id="{id}" data-ossia-staff="0"
                 data-ossia-bounds="{x} 21 {} 23" data-ossia-source="{source_offset}:{}">
                 <rect x="{x}" y="21" width="2" height="2"/>
               </g>"#,
            x + 2.,
            source_offset + 1
        )
    };
    let svg = format!(
        r#"<svg viewBox="0 0 100 60">
          <g><line x1="10" x2="90" y1="20" y2="20"/><line x1="10" x2="90" y1="21" y2="21"/>
          <line x1="10" x2="90" y1="22" y2="22"/><line x1="10" x2="90" y1="23" y2="23"/>
          <line x1="10" x2="90" y1="24" y2="24"/></g>
          {}
          <g data-ossia-kind="BarLine" data-ossia-bounds="9.8 20 10 24"/>
          <g data-ossia-kind="BarLine" data-ossia-bounds="34.8 20 35 24"/>
          <g data-ossia-kind="BarLine" data-ossia-bounds="59.8 20 60 24"/>
          <g data-ossia-kind="BarLine" data-ossia-bounds="89.8 20 90 24"/>
        </svg>"#,
        [
            head("a", source.find("c'4").unwrap(), 20.),
            head("b", source.find("d'4").unwrap(), 50.),
            head("c", source.find("f'4").unwrap(), 75.),
        ]
        .concat()
    );
    let track = [
        0x00, 0xff, 0x58, 0x04, 0x04, 0x02, 0x18, 0x08, 0x00, 0xff, 0x51, 0x03, 0x07, 0xa1, 0x20,
        0x00, 0x90, 0x3c, 0x40, 0x8f, 0x00, 0x80, 0x3c, 0x00, 0x00, 0x90, 0x3e, 0x40, 0x8f, 0x00,
        0x80, 0x3e, 0x00, 0x00, 0xff, 0x2f, 0x00,
    ];
    let midi = parse_midi(&smf(480, &track)).unwrap();
    let index = build_index(source);
    let layout = build_layout(&index, &[SvgPage { page: 1, svg: &svg }]).unwrap();
    assert!(index.measure_count().is_err());
    assert_eq!(layout.columns.len(), 3);
    assert!(layout
        .columns
        .iter()
        .all(|column| column.measure_start.is_none()));
    let playback = build_playback(&index, &layout, &midi, ContentHash::of(b"partial")).unwrap();
    assert_eq!(playback.systems.len(), layout.columns.len());
    assert_eq!(playback.systems[0].start_seconds, 0.0);
    assert_eq!(playback.systems.last().unwrap().end_seconds, 4.0);
    assert!(playback
        .systems
        .windows(2)
        .all(|pair| pair[0].end_seconds == pair[1].start_seconds));
    assert!(playback
        .systems
        .iter()
        .all(|span| span.end_seconds > span.start_seconds));
    assert!(playback.notes.iter().all(|note| {
        playback.groups.iter().any(|group| {
            group.pitches.contains(&note.pitch)
                && group
                    .start_seconds
                    .is_some_and(|start| (start - note.start_seconds).abs() <= 0.001)
        })
    }));
}

#[test]
fn unmeasured_layout_keeps_midi_onsets_without_a_visual_notehead() {
    let source =
        r#"music = { \partial 4 c'4 | d'4 } \score { \new Staff \music \layout {} \midi {} }"#;
    let start = source.find("c'4").unwrap();
    let svg = format!(
        r#"<svg viewBox="0 0 100 60">
          <g><line x1="10" x2="90" y1="20" y2="20"/><line x1="10" x2="90" y1="21" y2="21"/>
          <line x1="10" x2="90" y1="22" y2="22"/><line x1="10" x2="90" y1="23" y2="23"/>
          <line x1="10" x2="90" y1="24" y2="24"/></g>
          <g data-ossia-kind="NoteHead" data-ossia-id="only" data-ossia-staff="0"
             data-ossia-bounds="20 21 22 23" data-ossia-source="{start}:{}">
            <rect x="20" y="21" width="2" height="2"/>
          </g>
          <g data-ossia-kind="BarLine" data-ossia-bounds="9.8 20 10 24"/>
          <g data-ossia-kind="BarLine" data-ossia-bounds="49.8 20 50 24"/>
          <g data-ossia-kind="BarLine" data-ossia-bounds="89.8 20 90 24"/>
        </svg>"#,
        start + 3
    );
    let track = [
        0x00, 0xff, 0x58, 0x04, 0x04, 0x02, 0x18, 0x08, 0x00, 0xff, 0x51, 0x03, 0x07, 0xa1, 0x20,
        0x00, 0x90, 0x3c, 0x40, 0x87, 0x40, 0x80, 0x3c, 0x00, 0x00, 0x90, 0x3e, 0x40, 0x87, 0x40,
        0x80, 0x3e, 0x00, 0x00, 0xff, 0x2f, 0x00,
    ];
    let midi = parse_midi(&smf(480, &track)).unwrap();
    let index = build_index(source);
    let layout = build_layout(&index, &[SvgPage { page: 1, svg: &svg }]).unwrap();
    let playback = build_playback(&index, &layout, &midi, ContentHash::of(b"barest")).unwrap();
    assert_eq!(playback.notes.len(), 2);
    assert!(playback
        .groups
        .iter()
        .any(|group| group.element_ids.is_empty()));
    assert!(playback.notes.iter().all(|note| {
        playback.groups.iter().any(|group| {
            group.pitches.contains(&note.pitch)
                && group
                    .start_seconds
                    .is_some_and(|start| (start - note.start_seconds).abs() <= 0.001)
        })
    }));
}

#[test]
fn unmeasured_layout_maps_each_visual_onset_to_its_midi_time() {
    let source = r#"music = { \partial 4 c'4 | d'4 e'4 f'4 } \score { \new Staff \music \layout {} \midi {} }"#;
    let tokens = ["c'4", "d'4", "e'4", "f'4"];
    let heads = tokens
        .iter()
        .enumerate()
        .map(|(index, token)| {
            let start = source.find(token).unwrap();
            let x = 20. + index as f64 * 18.;
            format!(
                r#"<g data-ossia-kind="NoteHead" data-ossia-id="{index}" data-ossia-staff="0"
                     data-ossia-bounds="{x} 21 {} 23" data-ossia-source="{start}:{}">
                     <rect x="{x}" y="21" width="2" height="2"/>
                   </g>"#,
                x + 2.,
                start + token.len()
            )
        })
        .collect::<String>();
    let svg = format!(
        r#"<svg viewBox="0 0 100 60">
          <g><line x1="10" x2="90" y1="20" y2="20"/><line x1="10" x2="90" y1="21" y2="21"/>
          <line x1="10" x2="90" y1="22" y2="22"/><line x1="10" x2="90" y1="23" y2="23"/>
          <line x1="10" x2="90" y1="24" y2="24"/></g>
          {heads}
          <g data-ossia-kind="BarLine" data-ossia-bounds="9.8 20 10 24"/>
          <g data-ossia-kind="BarLine" data-ossia-bounds="39.8 20 40 24"/>
          <g data-ossia-kind="BarLine" data-ossia-bounds="69.8 20 70 24"/>
          <g data-ossia-kind="BarLine" data-ossia-bounds="89.8 20 90 24"/>
        </svg>"#
    );
    let mut track = vec![
        0x00, 0xff, 0x58, 0x04, 0x04, 0x02, 0x18, 0x08, 0x00, 0xff, 0x51, 0x03, 0x07, 0xa1, 0x20,
    ];
    for pitch in [60, 62, 64, 65] {
        track.extend([0x00, 0x90, pitch, 0x40, 0x87, 0x40, 0x80, pitch, 0x00]);
    }
    track.extend([0x00, 0xff, 0x2f, 0x00]);
    let midi = parse_midi(&smf(480, &track)).unwrap();
    let index = build_index(source);
    let layout = build_layout(&index, &[SvgPage { page: 1, svg: &svg }]).unwrap();
    let playback = build_playback(&index, &layout, &midi, ContentHash::of(b"onsets")).unwrap();
    for seconds in [0.0, 1.0, 2.0, 3.0] {
        let group = playback
            .groups
            .iter()
            .find(|group| group.start_seconds == Some(seconds) && !group.element_ids.is_empty())
            .unwrap();
        assert_eq!(group.element_ids.len(), 1);
    }
}

#[test]
fn property_commands_do_not_discard_tablature_playhead_spans() {
    let source = "music = { \\omit Staff.Clef \\temporary \\override Staff.Stem.transparent = ##t \\unset Staff.Stem.transparent \\once \\override Staff.BarLine.thick-thickness = #2 c'4 d'4 e'4 f'4 | } \\score { \\new TabStaff \\music \\layout {} \\midi {} }";
    let head = |id: &str, x: f64, token: &str| {
        let start = source.find(token).unwrap();
        format!(
            r#"<g data-ossia-kind="TabNoteHead" data-ossia-id="{id}" data-ossia-staff="0"
                 data-ossia-bounds="{x} 21 {} 23"
                 data-ossia-source="{start}:{}">
                 <rect x="{x}" y="21" width="2" height="2"/>
               </g>"#,
            x + 2.,
            start + token.len()
        )
    };
    let svg = format!(
        r#"<svg viewBox="0 0 100 60">
          <g><line x1="10" x2="90" y1="20" y2="20"/><line x1="10" x2="90" y1="21" y2="21"/>
          <line x1="10" x2="90" y1="22" y2="22"/><line x1="10" x2="90" y1="23" y2="23"/>
          <line x1="10" x2="90" y1="24" y2="24"/></g>
          {}
          <g data-ossia-kind="BarLine" data-ossia-bounds="9.8 20 10 24"/>
          <g data-ossia-kind="BarLine" data-ossia-bounds="89.8 20 90 24"/>
        </svg>"#,
        [
            ("n1", 30., "c'4"),
            ("n2", 40., "d'4"),
            ("n3", 50., "e'4"),
            ("n4", 60., "f'4"),
        ]
        .into_iter()
        .map(|(id, x, token)| head(id, x, token))
        .collect::<String>()
    );
    let track = [
        0x00, 0xff, 0x58, 0x04, 0x04, 0x02, 0x18, 0x08, 0x00, 0xff, 0x51, 0x03, 0x07, 0xa1, 0x20,
        0x00, 0x90, 0x3c, 0x40, 0x83, 0x60, 0x80, 0x3c, 0x00, 0x00, 0x90, 0x3e, 0x40, 0x83, 0x60,
        0x80, 0x3e, 0x00, 0x00, 0x90, 0x40, 0x40, 0x83, 0x60, 0x80, 0x40, 0x00, 0x00, 0x90, 0x41,
        0x40, 0x83, 0x60, 0x80, 0x41, 0x00, 0x00, 0xff, 0x2f, 0x00,
    ];
    let midi = parse_midi(&smf(480, &track)).unwrap();
    let index = build_index(source);
    let layout = build_layout(&index, &[SvgPage { page: 1, svg: &svg }]).unwrap();
    let playback =
        build_playback(&index, &layout, &midi, ContentHash::of(b"tab-properties")).unwrap();
    assert_eq!(playback.systems.len(), 1);
    assert_eq!(playback.groups.len(), 4);
    assert_eq!(playback.groups[0].element_ids.len(), 1);
    assert_eq!(playback.groups[0].pitches, vec![60]);
    assert_eq!(playback.groups[0].start_seconds, Some(0.0));
}

#[test]
fn fretboard_grid_systems_do_not_emit_playhead_spans() {
    let source =
        "music = { c'4 d'4 e'4 f'4 | g'4 a'4 b'4 c''4 | } \\score { \\new TabStaff \\music \\layout {} \\midi {} }";
    let head = |id: &str, x: f64, token: &str| {
        let start = source.find(token).unwrap();
        format!(
            r#"<g data-ossia-kind="TabNoteHead" data-ossia-id="{id}" data-ossia-staff="0"
                 data-ossia-bounds="{x} 21 {} 23"
                 data-ossia-source="{start}:{}">
                 <rect x="{x}" y="21" width="2" height="2"/>
               </g>"#,
            x + 2.,
            start + token.len()
        )
    };
    let svg = format!(
        r#"<svg viewBox="0 0 100 60">
          <g><line x1="30" x2="36" y1="5" y2="5"/><line x1="30" x2="36" y1="6" y2="6"/>
          <line x1="30" x2="36" y1="7" y2="7"/><line x1="30" x2="36" y1="8" y2="8"/>
          <line x1="30" x2="36" y1="9" y2="9"/></g>
          <rect x="29.8" y="5" width=".1" height="4"/>
          <rect x="32.8" y="5" width=".1" height="4"/>
          <rect x="35.8" y="5" width=".1" height="4"/>
          <g><line x1="10" x2="90" y1="20" y2="20"/><line x1="10" x2="90" y1="21" y2="21"/>
          <line x1="10" x2="90" y1="22" y2="22"/><line x1="10" x2="90" y1="23" y2="23"/>
          <line x1="10" x2="90" y1="24" y2="24"/></g>
          {}
          <g data-ossia-kind="BarLine" data-ossia-bounds="9.8 20 10 24"/>
          <g data-ossia-kind="BarLine" data-ossia-bounds="49.8 20 50 24"/>
          <g data-ossia-kind="BarLine" data-ossia-bounds="89.8 20 90 24"/>
        </svg>"#,
        [
            ("n1", 20., "c'4"),
            ("n2", 26., "d'4"),
            ("n3", 32., "e'4"),
            ("n4", 38., "f'4"),
            ("n5", 54., "g'4"),
            ("n6", 60., "a'4"),
            ("n7", 66., "b'4"),
            ("n8", 72., "c''4"),
        ]
        .into_iter()
        .map(|(id, x, token)| head(id, x, token))
        .collect::<String>()
    );
    let track = [
        0x00, 0xff, 0x58, 0x04, 0x04, 0x02, 0x18, 0x08, 0x00, 0xff, 0x51, 0x03, 0x07, 0xa1, 0x20,
        0x00, 0x90, 0x3c, 0x40, 0x83, 0x60, 0x80, 0x3c, 0x00, 0x00, 0x90, 0x3e, 0x40, 0x83, 0x60,
        0x80, 0x3e, 0x00, 0x00, 0x90, 0x40, 0x40, 0x83, 0x60, 0x80, 0x40, 0x00, 0x00, 0x90, 0x41,
        0x40, 0x83, 0x60, 0x80, 0x41, 0x00, 0x00, 0x90, 0x43, 0x40, 0x83, 0x60, 0x80, 0x43, 0x00,
        0x00, 0x90, 0x45, 0x40, 0x83, 0x60, 0x80, 0x45, 0x00, 0x00, 0x90, 0x47, 0x40, 0x83, 0x60,
        0x80, 0x47, 0x00, 0x00, 0x90, 0x48, 0x40, 0x83, 0x60, 0x80, 0x48, 0x00, 0x00, 0xff, 0x2f,
        0x00,
    ];
    let midi = parse_midi(&smf(480, &track)).unwrap();
    let index = build_index(source);
    let layout = build_layout(&index, &[SvgPage { page: 1, svg: &svg }]).unwrap();
    assert_eq!(layout.pages[0].systems.len(), 1, "{layout:?}");
    let playback = build_playback(&index, &layout, &midi, ContentHash::of(b"grid")).unwrap();
    assert_eq!(playback.systems.len(), 2);
    assert!(playback
        .systems
        .iter()
        .all(|span| span.y0 > 15.0 && span.y1 < 30.0));
}

#[test]
fn technical_note_commands_keep_the_measure_index() {
    let source = r#"
music = {
  \deadNotesOn c'4\harmonic d'4\glissando e'4 \revert TabStaff.Tie.stencil f'4 |
  g'1 |
  \deadNotesOff
}
\score { \new TabStaff \music \layout {} \midi {} }
"#;
    let index = build_index(source);
    assert_eq!(index.max_measures, 2, "{:?}", index.reason);
    assert_eq!(index.voices[0].measures.len(), 2);
}

#[test]
fn sequential_variables_map_midi_onto_the_engraved_measure_grid() {
    let source = text("rendered/sequential-variables/score.ly");
    let svg = text("rendered/sequential-variables/score.svg");
    let midi =
        parse_midi(&fs::read(root().join("rendered/sequential-variables/score.midi")).unwrap())
            .unwrap();
    let index = build_index(&source);
    let layout = build_layout(&index, &[SvgPage { page: 1, svg: &svg }]).unwrap();
    let playback = build_playback(&index, &layout, &midi, ContentHash::of(b"sequential")).unwrap();
    assert_eq!(playback.notes.len(), 9);
    assert_eq!(playback.systems.len(), 9);
    assert!(playback.duration_seconds > 8.0);
}
