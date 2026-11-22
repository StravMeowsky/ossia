use super::build_index;

#[test]
fn common_engraving_commands_preserve_measure_index() {
    let source =
        r#"notes = { \tupletSpan 8 \stemUp c'4\trill d' e' f' | } \score { \new Staff \notes }"#;
    let index = build_index(source);
    assert_eq!(index.measure_count(), Ok(1));
}

#[test]
fn parallel_voices_are_not_concatenated_into_rendered_measures() {
    let source = r#"upper = { c'1 | d'1 | } lower = { e'1 | f'1 | } \score { \new Staff << \new Voice \upper \new Voice \lower >> }"#;
    let index = build_index(source);
    assert_eq!(index.measure_count(), Ok(2));
    assert!(index.rendered_measure_sequence().is_empty());
}

#[test]
fn repeated_references_cannot_silently_lose_measures() {
    let source = r#"notes = { c'1 | } \score { \new Staff { \notes \notes } }"#;
    let index = build_index(source);
    assert_eq!(index.measure_count(), Ok(2));
}

#[test]
fn repeated_references_in_parallel_music_do_not_claim_a_count() {
    let source = r#"a = { c'1 | } b = { e'1 | } \score { \new Staff << { \a \a } { \b \b } >> }"#;
    assert!(build_index(source).measure_count().is_err());
}

#[test]
fn repeats_and_multiple_scores_do_not_return_guessed_counts() {
    let repeat =
        r#"music = { \partial 16 c'16 | \repeat volta 2 { c'1 | } } \score { \new Staff \music }"#;
    let multiple =
        r#"music = { c'1 | } \score { \new Staff \music } \score { \new Staff \music \midi {} }"#;
    assert!(build_index(repeat).measure_count().is_err());
    assert!(build_index(multiple).measure_count().is_err());
}

#[test]
fn a_midi_only_score_block_does_not_override_the_engraved_staff_map() {
    let source = r#"
notes = { c'1 | }
\score { \new Staff \notes \layout {} }
\score { \new Staff \notes \midi {} }
"#;
    let index = build_index(source);
    assert_eq!(index.staves.len(), 1);
    assert_eq!(index.reason, None);
    assert_eq!(index.measure_count(), Ok(1));
}

#[test]
fn staff_contexts_in_referenced_variables_are_discovered() {
    let source = r#"
upper = { c'1 | }
lower = { g1 | }
notes = \context Staff << \context Voice = "upper" \upper \context Voice = "lower" \lower >>
staff = \context Staff << \notes >>
\score { \staff }
"#;
    let index = build_index(source);
    assert_eq!(index.staves.len(), 1);
    assert_eq!(index.staves[0].voices, ["upper", "lower"]);
}

#[test]
fn cyclic_staff_references_are_reported() {
    let source = r#"one = { \two } two = { \one } \score { \one }"#;
    let index = build_index(source);
    assert!(index
        .reason
        .as_deref()
        .is_some_and(|reason| reason.contains("cyclic")));
}

#[test]
fn sequential_variables_get_engraved_measure_numbers() {
    let source = r#"
intro = { \time 4/4 c'1 | d'1 | }
main = { e'1 | f'1 | g'1 | }
one = { a'1 | b'1 | }
two = { c''1 | d''1 | }
\score {
  \new Staff {
    \intro
    \repeat volta 2 { \main }
    \alternative { \one \two }
  }
}
"#;
    let index = build_index(source);
    let sequence = index.rendered_measure_sequence();
    let labels = sequence
        .iter()
        .map(|measure| {
            let item = &measure.measures[0];
            (item.voice.as_str(), item.measure)
        })
        .collect::<Vec<_>>();
    assert_eq!(
        labels,
        vec![
            ("intro", 1),
            ("intro", 2),
            ("main", 1),
            ("main", 2),
            ("main", 3),
            ("one", 1),
            ("one", 2),
            ("two", 1),
            ("two", 2),
        ]
    );
    assert_eq!(index.rendered_measure("intro", 2, None), Some(2));
    assert_eq!(index.rendered_measure("main", 2, None), Some(4));
    assert_eq!(index.rendered_measure("two", 1, None), Some(8));
}
