use super::*;

#[test]
fn quantized_png_preserves_palette_alpha() {
    let mut pixmap = tiny_skia::Pixmap::new(2, 1).unwrap();
    pixmap.fill(tiny_skia::Color::from_rgba8(0, 0, 0, 0));
    pixmap.pixels_mut()[0] = tiny_skia::PremultipliedColorU8::from_rgba(0, 0, 0, 0).unwrap();
    pixmap.pixels_mut()[1] = tiny_skia::PremultipliedColorU8::from_rgba(0, 0, 0, 255).unwrap();
    let bytes = encode_quantized_png(&pixmap, 60).unwrap();
    assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n");
    assert_eq!(bytes[25], 3);
    assert!(bytes.windows(4).any(|window| window == b"tRNS"));
}

#[test]
fn lilypond_whiteout_color_rasterizes_without_a_black_block() {
    let original = "\\score { { c'1 } }\n";
    let derived = crate::source::derive(original, RenderFormat::Svg, true).unwrap();
    let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="40" height="40" viewBox="0 0 40 40"><g color="rgba(100%, 100%, 100%, 100.0000%)"><rect width="40" height="40" fill="currentColor"/></g></svg>"#;
    let normalized = crate::source::normalize_svg(svg, &derived, original).unwrap();
    let tree = usvg::Tree::from_str(&normalized, &usvg::Options::default()).unwrap();
    let mut pixmap = tiny_skia::Pixmap::new(40, 40).unwrap();
    pixmap.fill(tiny_skia::Color::BLACK);
    resvg::render(
        &tree,
        tiny_skia::Transform::identity(),
        &mut pixmap.as_mut(),
    );
    let pixel = pixmap.pixel(20, 20).unwrap();
    assert_eq!(
        (pixel.red(), pixel.green(), pixel.blue()),
        (255, 255, 255),
        "LilyPond whiteout must not rasterize as black"
    );
}
