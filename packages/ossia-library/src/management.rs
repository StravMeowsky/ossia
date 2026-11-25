use crate::{db, *};
use rusqlite::{params, OptionalExtension};

impl Library {
    /// Page through active or archived management metadata without reading mutable sources.
    pub fn management_score_page(
        &self,
        archived: bool,
        limit: usize,
        offset: usize,
    ) -> Result<ManagementScorePage> {
        let conn = self.db()?;
        let condition = if archived {
            "deleted_at IS NOT NULL"
        } else {
            "deleted_at IS NULL"
        };
        let total: i64 = conn
            .query_row(
                &format!("SELECT count(*) FROM documents WHERE {condition}"),
                [],
                |row| row.get(0),
            )
            .map_err(storage_error)?;
        let total = usize::try_from(total).map_err(storage_error)?;
        let order = if archived {
            "deleted_at,document_id"
        } else {
            "created_at,document_id"
        };
        let mut statement = conn
            .prepare(&format!(
                "SELECT document_id,title,created_at,updated_at,deleted_at,composer,instruments,period
                 FROM documents WHERE {condition}
                 ORDER BY {order} LIMIT ?1 OFFSET ?2"
            ))
            .map_err(storage_error)?;
        let rows = statement
            .query_map(
                params![
                    i64::try_from(limit).map_err(storage_error)?,
                    i64::try_from(offset).map_err(storage_error)?
                ],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, Option<String>>(4)?,
                        row.get::<_, Option<String>>(5)?,
                        row.get::<_, Option<String>>(6)?,
                        row.get::<_, Option<String>>(7)?,
                    ))
                },
            )
            .map_err(storage_error)?;
        let items = rows
            .map(|row| {
                let (
                    document_id,
                    title,
                    created_at,
                    updated_at,
                    archived_at,
                    composer,
                    instruments,
                    period,
                ) = row.map_err(storage_error)?;
                Ok(ManagementScoreSummary {
                    document_id: db::parse(document_id)?,
                    title,
                    created_at,
                    updated_at,
                    archived_at,
                    composer,
                    instruments: instruments
                        .map(|text| serde_json::from_str::<Vec<String>>(&text))
                        .transpose()
                        .map_err(storage_error)?
                        .filter(|list| !list.is_empty()),
                    period,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let end = offset.saturating_add(items.len());
        Ok(ManagementScorePage {
            items,
            total,
            offset,
            next_offset: (end < total).then_some(end),
        })
    }

    /// Read the first page from a score's last successful render, including archived scores.
    pub fn management_score_preview(&self, document_id: DocumentId) -> Result<Option<Vec<u8>>> {
        let body: Option<String> = self
            .db()?
            .query_row(
                "SELECT last_render FROM documents WHERE document_id=?1",
                [document_id.to_string()],
                |row| row.get(0),
            )
            .optional()
            .map_err(storage_error)?
            .flatten();
        let Some(body) = body else {
            return Ok(None);
        };
        let render: RenderManifest = db::decode(body)?;
        let Some(page) = render.pages.iter().find(|page| page.page == 1) else {
            return Ok(None);
        };
        self.read_artifact(&page.artifact).map(Some)
    }
}
