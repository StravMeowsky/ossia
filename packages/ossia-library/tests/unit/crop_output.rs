use crate::{test_support::Fixture, *};
use std::{fs, path::Path};

fn rendered(id: &str) -> (String, Vec<String>) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures");
    let source = fs::read_to_string(root.join(format!("rendered/{id}/score.ly"))).unwrap();
    let text = fs::read_to_string(root.join("rendered/manifest.json")).unwrap();
    let manifest: serde_json::Value = serde_json::from_str(&text).unwrap();
    let pages = manifest
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["id"] == id)
        .unwrap()["pages"]
        .as_array()
        .unwrap()
        .iter()
        .map(|page| fs::read_to_string(root.join(page.as_str().unwrap())).unwrap())
        .collect();
    (source, pages)
}

fn crop_args(version_id: VersionId, page: Option<u32>) -> OutputArgs {
    OutputArgs {
        source: OutputSource::Version { version_id },
        format: OutputFormat::Svg,
        pages: page.map(|page| vec![page]),
        width: None,
        pdf_password: None,
        png_quality: None,
        png_transparent: None,
        output_path: None,
        output_directory: None,
        crop: Some(OutputCrop::Measures {
            range: MeasureRange { start: 1, end: 3 },
            staff: None,
        }),
        padding_staff_spaces: None,
        audio_range: None,
        include_drawing: None,
        wait_ms: None,
    }
}

#[test]
fn single_file_crop_publishes_to_output_path_without_overwriting() {
    let (source, pages) = rendered("columns-lf");
    let fixture = Fixture::with_svgs(&source, Some(pages));
    let target = fixture._root.path().join("crop.svg");
    let version_id = fixture.doc.head_version_id.unwrap();
    let outcome = fixture
        .library
        .crop_output_file(
            &fixture.engine,
            &crop_args(version_id, Some(1)),
            &target,
            &RunContext::default(),
        )
        .unwrap();
    assert_eq!(outcome.files.len(), 1);
    assert_eq!(Path::new(&outcome.files[0].path), target);
    assert_eq!(Path::new(&outcome.directory), target.parent().unwrap());
    assert!(fs::read_to_string(&target).unwrap().contains("<svg"));
    let original = fs::read(&target).unwrap();
    let error = fixture
        .library
        .crop_output_file(
            &fixture.engine,
            &crop_args(version_id, Some(1)),
            &target,
            &RunContext::default(),
        )
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::Conflict);
    assert_eq!(fs::read(&target).unwrap(), original);
}

#[test]
fn page_filter_allows_single_file_output_for_a_multi_page_range() {
    let (source, pages) = rendered("polyphonic-pages");
    let fixture = Fixture::with_svgs(&source, Some(pages));
    let target = fixture._root.path().join("page-2.svg");
    let version_id = fixture.doc.head_version_id.unwrap();
    fixture
        .library
        .crop_output_file(
            &fixture.engine,
            &crop_args(version_id, Some(2)),
            &target,
            &RunContext::default(),
        )
        .unwrap();
    assert!(target.is_file());

    let unfiltered = fixture._root.path().join("all-pages.svg");
    let error = fixture
        .library
        .crop_output_file(
            &fixture.engine,
            &crop_args(version_id, None),
            &unfiltered,
            &RunContext::default(),
        )
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::InvalidInput, "{}", error.message);
    assert!(!unfiltered.exists());
}
