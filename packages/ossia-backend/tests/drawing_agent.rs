mod common;

use common::Fixture;
use ossia_backend::*;

fn stroke(page: u32, x: f64) -> DrawingStroke {
    DrawingStroke {
        page,
        width: 0.01,
        color: DrawingColor::Blue,
        points: vec![DrawingPoint { x, y: 0.5 }],
    }
}

fn read(document_id: DocumentId) -> ReadDrawingArgs {
    ReadDrawingArgs {
        document_id,
        page: None,
        offset: None,
        limit: None,
        expected_revision: None,
        output_path: None,
    }
}

#[tokio::test]
async fn agent_drawing_paging_edits_and_file_migration() {
    let src = Fixture::new(false).await;
    let dst = Fixture::new(false).await;
    let id = src.doc.document_id;
    let r = &src.runtime;
    assert_eq!(r.read_drawing(read(id)).unwrap().stroke_count, 0);
    let saved = r
        .write_drawing(WriteDrawingArgs {
            document_id: id,
            expected_revision: 0,
            change: DrawingChange::Replace {
                strokes: vec![stroke(1, 0.1), stroke(2, 0.2), stroke(1, 0.3)],
            },
        })
        .unwrap();
    assert_eq!((saved.revision, saved.stroke_count), (1, 3));
    let mut args = read(id);
    args.page = Some(1);
    args.limit = Some(1);
    let first = r.read_drawing(args.clone()).unwrap();
    assert_eq!(first.strokes[0].index, 0);
    assert_eq!(first.next_offset, Some(2));
    args.offset = first.next_offset;
    assert!(r.read_drawing(args.clone()).is_err());
    args.expected_revision = Some(first.revision);
    assert_eq!(r.read_drawing(args.clone()).unwrap().strokes[0].index, 2);
    let edit = |revision, edits| WriteDrawingArgs {
        document_id: id,
        expected_revision: revision,
        change: DrawingChange::Edit { edits },
    };
    let before = r.library.drawing(id).unwrap();
    assert!(r
        .write_drawing(edit(
            1,
            vec![
                DrawingEdit::Delete { index: 0 },
                DrawingEdit::Delete { index: 0 }
            ]
        ))
        .is_err());
    assert!(r
        .write_drawing(edit(1, vec![DrawingEdit::Delete { index: 9 }]))
        .is_err());
    assert!(r
        .write_drawing(edit(
            1,
            vec![DrawingEdit::Add {
                stroke: stroke(0, 0.1)
            }]
        ))
        .is_err());
    assert_eq!(r.library.drawing(id).unwrap(), before);
    let result = r
        .write_drawing(edit(
            1,
            vec![
                DrawingEdit::Delete { index: 0 },
                DrawingEdit::Replace {
                    index: 2,
                    stroke: stroke(3, 0.7),
                },
                DrawingEdit::Add {
                    stroke: stroke(4, 0.8),
                },
            ],
        ))
        .unwrap();
    assert_eq!((result.revision, result.stroke_count), (2, 3));
    assert_eq!(r.read_drawing(args).unwrap_err().code, ErrorCode::Conflict);
    assert_eq!(
        r.write_drawing(edit(1, vec![])).unwrap_err().code,
        ErrorCode::Conflict
    );
    let path = src
        .root
        .path()
        .join("drawing.json")
        .to_str()
        .unwrap()
        .to_string();
    let mut export = read(id);
    export.output_path = Some(path.clone());
    assert_eq!(r.read_drawing(export.clone()).unwrap().stroke_count, 3);
    assert!(r.read_drawing(export).is_err());
    let imported = dst
        .runtime
        .write_drawing(WriteDrawingArgs {
            document_id: dst.doc.document_id,
            expected_revision: 0,
            change: DrawingChange::Import { input_path: path },
        })
        .unwrap();
    assert_eq!(imported.revision, 1);
    assert_eq!(
        dst.runtime
            .library
            .drawing(dst.doc.document_id)
            .unwrap()
            .strokes,
        r.library.drawing(id).unwrap().strokes
    );
    let clear = dst
        .runtime
        .write_drawing(WriteDrawingArgs {
            document_id: dst.doc.document_id,
            expected_revision: 1,
            change: DrawingChange::Replace { strokes: vec![] },
        })
        .unwrap();
    assert_eq!((clear.revision, clear.stroke_count), (2, 0));
}

#[tokio::test]
async fn drawing_file_limits_and_oversized_stroke_are_explicit() {
    let f = Fixture::new(false).await;
    let id = f.doc.document_id;
    let bad = f.root.path().join("bad.json");
    std::fs::write(&bad, b"not json").unwrap();
    let import = |path: String| WriteDrawingArgs {
        document_id: id,
        expected_revision: 0,
        change: DrawingChange::Import { input_path: path },
    };
    assert!(f
        .runtime
        .write_drawing(import(bad.to_string_lossy().into()))
        .is_err());
    let huge = f.root.path().join("huge.json");
    let file = std::fs::File::create(&huge).unwrap();
    file.set_len(32 * 1024 * 1024 + 1).unwrap();
    assert!(f
        .runtime
        .write_drawing(import(huge.to_string_lossy().into()))
        .is_err());
    assert_eq!(f.runtime.library.drawing(id).unwrap().revision, 0);
    let mut long = stroke(1, 0.1);
    long.points = vec![
        DrawingPoint {
            x: 0.123456,
            y: 0.654321
        };
        5000
    ];
    f.runtime
        .write_drawing(WriteDrawingArgs {
            document_id: id,
            expected_revision: 0,
            change: DrawingChange::Replace {
                strokes: vec![long],
            },
        })
        .unwrap();
    assert!(f
        .runtime
        .read_drawing(read(id))
        .unwrap_err()
        .message
        .contains("export"));
}
