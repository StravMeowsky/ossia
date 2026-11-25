use super::{mask_metadata_references, score_transpositions};
use crate::build_index;

#[test]
fn distinct_transpositions_of_the_same_voice_are_ambiguous() {
    let source = r#"music = { c'4 d'4 }
\score { << \new Staff { \transpose c d { \music } } \new Staff { \transpose c e { \music } } >> \midi {} }"#;
    assert_eq!(
        score_transpositions(&build_index(source)).get("music"),
        Some(&None)
    );
}

#[test]
fn metadata_mask_does_not_remove_music_or_conflicting_definitions() {
    let source = r#"global = { \key es \major \time 3/4 }
music = { c'4 }
\score { \new Staff { \global \music } }"#;
    let masked = mask_metadata_references(source);
    assert_eq!(masked.len(), source.len());
    assert!(!masked.contains("\\global"));
    assert!(masked.contains("\\music"));
    let duplicate = format!("{source}\nglobal = {{ c'4 }}");
    assert!(mask_metadata_references(&duplicate).contains("\\global"));
}
