use super::*;
use crate::test_support::Fixture;

#[test]
fn drawings_without_ink_color_default_to_black() {
    let drawing: Drawing = serde_json::from_str(
        r#"{"revision":1,"strokes":[{"page":1,"width":0.01,"points":[{"x":0.5,"y":0.5}]}]}"#,
    )
    .unwrap();
    assert_eq!(drawing.strokes[0].color, DrawingColor::Black);
}

#[test]
fn drawing_is_persisted_and_rejects_stale_writes() {
    let fixture = Fixture::new();
    let id = fixture.doc.document_id;
    let empty = fixture.library.drawing(id).unwrap();
    assert_eq!(empty.revision, 0);
    let drawing = Drawing {
        revision: 0,
        strokes: vec![DrawingStroke {
            page: 1,
            width: 0.01,
            color: DrawingColor::Black,
            points: vec![DrawingPoint { x: 0.25, y: 0.75 }],
        }],
    };
    let saved = fixture.library.save_drawing(id, drawing.clone()).unwrap();
    assert_eq!(saved.revision, 1);
    assert_eq!(fixture.library.drawing(id).unwrap(), saved);
    assert_eq!(
        fixture.library.save_drawing(id, drawing).unwrap_err().code,
        ErrorCode::Conflict
    );
    assert_eq!(fixture.library.drawing(id).unwrap(), saved);
}

#[test]
fn drawing_rejects_invalid_coordinates_and_pages() {
    let fixture = Fixture::new();
    let id = fixture.doc.document_id;
    let mut drawing = Drawing {
        revision: 0,
        strokes: vec![DrawingStroke {
            page: 0,
            width: 0.01,
            color: DrawingColor::Black,
            points: vec![DrawingPoint { x: 0.0, y: 0.0 }],
        }],
    };
    assert!(fixture.library.save_drawing(id, drawing.clone()).is_err());
    drawing.strokes[0].page = 1;
    drawing.strokes[0].points[0].x = 1.1;
    assert!(fixture.library.save_drawing(id, drawing).is_err());
    assert_eq!(fixture.library.drawing(id).unwrap().revision, 0);
}
