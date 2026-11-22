use crate::*;
use regex::Regex;
use std::sync::OnceLock;

#[cfg(test)]
#[path = "../tests/unit/drawing_output.rs"]
mod tests;

/// Add drawing strokes in the original SVG coordinate space before rasterizing or cropping.
pub(crate) fn overlay_svg(svg: &[u8], page: u32, drawing: Option<&Drawing>) -> Result<Vec<u8>> {
    let Some(drawing) = drawing else {
        return Ok(svg.to_vec());
    };
    let strokes: Vec<_> = drawing
        .strokes
        .iter()
        .filter(|stroke| stroke.page == page)
        .collect();
    if strokes.is_empty() {
        return Ok(svg.to_vec());
    }
    let source = std::str::from_utf8(svg).map_err(storage_error)?;
    let root = source
        .find("<svg")
        .ok_or_else(|| AppError::invalid("SVG root is missing"))?;
    let end = source[root..]
        .find('>')
        .map(|offset| root + offset)
        .ok_or_else(|| AppError::invalid("SVG root is incomplete"))?;
    static VIEWBOX: OnceLock<Regex> = OnceLock::new();
    let viewbox = VIEWBOX
        .get_or_init(|| Regex::new(r#"\bviewBox\s*=\s*["']([^"']+)["']"#).unwrap())
        .captures(&source[root..end])
        .and_then(|captures| captures.get(1))
        .ok_or_else(|| AppError::invalid("SVG viewBox is unavailable"))?
        .as_str();
    let values = viewbox
        .split(|c: char| c.is_whitespace() || c == ',')
        .filter(|part| !part.is_empty())
        .map(str::parse::<f64>)
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|_| AppError::invalid("invalid SVG viewBox"))?;
    if values.len() != 4
        || values.iter().any(|v| !v.is_finite())
        || values[2] <= 0.0
        || values[3] <= 0.0
    {
        return Err(AppError::invalid("invalid SVG viewBox"));
    }
    let [x, y, width, height]: [f64; 4] = values
        .try_into()
        .map_err(|_| AppError::invalid("invalid SVG viewBox"))?;
    let close = source
        .rfind("</svg>")
        .ok_or_else(|| AppError::invalid("SVG root is incomplete"))?;
    let mut overlay = String::from(
        "<g fill=\"none\" stroke=\"black\" stroke-linecap=\"round\" stroke-linejoin=\"round\">",
    );
    for stroke in strokes {
        let [red, green, blue] = stroke.color.rgb();
        let color = format!("#{red:02x}{green:02x}{blue:02x}");
        let stroke_width = stroke.width * width;
        let point = |p: &DrawingPoint| (x + p.x * width, y + p.y * height);
        if stroke.points.len() == 1 {
            let (px, py) = point(&stroke.points[0]);
            overlay.push_str(&format!(
                "<circle cx=\"{px}\" cy=\"{py}\" r=\"{}\" fill=\"{color}\" stroke=\"none\"/>",
                stroke_width / 2.0
            ));
        } else {
            let points = stroke
                .points
                .iter()
                .map(|p| {
                    let (px, py) = point(p);
                    format!("{px},{py}")
                })
                .collect::<Vec<_>>()
                .join(" ");
            overlay.push_str(&format!(
                "<polyline points=\"{points}\" stroke-width=\"{stroke_width}\" stroke=\"{color}\"/>"
            ));
        }
    }
    overlay.push_str("</g>");
    Ok(format!("{}{overlay}{}", &source[..close], &source[close..]).into_bytes())
}
