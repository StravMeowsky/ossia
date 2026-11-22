use super::*;

#[test]
fn svg_strokes_follow_page_viewbox_and_filter_other_pages() {
    let drawing = Drawing {
        revision: 1,
        strokes: vec![
            DrawingStroke {
                page: 1,
                width: 0.01,
                color: DrawingColor::Blue,
                points: vec![
                    DrawingPoint { x: 0.25, y: 0.5 },
                    DrawingPoint { x: 0.75, y: 0.75 },
                ],
            },
            DrawingStroke {
                page: 2,
                width: 0.02,
                color: DrawingColor::Black,
                points: vec![DrawingPoint { x: 0.5, y: 0.5 }],
            },
        ],
    };
    let svg = br#"<svg viewBox="10 20 200 400"><path/></svg>"#;
    let first = String::from_utf8(overlay_svg(svg, 1, Some(&drawing)).unwrap()).unwrap();
    assert!(first.contains("points=\"60,220 160,320\""));
    assert!(first.contains("stroke-width=\"2\""));
    assert!(first.contains("stroke=\"#007aff\""));
    assert!(!first.contains("<circle"));
    let second = String::from_utf8(overlay_svg(svg, 2, Some(&drawing)).unwrap()).unwrap();
    assert!(second.contains("cx=\"110\" cy=\"220\" r=\"2\""));
    assert_eq!(overlay_svg(svg, 3, Some(&drawing)).unwrap(), svg);
}
