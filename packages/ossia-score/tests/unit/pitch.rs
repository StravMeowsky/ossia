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
