use super::*;
use std::io::{Read, Write};

const MAX_DRAWING_FILE: usize = 32 * 1024 * 1024;
const MAX_PAGE_BYTES: usize = 64 * 1024;
const MAX_PAGE_STROKES: usize = 100;

fn drawing_conflict() -> AppError {
    AppError::new(
        ErrorCode::Conflict,
        "drawing has changed; reload before saving or paging",
    )
}

fn absolute_path(path: &str) -> Result<&Path> {
    let path = Path::new(path);
    if !path.is_absolute() {
        return Err(invalid("drawing file path must be absolute"));
    }
    Ok(path)
}

impl Runtime {
    /// Read a bounded drawing page or export a complete snapshot.
    pub fn read_drawing(&self, args: ReadDrawingArgs) -> Result<ReadDrawingOutcome> {
        if args.page == Some(0) {
            return Err(invalid("page must be one-based"));
        }
        if args.offset.unwrap_or(0) > 0 && args.expected_revision.is_none() {
            return Err(invalid("expectedRevision is required when paging"));
        }
        if args.output_path.is_some()
            && (args.page.is_some() || args.offset.is_some() || args.limit.is_some())
        {
            return Err(invalid(
                "outputPath cannot be combined with page, offset or limit",
            ));
        }
        let drawing = self.library.drawing(args.document_id)?;
        if args
            .expected_revision
            .is_some_and(|r| r != drawing.revision)
        {
            return Err(drawing_conflict());
        }
        let count = drawing.strokes.len();
        if let Some(path) = args.output_path {
            let destination = absolute_path(&path)?;
            let data = serde_json::to_vec(&drawing).map_err(storage)?;
            if data.len() > MAX_DRAWING_FILE {
                return Err(invalid("drawing exceeds 32 MiB file limit"));
            }
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(destination)
                .map_err(storage)?;
            if let Err(error) = file.write_all(&data) {
                drop(file);
                let _ = std::fs::remove_file(destination);
                return Err(storage(error));
            }
            return Ok(ReadDrawingOutcome {
                revision: drawing.revision,
                stroke_count: count,
                strokes: vec![],
                next_offset: None,
                output_path: Some(path),
            });
        }
        let limit = args.limit.unwrap_or(25);
        if limit == 0 || limit > MAX_PAGE_STROKES {
            return Err(invalid("limit must be between 1 and 100"));
        }
        let offset = args.offset.unwrap_or(0);
        if offset > count {
            return Err(invalid("offset exceeds drawing length"));
        }
        let mut strokes = Vec::new();
        let mut bytes = 0;
        let mut next = None;
        for (index, stroke) in drawing.strokes.into_iter().enumerate().skip(offset) {
            if args.page.is_some_and(|page| stroke.page != page) {
                continue;
            }
            let entry = IndexedDrawingStroke { index, stroke };
            let size = serde_json::to_vec(&entry).map_err(storage)?.len() + 1;
            if strokes.len() == limit || bytes + size > MAX_PAGE_BYTES {
                if strokes.is_empty() {
                    return Err(invalid(
                        "stroke exceeds page size; export the drawing to a file",
                    ));
                }
                next = Some(index);
                break;
            }
            bytes += size;
            strokes.push(entry);
        }
        Ok(ReadDrawingOutcome {
            revision: drawing.revision,
            stroke_count: count,
            strokes,
            next_offset: next,
            output_path: None,
        })
    }

    /// Save a complete drawing and notify subscribed score pages.
    pub fn save_drawing(&self, document: DocumentId, drawing: Drawing) -> Result<Drawing> {
        let drawing = self.library.save_drawing(document, drawing)?;
        self.emit(
            "drawing-changed",
            Some(document),
            None,
            json!({"revision": drawing.revision}),
        );
        Ok(drawing)
    }

    /// Apply one revision-guarded drawing change.
    pub fn write_drawing(&self, args: WriteDrawingArgs) -> Result<WriteDrawingOutcome> {
        let current = self.library.drawing(args.document_id)?;
        if current.revision != args.expected_revision {
            return Err(drawing_conflict());
        }
        let strokes = match args.change {
            DrawingChange::Import { input_path } => {
                let file = std::fs::File::open(absolute_path(&input_path)?).map_err(storage)?;
                let mut data = Vec::new();
                file.take((MAX_DRAWING_FILE + 1) as u64)
                    .read_to_end(&mut data)
                    .map_err(storage)?;
                if data.len() > MAX_DRAWING_FILE {
                    return Err(invalid("drawing exceeds 32 MiB file limit"));
                }
                let imported: Drawing =
                    serde_json::from_slice(&data).map_err(|e| invalid(e.to_string()))?;
                imported.strokes
            }
            DrawingChange::Replace { strokes } => strokes,
            DrawingChange::Edit { edits } => {
                let mut replacements = std::collections::HashMap::new();
                let mut additions = Vec::new();
                for edit in edits {
                    let (index, replacement) = match edit {
                        DrawingEdit::Add { stroke } => {
                            additions.push(stroke);
                            continue;
                        }
                        DrawingEdit::Delete { index } => (index, None),
                        DrawingEdit::Replace { index, stroke } => (index, Some(stroke)),
                    };
                    if index >= current.strokes.len()
                        || replacements.insert(index, replacement).is_some()
                    {
                        return Err(invalid("drawing edit index is out of range or duplicated"));
                    }
                }
                let mut strokes: Vec<_> = current
                    .strokes
                    .into_iter()
                    .enumerate()
                    .filter_map(|(index, stroke)| {
                        replacements.remove(&index).unwrap_or(Some(stroke))
                    })
                    .collect();
                strokes.extend(additions);
                strokes
            }
        };
        let drawing = Drawing {
            revision: args.expected_revision,
            strokes,
        };
        if serde_json::to_vec(&drawing).map_err(storage)?.len() > MAX_DRAWING_FILE {
            return Err(invalid("drawing exceeds 32 MiB limit"));
        }
        let saved = self.save_drawing(args.document_id, drawing)?;
        Ok(WriteDrawingOutcome {
            revision: saved.revision,
            stroke_count: saved.strokes.len(),
        })
    }
}
