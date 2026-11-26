use super::*;

#[test]
fn output_groups_require_contiguous_numeric_pages_and_unique_exports() {
    let dir = tempfile::tempdir().unwrap();
    for n in [10, 3, 1, 7, 4, 8, 2, 5, 9, 6] {
        fs::write(dir.path().join(format!("score-{n}.svg")), "<svg/>").unwrap();
    }
    assert_eq!(
        collect(dir.path(), RenderFormat::Svg)
            .unwrap()
            .iter()
            .map(|x| x.0)
            .collect::<Vec<_>>(),
        (1..=10).collect::<Vec<_>>()
    );
    fs::remove_file(dir.path().join("score-5.svg")).unwrap();
    assert!(collect(dir.path(), RenderFormat::Svg).is_err());
    fs::write(dir.path().join("score.pdf"), "%PDF-1.4").unwrap();
    fs::write(dir.path().join("score-other.pdf"), "%PDF-1.4").unwrap();
    assert!(collect(dir.path(), RenderFormat::Pdf).is_err());
    fs::write(dir.path().join("another.svg"), "<svg/>").unwrap();
    assert!(collect(dir.path(), RenderFormat::Svg).is_err());
}

#[test]
fn consecutive_score_midi_files_follow_file_order() {
    let dir = tempfile::tempdir().unwrap();
    for name in ["score-2.midi", "score.midi", "score-1.midi"] {
        fs::write(dir.path().join(name), "MThd").unwrap();
    }
    let files = collect(dir.path(), RenderFormat::Midi).unwrap();
    let names: Vec<_> = files
        .iter()
        .map(|(_, path)| path.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    assert_eq!(names, ["score.midi", "score-1.midi", "score-2.midi"]);
    fs::remove_file(dir.path().join("score-1.midi")).unwrap();
    assert!(collect(dir.path(), RenderFormat::Midi).is_err());
}

#[test]
fn score_lengths_are_read_from_the_engine_log() {
    let log = "Parsing...\nossia-score-length:7/4\nossia-score-length:1/1\nInterpreting music...";
    assert_eq!(score_lengths(log).unwrap(), vec![(7, 4), (1, 1)]);
    assert!(score_lengths("ossia-score-length:x").is_err());
}
