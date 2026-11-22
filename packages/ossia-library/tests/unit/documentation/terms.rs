use super::*;

#[test]
fn expands_existing_and_guitar_terms() {
    assert_eq!(expand("八度"), "octave");
    assert_eq!(expand("六线谱"), "tablature");
    assert_eq!(expand("吉他"), "guitar");
    assert_eq!(expand("指法"), "fingering");
    assert_eq!(expand("变调夹"), "capo");
}

#[test]
fn separates_adjacent_chinese_terms() {
    assert_eq!(expand("吉他六线谱"), "guitar tablature");
    assert_eq!(expand("相对音高 六线谱"), "relative tablature");
}
