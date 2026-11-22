use super::*;
use crate::ErrorCode;
use std::io::Write;
use zip::{write::SimpleFileOptions, ZipWriter};

const MUSICXML: &[u8] = include_bytes!("../../../../tests/fixtures/inputs/xml-clean.musicxml");

#[test]
fn extracts_the_declared_rootfile() {
    assert_eq!(
        extract(include_bytes!(
            "../../../../tests/fixtures/inputs/xml-clean.mxl"
        ))
        .unwrap(),
        MUSICXML
    );
}

#[test]
fn rejects_damaged_missing_and_ambiguous_archives() {
    assert_eq!(
        extract(b"not a zip").unwrap_err().code,
        ErrorCode::InvalidInput
    );

    let missing = archive(&[("score.musicxml", MUSICXML)]);
    assert_eq!(extract(&missing).unwrap_err().code, ErrorCode::InvalidInput);

    let ambiguous = archive(&[
        (
            CONTAINER_PATH,
            br#"<container><rootfiles>
                <rootfile full-path="one.musicxml"/>
                <rootfile full-path="two.musicxml"/>
            </rootfiles></container>"#,
        ),
        ("one.musicxml", MUSICXML),
        ("two.musicxml", MUSICXML),
    ]);
    let error = extract(&ambiguous).unwrap_err();
    assert_eq!(error.code, ErrorCode::InvalidInput);
    assert!(error.message.contains("multiple root files"));
}

fn archive(files: &[(&str, &[u8])]) -> Vec<u8> {
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    for (name, data) in files {
        writer
            .start_file(*name, SimpleFileOptions::default())
            .unwrap();
        writer.write_all(data).unwrap();
    }
    writer.finish().unwrap().into_inner()
}
