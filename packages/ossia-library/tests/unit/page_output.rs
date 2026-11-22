use crate::{test_support::Fixture, *};

fn svg(page: u32) -> String {
    format!(r#"<svg viewBox="0 0 100 100"><path data-page="{page}" d="M 0 0 L 10 10"/></svg>"#)
}

#[test]
fn page_output_batches_selected_pages_into_one_directory() {
    let f = Fixture::with_svgs("A", Some(vec![svg(1), svg(2)]));
    let parent = f._root.path().join("exports");
    let pages = vec![2, 1];
    let outcome = f
        .library
        .page_output(
            &f.engine,
            &OutputArgs {
                source: OutputSource::Version {
                    version_id: f.doc.head_version_id.unwrap(),
                },
                format: OutputFormat::Svg,
                pages: Some(pages.clone()),
                width: None,
                pdf_password: None,
                png_quality: None,
                png_transparent: None,
                output_path: None,
                output_directory: Some(parent.to_string_lossy().into_owned()),
                crop: None,
                padding_staff_spaces: None,
                audio_range: None,
                include_drawing: None,
                wait_ms: None,
            },
            &parent,
            "pages",
            &RunContext::default(),
        )
        .unwrap();
    assert_eq!(outcome.files.len(), 2);
    assert_eq!(
        outcome
            .files
            .iter()
            .map(|file| file.page)
            .collect::<Vec<_>>(),
        vec![1, 2]
    );
    assert_eq!(outcome.directory, parent.join("pages").to_string_lossy());
    assert!(outcome
        .files
        .iter()
        .all(|file| std::path::Path::new(&file.path).is_file()));
}

#[test]
fn page_batch_uses_the_drawing_snapshot_for_each_page() {
    let f = Fixture::with_svgs("A", Some(vec![svg(1), svg(2)]));
    let request = OutputArgs {
        source: OutputSource::Version {
            version_id: f.doc.head_version_id.unwrap(),
        },
        format: OutputFormat::Svg,
        pages: Some(vec![1, 2]),
        width: None,
        pdf_password: None,
        png_quality: None,
        png_transparent: None,
        include_drawing: Some(true),
        output_path: None,
        output_directory: None,
        crop: None,
        padding_staff_spaces: None,
        audio_range: None,
        wait_ms: None,
    };
    let drawing = Drawing {
        revision: 1,
        strokes: vec![DrawingStroke {
            page: 2,
            width: 0.02,
            color: DrawingColor::Black,
            points: vec![DrawingPoint { x: 0.5, y: 0.5 }],
        }],
    };
    f.library
        .save_drawing(
            f.doc.document_id,
            Drawing {
                revision: 0,
                ..drawing
            },
        )
        .unwrap();
    let result = f
        .library
        .page_output(
            &f.engine,
            &request,
            f._root.path(),
            "marked",
            &RunContext::default(),
        )
        .unwrap();
    let first = std::fs::read_to_string(&result.files[0].path).unwrap();
    let second = std::fs::read_to_string(&result.files[1].path).unwrap();
    assert!(!first.contains("<circle"));
    assert!(second.contains("<circle cx=\"50\" cy=\"50\""));
}
