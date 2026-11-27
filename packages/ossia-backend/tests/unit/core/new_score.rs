use super::*;

fn args(instrument: NewScoreInstrument) -> ManagementCreateScoreArgs {
    ManagementCreateScoreArgs {
        title: "Morning Song".into(),
        composer: Some("A.Composer".into()),
        instrument,
        fifths: 0,
        mode: NewScoreMode::Major,
        time_numerator: 4,
        time_denominator: 4,
        tempo_text: Some("Andante".into()),
        tempo_bpm: Some(72),
        measures: 8,
    }
}

#[test]
fn piano_score_has_two_staff_variables_and_tempo_only_on_the_upper_staff() {
    let source = new_score_source(&args(NewScoreInstrument::Piano)).unwrap();
    assert_eq!(
        source,
        r#"\version "2.26.0"
\language "english"

\header {
  title = "Morning Song"
  composer = "A.Composer"
}

upper = \relative c'' {
  \clef treble \key c \major \time 4/4
  \tempo "Andante" 4 = 72
  R1*8
  \bar "|."
}

lower = \relative c {
  \clef bass \key c \major \time 4/4
  R1*8
  \bar "|."
}

\score {
  \new PianoStaff <<
    \new Staff = "upper" \upper
    \new Staff = "lower" \lower
  >>
  \layout { }
  \midi { }
}
"#
    );
}

#[test]
fn string_instruments_use_one_staff_with_their_clef() {
    for (instrument, variable, start, clef) in [
        (NewScoreInstrument::Violin, "violin", "c''", "treble"),
        (NewScoreInstrument::Viola, "viola", "c'", "alto"),
        (NewScoreInstrument::Cello, "cello", "c", "bass"),
    ] {
        let source = new_score_source(&args(instrument)).unwrap();
        assert!(source.contains(&format!("{variable} = \\relative {start} {{")));
        assert!(source.contains(&format!("\\clef {clef} \\key c \\major")));
        assert!(source.contains(&format!("  \\new Staff \\{variable}\n")));
        assert!(!source.contains("PianoStaff"));
    }
}

#[test]
fn keys_follow_the_circle_of_fifths_in_both_modes() {
    let mut a = args(NewScoreInstrument::Violin);
    for (fifths, mode, key) in [
        (-7, NewScoreMode::Major, "\\key cf \\major"),
        (-3, NewScoreMode::Major, "\\key ef \\major"),
        (2, NewScoreMode::Major, "\\key d \\major"),
        (7, NewScoreMode::Major, "\\key cs \\major"),
        (-7, NewScoreMode::Minor, "\\key af \\minor"),
        (0, NewScoreMode::Minor, "\\key a \\minor"),
        (4, NewScoreMode::Minor, "\\key cs \\minor"),
        (7, NewScoreMode::Minor, "\\key as \\minor"),
    ] {
        a.fifths = fifths;
        a.mode = mode;
        assert!(
            new_score_source(&a).unwrap().contains(key),
            "{fifths} {mode:?}"
        );
    }
}

#[test]
fn empty_measures_use_one_note_value_when_it_exists() {
    let mut a = args(NewScoreInstrument::Violin);
    for ((numerator, denominator), rest, unit) in [
        ((4, 4), "R1*8", "4 = 72"),
        ((2, 2), "R1*8", "2 = 72"),
        ((3, 4), "R2.*8", "4 = 72"),
        ((2, 4), "R2*8", "4 = 72"),
        ((6, 8), "R2.*8", "4. = 72"),
        ((12, 8), "R1.*8", "4. = 72"),
        ((3, 8), "R4.*8", "8 = 72"),
        ((5, 4), "R1*5/4*8", "4 = 72"),
        ((7, 8), "R1*7/8*8", "8 = 72"),
    ] {
        a.time_numerator = numerator;
        a.time_denominator = denominator;
        let source = new_score_source(&a).unwrap();
        assert!(
            source.contains(&format!("  {rest}\n")),
            "{numerator}/{denominator}"
        );
        assert!(source.contains(&format!("\\time {numerator}/{denominator}")));
        assert!(source.contains(unit), "{numerator}/{denominator}: {source}");
    }
    a.time_numerator = 4;
    a.time_denominator = 4;
    a.measures = 1;
    assert!(new_score_source(&a).unwrap().contains("  R1\n"));
}

#[test]
fn optional_fields_are_left_out_when_blank() {
    let mut a = args(NewScoreInstrument::Cello);
    a.composer = Some("  ".into());
    a.tempo_text = None;
    a.tempo_bpm = None;
    let source = new_score_source(&a).unwrap();
    assert!(!source.contains("composer"));
    assert!(!source.contains("\\tempo"));
    a.tempo_text = Some("Allegro".into());
    assert!(new_score_source(&a)
        .unwrap()
        .contains("  \\tempo \"Allegro\"\n"));
    a.tempo_text = None;
    a.tempo_bpm = Some(96);
    assert!(new_score_source(&a).unwrap().contains("  \\tempo 4 = 96\n"));
}

#[test]
fn text_is_quoted_for_lilypond() {
    let mut a = args(NewScoreInstrument::Violin);
    a.title = r#"Song "One" \ Two"#.into();
    let source = new_score_source(&a).unwrap();
    assert!(source.contains(r#"title = "Song \"One\" \\ Two""#));
}

#[test]
fn invalid_requests_are_refused() {
    let mut a = args(NewScoreInstrument::Violin);
    a.title = " ".into();
    assert!(new_score_source(&a).is_err());
    let mut a = args(NewScoreInstrument::Violin);
    a.title = "line\nbreak".into();
    assert!(new_score_source(&a).is_err());
    let mut a = args(NewScoreInstrument::Violin);
    a.fifths = 8;
    assert!(new_score_source(&a).is_err());
    let mut a = args(NewScoreInstrument::Violin);
    a.time_denominator = 3;
    assert!(new_score_source(&a).is_err());
    let mut a = args(NewScoreInstrument::Violin);
    a.time_numerator = 0;
    assert!(new_score_source(&a).is_err());
    let mut a = args(NewScoreInstrument::Violin);
    a.measures = 0;
    assert!(new_score_source(&a).is_err());
    let mut a = args(NewScoreInstrument::Violin);
    a.tempo_bpm = Some(10);
    assert!(new_score_source(&a).is_err());
}
