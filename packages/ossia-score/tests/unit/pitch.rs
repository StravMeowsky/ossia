use super::resolve_pitches;
use crate::SourceMap;

#[test]
fn german_include_distinguishes_b_flat_and_h_natural() {
    let source = "\\include \"deutsch.ly\"\n{ b'4 h'4 }";
    let start = source.find('{').unwrap();
    let span = SourceMap::new(source).span(start, source.len()).unwrap();
    let result = resolve_pitches(source, span).unwrap();
    assert_eq!(result.unsupported, None);
    assert_eq!(
        result
            .events
            .iter()
            .map(|event| event.pitches[0])
            .collect::<Vec<_>>(),
        [70, 71]
    );
}

#[test]
fn transposed_relative_double_stops_keep_both_pitches() {
    let source = r#"\transpose g f \relative c'' { a4 b8 a g fis | <g b>2. | b'4 }"#;
    let span = SourceMap::new(source).span(0, source.len()).unwrap();
    let result = resolve_pitches(source, span).unwrap();
    assert_eq!(result.unsupported, None);
    let chord = result
        .events
        .iter()
        .find(|event| event.members.len() == 2)
        .unwrap();
    assert_eq!(chord.pitches, [65, 69]);
    assert_eq!(
        chord
            .members
            .iter()
            .map(|member| member.midi)
            .collect::<Vec<_>>(),
        [65, 69]
    );
}

#[test]
fn grace_notes_keep_the_relative_pitch_chain() {
    let source = r#"\relative c'' { c4 \appoggiatura d16 e4 \acciaccatura g8 f4 \grace { a16 } g4 \slashedGrace b8 a4 }"#;
    let span = SourceMap::new(source).span(0, source.len()).unwrap();
    let result = resolve_pitches(source, span).unwrap();
    assert_eq!(result.unsupported, None);
    assert_eq!(
        result
            .events
            .iter()
            .map(|event| event.pitches[0])
            .collect::<Vec<_>>(),
        [72, 74, 76, 79, 77, 81, 79, 83, 81]
    );
}

fn first_pitches(source: &str) -> (Option<String>, Vec<i32>) {
    let start = source.rfind('{').unwrap();
    let span = SourceMap::new(source).span(start, source.len()).unwrap();
    let result = resolve_pitches(source, span).unwrap();
    let pitches = result.events.iter().map(|event| event.pitches[0]).collect();
    (result.unsupported, pitches)
}

#[test]
fn language_command_selects_the_same_names_as_the_include() {
    // English: -s and -f are sharp and flat on every step, so "as" is A sharp.
    for directive in ["\\language \"english\"", "\\include \"english.ly\""] {
        let source = format!("{directive}\n{{ as'4 es'4 bf'4 fs'4 ass'4 aff'4 }}");
        assert_eq!(
            first_pitches(&source),
            (None, vec![70, 65, 70, 66, 71, 67]),
            "{directive}"
        );
    }
    let source = "\\language \"deutsch\"\n{ b'4 h'4 es'4 }";
    assert_eq!(first_pitches(source), (None, vec![70, 71, 63]));
    let source = "\\language \"nederlands\"\n{ as'4 es'4 bes'4 fis'4 }";
    assert_eq!(first_pitches(source), (None, vec![68, 63, 70, 66]));
}

#[test]
fn unverified_or_mid_expression_languages_stay_unsupported() {
    let (reason, _) = first_pitches("\\language \"italiano\"\n{ do'4 }");
    assert!(reason.is_some_and(|r| r.contains("note language")));
    let source = "{ c'4 \\language \"english\" bf'4 }";
    let span = SourceMap::new(source).span(0, source.len()).unwrap();
    let result = resolve_pitches(source, span).unwrap();
    assert!(result
        .unsupported
        .is_some_and(|r| r.contains("inside pitch expression")));
}
