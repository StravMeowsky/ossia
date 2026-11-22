use crate::*;

#[cfg(test)]
#[path = "../tests/unit/drawing.rs"]
mod tests;

impl Library {
    pub(crate) fn drawing_for_output(&self, request: &OutputArgs) -> Result<Option<Drawing>> {
        if request.include_drawing != Some(true) {
            return Ok(None);
        }
        if !matches!(
            request.format,
            OutputFormat::Pdf | OutputFormat::Png | OutputFormat::Svg
        ) {
            return Err(AppError::invalid(
                "includeDrawing supports only PDF, PNG and SVG",
            ));
        }
        let document = match request.source {
            OutputSource::Version { version_id } => {
                self.version_snapshot(version_id)?.document_id()
            }
            OutputSource::Job { job_id } => self
                .job_snapshot(job_id)?
                .ok_or_else(not_found)?
                .document_id(),
        };
        self.drawing(document).map(Some)
    }

    /// Read the shared freehand drawing for an active document.
    pub fn drawing(&self, document: DocumentId) -> Result<Drawing> {
        let conn = self.db()?;
        db::document(&conn, document)?;
        let body: Option<String> = conn
            .query_row(
                "SELECT body FROM drawings WHERE document_id=?1",
                [document.to_string()],
                |row| row.get(0),
            )
            .optional()
            .map_err(storage_error)?;
        body.map(db::decode).transpose().map(|drawing| {
            drawing.unwrap_or(Drawing {
                revision: 0,
                strokes: Vec::new(),
            })
        })
    }

    /// Atomically replace a drawing if its expected revision is current.
    pub fn save_drawing(&self, document: DocumentId, mut drawing: Drawing) -> Result<Drawing> {
        if drawing.strokes.len() > 100_000
            || drawing.strokes.iter().any(|stroke| {
                stroke.page == 0
                    || !stroke.width.is_finite()
                    || stroke.width <= 0.0
                    || stroke.width > 1.0
                    || stroke.points.is_empty()
                    || stroke.points.len() > 100_000
                    || stroke.points.iter().any(|point| {
                        !point.x.is_finite()
                            || !point.y.is_finite()
                            || !(0.0..=1.0).contains(&point.x)
                            || !(0.0..=1.0).contains(&point.y)
                    })
            })
        {
            return Err(AppError::invalid("invalid drawing"));
        }
        let mut conn = self.db()?;
        let tx = conn.transaction().map_err(storage_error)?;
        db::document(&tx, document)?;
        let revision: Option<i64> = tx
            .query_row(
                "SELECT revision FROM drawings WHERE document_id=?1",
                [document.to_string()],
                |row| row.get(0),
            )
            .optional()
            .map_err(storage_error)?;
        if u64::try_from(revision.unwrap_or(0)).map_err(storage_error)? != drawing.revision {
            return Err(conflict("drawing has changed; reload before saving"));
        }
        drawing.revision = drawing
            .revision
            .checked_add(1)
            .filter(|revision| *revision <= i64::MAX as u64)
            .ok_or_else(|| conflict("drawing revision overflow"))?;
        tx.execute(
            "INSERT INTO drawings(document_id,revision,body) VALUES (?1,?2,?3) ON CONFLICT(document_id) DO UPDATE SET revision=excluded.revision,body=excluded.body",
            params![document.to_string(), drawing.revision as i64, db::encode(&drawing)?],
        ).map_err(storage_error)?;
        tx.commit().map_err(storage_error)?;
        Ok(drawing)
    }
}
