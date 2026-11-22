use crate::{AppError, SourceMap};
use quick_xml::{events::Event, Reader};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

mod geometry;
use geometry::{numbers, transform, Matrix};
pub use geometry::{path_bbox, BBox};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceAnchor {
    pub href: String,
    pub line: usize,
    pub char_offset: usize,
    pub column: usize,
    pub byte: Option<usize>,
}

fn anchor(href: &str, map: &SourceMap<'_>) -> Option<SourceAnchor> {
    if !href.starts_with("textedit:") {
        return None;
    }
    let mut parts = href.rsplitn(4, ':');
    let column = parts.next()?.parse().ok()?;
    let char_offset = parts.next()?.parse().ok()?;
    let line = parts.next()?.parse().ok()?;
    // Only the compiled source can establish a position. Included standard-library files cannot.
    let file = parts.next()?;
    let source_file = file.rsplit('/').next()?;
    let byte = if source_file == "score.ly" {
        map.anchor_byte(line, char_offset)
    } else {
        None
    };
    Some(SourceAnchor {
        href: href.into(),
        line,
        char_offset,
        column,
        byte,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SvgElement {
    pub kind: String,
    pub bbox: BBox,
    pub anchor: Option<SourceAnchor>,
}

mod objects;
use objects::ObjectOrigin;
pub use objects::SvgObject;

#[derive(Clone)]
struct Frame {
    matrix: Matrix,
    anchor: Option<SourceAnchor>,
    hidden: bool,
    object: Option<ObjectOrigin>,
    graphic: Option<String>,
    link_rect: bool,
}

pub(crate) fn read_svg(
    svg: &str,
    map: &SourceMap<'_>,
) -> Result<(BBox, Vec<SvgElement>, Vec<SvgObject>), AppError> {
    let mut reader = Reader::from_str(svg);
    let mut stack = vec![Frame {
        matrix: Matrix::identity(),
        anchor: None,
        hidden: false,
        object: None,
        graphic: None,
        link_rect: false,
    }];
    let mut elements = Vec::new();
    let mut objects = Vec::new();
    let mut graphic_boxes = HashMap::<String, BBox>::new();
    let mut graphic_text = HashSet::<String>::new();
    let mut link_boxes = Vec::<(usize, BBox)>::new();
    let mut view_box = None;
    let mut seen_root = false;
    let mut closed_root = false;
    loop {
        let event = reader
            .read_event()
            .map_err(|_| AppError::invalid("malformed SVG XML"))?;
        let empty = matches!(event, Event::Empty(_));
        match event {
            Event::Start(e) | Event::Empty(e) => {
                let tag = e.local_name().as_ref().to_string();
                let mut attrs = HashMap::new();
                for a in e.attributes() {
                    let a = a.map_err(|_| AppError::invalid("malformed SVG attribute"))?;
                    let key = a.key.local_name().as_ref().to_string();
                    let value = a
                        .normalized_value(quick_xml::XmlVersion::Implicit1_0)
                        .map_err(|_| AppError::invalid("invalid SVG attribute entity"))?
                        .into_owned();
                    attrs.insert(key, value);
                }
                if !seen_root {
                    if tag != "svg" {
                        return Err(AppError::invalid("SVG root is missing"));
                    }
                    seen_root = true;
                    let n = numbers(
                        attrs
                            .get("viewBox")
                            .ok_or_else(|| AppError::unsupported("SVG viewBox is required"))?,
                    )?;
                    if n.len() != 4 || n[2] <= 0. || n[3] <= 0. {
                        return Err(AppError::invalid("invalid SVG viewBox"));
                    }
                    view_box = Some(BBox::points(&[(n[0], n[1]), (n[0] + n[2], n[1] + n[3])])?);
                } else if closed_root || tag == "svg" {
                    return Err(AppError::unsupported("multiple or nested SVG roots"));
                }
                let mut frame = stack.last().unwrap().clone();
                frame.matrix = frame.matrix.then(transform(
                    attrs.get("transform").map_or("", String::as_str),
                )?);
                frame.hidden |= matches!(tag.as_str(), "defs" | "clipPath" | "symbol")
                    || attrs.get("display").is_some_and(|v| v == "none");
                if let Some(id) = attrs.get("data-ossia-graphic") {
                    frame.graphic = Some(id.clone());
                }
                if tag == "a" {
                    frame.anchor = attrs.get("href").and_then(|s| anchor(s, map));
                }
                if let Some((origin, object)) = objects::read(&attrs, map, frame.matrix)? {
                    frame.object = Some(origin);
                    frame.link_rect = !attrs.contains_key("data-ossia-bounds");
                    objects.extend(object);
                }
                let num = |key: &str, default: f64| -> Result<f64, AppError> {
                    match attrs.get(key) {
                        Some(s) => {
                            let n = s
                                .parse::<f64>()
                                .map_err(|_| AppError::invalid("bad SVG coordinate"))?;
                            if !n.is_finite() {
                                return Err(AppError::invalid("non-finite SVG coordinate"));
                            }
                            Ok(n)
                        }
                        None => Ok(default),
                    }
                };
                let mut link_local = None;
                let bbox = match tag.as_str() {
                    "line" => {
                        let p = frame.matrix.point(num("x1", 0.)?, num("y1", 0.)?);
                        let q = frame.matrix.point(num("x2", 0.)?, num("y2", 0.)?);
                        Some(BBox::points(&[p, q])?)
                    }
                    "rect" => {
                        let x = num("x", 0.)?;
                        let y = num("y", 0.)?;
                        let w = num("width", 0.)?;
                        let h = num("height", 0.)?;
                        if w < 0. || h < 0. {
                            return Err(AppError::invalid("negative SVG dimensions"));
                        }
                        let local = BBox {
                            x0: x,
                            y0: y,
                            x1: x + w,
                            y1: y + h,
                        };
                        if frame.link_rect {
                            link_local = Some(local);
                        }
                        Some(frame.matrix.bbox(local)?)
                    }
                    "polygon" | "polyline" => {
                        let points = numbers(
                            attrs
                                .get("points")
                                .ok_or_else(|| AppError::invalid("SVG polygon has no points"))?,
                        )?;
                        if points.len() < 4 || points.len() % 2 != 0 {
                            return Err(AppError::invalid("invalid SVG polygon points"));
                        }
                        let transformed: Vec<_> = points
                            .as_chunks::<2>()
                            .0
                            .iter()
                            .map(|p| frame.matrix.point(p[0], p[1]))
                            .collect();
                        Some(BBox::points(&transformed)?)
                    }
                    "circle" => {
                        let radius = num("r", 0.)?;
                        if radius < 0. {
                            return Err(AppError::invalid("negative SVG radius"));
                        }
                        Some(frame.matrix.ellipse_bbox(
                            num("cx", 0.)?,
                            num("cy", 0.)?,
                            radius,
                            radius,
                        )?)
                    }
                    "ellipse" => {
                        let rx = num("rx", 0.)?;
                        let ry = num("ry", 0.)?;
                        if rx < 0. || ry < 0. {
                            return Err(AppError::invalid("negative SVG radius"));
                        }
                        Some(
                            frame
                                .matrix
                                .ellipse_bbox(num("cx", 0.)?, num("cy", 0.)?, rx, ry)?,
                        )
                    }
                    "path" => Some(
                        frame.matrix.bbox(path_bbox(
                            attrs
                                .get("d")
                                .ok_or_else(|| AppError::invalid("SVG path has no data"))?,
                        )?)?,
                    ),
                    "text" => {
                        let p = frame.matrix.point(num("x", 0.)?, num("y", 0.)?);
                        Some(BBox::points(&[p])?)
                    }
                    "svg" | "g" | "a" | "style" | "tspan" | "defs" | "clipPath" | "symbol"
                    | "title" | "desc" | "metadata" => None,
                    _ => {
                        return Err(AppError::unsupported(format!(
                            "unsupported SVG element: {tag}"
                        )))
                    }
                };
                if !frame.hidden {
                    if let Some(bbox) = bbox {
                        if let Some(id) = &frame.graphic {
                            if tag == "text" {
                                graphic_text.insert(id.clone());
                            } else {
                                graphic_boxes
                                    .entry(id.clone())
                                    .and_modify(|current| {
                                        current.x0 = current.x0.min(bbox.x0);
                                        current.y0 = current.y0.min(bbox.y0);
                                        current.x1 = current.x1.max(bbox.x1);
                                        current.y1 = current.y1.max(bbox.y1);
                                    })
                                    .or_insert(bbox);
                            }
                        }
                        if let Some(object) = &frame.object {
                            if tag == "rect" {
                                if let Some(local) = link_local {
                                    link_boxes.push((
                                        objects.len(),
                                        frame.matrix.bbox(BBox {
                                            x0: local.x0,
                                            y0: -local.y1,
                                            x1: local.x1,
                                            y1: -local.y0,
                                        })?,
                                    ));
                                }
                                objects.push(SvgObject {
                                    kind: object.kind.clone(),
                                    bbox,
                                    span: object.span,
                                    staff_index: object.staff_index,
                                    hit_only: object.hit_only,
                                    graphic_id: object.graphic_id.clone(),
                                    owners: object.owners.clone(),
                                    related: object.related.clone(),
                                });
                            }
                        } else {
                            elements.push(SvgElement {
                                kind: tag.clone(),
                                bbox,
                                anchor: frame.anchor.clone(),
                            });
                        }
                    }
                }
                if !empty {
                    stack.push(frame);
                } else if tag == "svg" {
                    closed_root = true;
                }
            }
            Event::End(_) => {
                if stack.len() <= 1 {
                    return Err(AppError::invalid("unmatched SVG close"));
                }
                stack.pop();
                if stack.len() == 1 {
                    closed_root = true;
                }
            }
            Event::DocType(_) => return Err(AppError::unsupported("SVG DTD is not supported")),
            Event::Eof => break,
            _ => {}
        }
    }
    if !closed_root || stack.len() != 1 {
        return Err(AppError::invalid("truncated SVG"));
    }
    for (index, flipped) in link_boxes {
        let object = &mut objects[index];
        let bbox = object
            .graphic_id
            .as_ref()
            .and_then(|id| graphic_boxes.get(id));
        let text = object
            .graphic_id
            .as_ref()
            .is_some_and(|id| graphic_text.contains(id));
        object.bbox = if object.hit_only || text {
            flipped
        } else {
            bbox.filter(|bbox| bbox.x1 > bbox.x0 && bbox.y1 > bbox.y0)
                .copied()
                .unwrap_or(flipped)
        };
    }
    Ok((
        view_box.ok_or_else(|| AppError::invalid("empty SVG"))?,
        elements,
        objects,
    ))
}
