use super::*;

/// Pitch and musical moment supplied by LilyPond's notehead event.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EngravedNote {
    pub pitch: f64,
    pub numerator: i64,
    pub denominator: u64,
    pub grace_numerator: i64,
    pub grace_denominator: u64,
}

/// Musical start and duration supplied by a rhythmic engraving event.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EngravedTiming {
    pub numerator: i64,
    pub denominator: u64,
    pub duration_numerator: u64,
    pub duration_denominator: u64,
}

/// Exact stencil bounds and source metadata emitted by the engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SvgObject {
    pub note: Option<EngravedNote>,
    pub timing: Option<EngravedTiming>,
    pub kind: String,
    pub bbox: BBox,
    pub span: Option<ossia_types::SourceSpan>,
    pub staff_index: Option<usize>,
    pub hit_only: bool,
    pub graphic_id: Option<String>,
    pub owners: Vec<String>,
    pub related: Vec<String>,
}

#[derive(Clone)]
pub(super) struct ObjectOrigin {
    pub(super) note: Option<EngravedNote>,
    pub(super) timing: Option<EngravedTiming>,
    pub(super) kind: String,
    pub(super) span: Option<ossia_types::SourceSpan>,
    pub(super) staff_index: Option<usize>,
    pub(super) hit_only: bool,
    pub(super) graphic_id: Option<String>,
    pub(super) owners: Vec<String>,
    pub(super) related: Vec<String>,
}

pub(super) fn read(
    attrs: &HashMap<String, String>,
    map: &SourceMap<'_>,
    matrix: Matrix,
) -> Result<Option<(ObjectOrigin, Option<SvgObject>)>, AppError> {
    let Some(kind) = attrs.get("data-ossia-kind") else {
        return Ok(None);
    };
    let span = attrs
        .get("data-ossia-source")
        .map(|value| {
            let (start, end) = value
                .split_once(':')
                .ok_or_else(|| AppError::invalid("invalid element source"))?;
            map.span(
                start
                    .parse()
                    .map_err(|_| AppError::invalid("invalid element offset"))?,
                end.parse()
                    .map_err(|_| AppError::invalid("invalid element offset"))?,
            )
        })
        .transpose()?;
    let timing = attrs
        .get("data-ossia-timing")
        .map(|raw| {
            let fields = raw.split(',').collect::<Vec<_>>();
            let invalid = || AppError::invalid("invalid engraved timing metadata");
            if fields.len() != 4 {
                return Err(invalid());
            }
            let timing = EngravedTiming {
                numerator: fields[0].parse().map_err(|_| invalid())?,
                denominator: fields[1].parse().map_err(|_| invalid())?,
                duration_numerator: fields[2].parse().map_err(|_| invalid())?,
                duration_denominator: fields[3].parse().map_err(|_| invalid())?,
            };
            if timing.denominator == 0 || timing.duration_denominator == 0 {
                return Err(invalid());
            }
            Ok(timing)
        })
        .transpose()?;
    let origin = ObjectOrigin {
        timing,
        note: attrs
            .get("data-ossia-note")
            .map(|raw| {
                let fields = raw.split(',').collect::<Vec<_>>();
                let invalid = || AppError::invalid("invalid engraved note metadata");
                if fields.len() != 5 {
                    return Err(invalid());
                }
                let note = EngravedNote {
                    pitch: fields[0].parse().map_err(|_| invalid())?,
                    numerator: fields[1].parse().map_err(|_| invalid())?,
                    denominator: fields[2].parse().map_err(|_| invalid())?,
                    grace_numerator: fields[3].parse().map_err(|_| invalid())?,
                    grace_denominator: fields[4].parse().map_err(|_| invalid())?,
                };
                if !note.pitch.is_finite() || note.denominator == 0 || note.grace_denominator == 0 {
                    return Err(invalid());
                }
                Ok(note)
            })
            .transpose()?,
        kind: kind.clone(),
        span,
        staff_index: attrs.get("data-ossia-staff").and_then(|v| v.parse().ok()),
        hit_only: attrs.get("data-ossia-hit").is_some_and(|v| v == "true"),
        graphic_id: attrs.get("data-ossia-id").cloned(),
        related: attrs
            .get("data-ossia-related")
            .map(|v| {
                v.split(',')
                    .filter(|s| !s.is_empty())
                    .map(str::to_owned)
                    .collect()
            })
            .unwrap_or_default(),
        owners: attrs
            .get("data-ossia-owners")
            .map(|v| {
                v.split(',')
                    .filter(|s| !s.is_empty())
                    .map(str::to_owned)
                    .collect()
            })
            .unwrap_or_default(),
    };
    let object = attrs
        .get("data-ossia-bounds")
        .map(|value| {
            let coords = numbers(value)?;
            if coords.len() != 4 {
                return Err(AppError::invalid("invalid element bounds"));
            }
            Ok(SvgObject {
                note: origin.note,
                timing: origin.timing,
                kind: kind.clone(),
                bbox: matrix.bbox(BBox::points(&[
                    (coords[0], coords[1]),
                    (coords[2], coords[3]),
                ])?)?,
                span,
                staff_index: origin.staff_index,
                hit_only: origin.hit_only,
                graphic_id: origin.graphic_id.clone(),
                owners: origin.owners.clone(),
                related: origin.related.clone(),
            })
        })
        .transpose()?;
    Ok(Some((origin, object)))
}
