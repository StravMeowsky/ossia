use super::*;
use rusqlite::{params, Connection, OptionalExtension};

impl Library {
    pub fn get_document(&self, id: DocumentId) -> Result<DocumentState> {
        self.observe_source(id)
    }

    /// Change the catalog fields used to group the library. `None` leaves a field unchanged;
    /// an empty value clears it.
    pub fn update_catalog(
        &self,
        id: DocumentId,
        composer: Option<&str>,
        instruments: Option<&[String]>,
        period: Option<&str>,
    ) -> Result<DocumentSummary> {
        let composer = composer.map(|value| catalog_text("composer", value)).transpose()?;
        let period = period.map(|value| catalog_text("period", value)).transpose()?;
        let instruments = instruments
            .map(|values| {
                if values.len() > 20 {
                    return Err(AppError::invalid("at most 20 instruments are allowed"));
                }
                let mut list: Vec<String> = Vec::new();
                for value in values {
                    if let Some(name) = catalog_text("instrument", value)? {
                        if !list.iter().any(|known| known.eq_ignore_ascii_case(&name)) {
                            list.push(name);
                        }
                    }
                }
                Ok((!list.is_empty())
                    .then(|| serde_json::to_string(&list).map_err(storage_error))
                    .transpose()?)
            })
            .transpose()?;

        let mut conn = self.db()?;
        let tx = conn.transaction().map_err(storage_error)?;
        db::document(&tx, id)?;
        for (column, value) in [
            ("composer", composer),
            ("instruments", instruments),
            ("period", period),
        ] {
            if let Some(value) = value {
                tx.execute(
                    &format!("UPDATE documents SET {column}=?1 WHERE document_id=?2"),
                    params![value, id.to_string()],
                )
                .map_err(storage_error)?;
            }
        }
        tx.execute(
            "UPDATE documents SET updated_at=?1 WHERE document_id=?2",
            params![db::now(), id.to_string()],
        )
        .map_err(storage_error)?;
        let doc = db::document(&tx, id)?;
        let summary = self.document_summary(&tx, &doc)?;
        tx.commit().map_err(storage_error)?;
        Ok(summary)
    }

    /// Replace one active document's library title without touching its source or history.
    pub fn update_title(&self, id: DocumentId, title: &str) -> Result<DocumentSummary> {
        let title = title.trim();
        if title.is_empty() {
            return Err(AppError::invalid("title must not be empty"));
        }
        if title.chars().count() > 200 {
            return Err(AppError::invalid("title must be at most 200 characters"));
        }
        if title.chars().any(char::is_control) {
            return Err(AppError::invalid(
                "title must not contain control characters",
            ));
        }

        let mut conn = self.db()?;
        let tx = conn.transaction().map_err(storage_error)?;
        db::document(&tx, id)?;
        tx.execute(
            "UPDATE documents SET title=?1,updated_at=?2 WHERE document_id=?3",
            params![title, db::now(), id.to_string()],
        )
        .map_err(storage_error)?;
        let doc = db::document(&tx, id)?;
        let summary = self.document_summary(&tx, &doc)?;
        tx.commit().map_err(storage_error)?;
        Ok(summary)
    }

    pub fn observe_source(&self, id: DocumentId) -> Result<DocumentState> {
        let conn = self.db()?;
        let doc = db::document(&conn, id)?;
        let snapshot = self.snapshot(&doc)?;
        let dirty = match doc.head {
            Some(head) => db::version(&conn, head)?.source_hash != *snapshot.source_hash(),
            None => true,
        };
        let pending = conn
            .query_row(
                "SELECT 1 FROM worktree_intents WHERE document_id=?1",
                [id.to_string()],
                |r| r.get::<_, i32>(0),
            )
            .optional()
            .map_err(storage_error)?
            .is_some();
        let edit_state = if pending {
            DocumentEditState::RecoveryConflict
        } else {
            match db::active_edit(&conn, id)?.map(|e| e.status) {
                Some(EditStatus::Active) => DocumentEditState::Active,
                Some(EditStatus::Interrupted) => DocumentEditState::Interrupted,
                _ => DocumentEditState::Idle,
            }
        };
        let current_row = conn.query_row("SELECT rowid,body FROM renders WHERE document_id=?1 AND source_hash=?2 ORDER BY (body=(SELECT last_render FROM documents WHERE document_id=?1)) DESC,rowid DESC LIMIT 1",
            params![id.to_string(),snapshot.source_hash().to_string()],|r|Ok((r.get::<_,i64>(0)?,r.get::<_,String>(1)?))).optional().map_err(storage_error)?;
        if let Some((rowid, _)) = &current_row {
            conn.execute(
                "UPDATE renders SET last_used_at=?1 WHERE rowid=?2",
                params![db::now_millis(), rowid],
            )
            .map_err(storage_error)?;
        }
        let current = current_row.map(|(_, body)| body);
        let fallback = if current.is_none() {
            conn.query_row(
                "SELECT last_render FROM documents WHERE document_id=?1",
                [id.to_string()],
                |r| r.get::<_, Option<String>>(0),
            )
            .map_err(storage_error)?
        } else {
            None
        };
        Ok(DocumentState { document_id:id,title:doc.title,source_path:path_text(self.store.path(&doc.source_path)?),
            head_version_id:doc.head,source_hash:snapshot.source_hash().clone(),dirty,edit_state,
            last_successful_render:current.or(fallback).map(db::decode).transpose()?,
            last_check:conn.query_row("SELECT body FROM jobs WHERE document_id=?1 AND source_hash=?2 AND kind IN ('check','import') ORDER BY rowid DESC LIMIT 1", params![id.to_string(),snapshot.source_hash().to_string()],|r|r.get::<_,String>(0)).optional().map_err(storage_error)?.map(db::decode).transpose()? })
    }

    pub fn list_documents(&self) -> Result<Vec<DocumentState>> {
        let ids: Vec<String> = {
            let conn = self.db()?;
            let mut stmt = conn
                .prepare("SELECT document_id FROM documents WHERE deleted_at IS NULL ORDER BY created_at,document_id")
                .map_err(storage_error)?;
            let rows = stmt.query_map([], |r| r.get(0)).map_err(storage_error)?;
            rows.collect::<std::result::Result<_, _>>()
                .map_err(storage_error)?
        };
        ids.into_iter()
            .map(|id| self.get_document(db::parse(id)?))
            .collect()
    }

    /// List compact document metadata without loading render or check history.
    pub fn document_page(&self, limit: usize, offset: usize) -> Result<DocumentPage> {
        let conn = self.db()?;
        let total: i64 = conn
            .query_row(
                "SELECT count(*) FROM documents WHERE deleted_at IS NULL",
                [],
                |r| r.get(0),
            )
            .map_err(storage_error)?;
        let total = usize::try_from(total).map_err(storage_error)?;
        let mut statement = conn
            .prepare(
                "SELECT document_id FROM documents WHERE deleted_at IS NULL ORDER BY created_at,document_id LIMIT ?1 OFFSET ?2",
            )
            .map_err(storage_error)?;
        let ids = statement
            .query_map(
                params![
                    i64::try_from(limit).map_err(storage_error)?,
                    i64::try_from(offset).map_err(storage_error)?
                ],
                |r| r.get::<_, String>(0),
            )
            .map_err(storage_error)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(storage_error)?;
        let items = ids
            .into_iter()
            .map(|id| {
                let doc = db::document(&conn, db::parse(id)?)?;
                self.document_summary(&conn, &doc)
            })
            .collect::<Result<Vec<_>>>()?;
        let end = offset.saturating_add(items.len());
        Ok(DocumentPage {
            items,
            total,
            offset,
            next_offset: (end < total).then_some(end),
        })
    }

    pub(crate) fn document_summary(
        &self,
        conn: &Connection,
        doc: &db::Document,
    ) -> Result<DocumentSummary> {
        let snapshot = self.snapshot(doc)?;
        let dirty = match doc.head {
            Some(head) => db::version(conn, head)?.source_hash != *snapshot.source_hash(),
            None => true,
        };
        let pending = conn
            .query_row(
                "SELECT 1 FROM worktree_intents WHERE document_id=?1",
                [doc.id.to_string()],
                |r| r.get::<_, i32>(0),
            )
            .optional()
            .map_err(storage_error)?
            .is_some();
        let edit_state = if pending {
            DocumentEditState::RecoveryConflict
        } else {
            match db::active_edit(conn, doc.id)?.map(|e| e.status) {
                Some(EditStatus::Active) => DocumentEditState::Active,
                Some(EditStatus::Interrupted) => DocumentEditState::Interrupted,
                _ => DocumentEditState::Idle,
            }
        };
        Ok(DocumentSummary {
            document_id: doc.id,
            url: None,
            title: doc.title.clone(),
            updated_at: doc.updated_at.clone(),
            head_version_id: doc.head,
            source_hash: snapshot.source_hash().clone(),
            dirty,
            edit_state,
        })
    }
}

/// Trim a catalog value; empty clears it, and the same limits as titles apply.
fn catalog_text(field: &str, value: &str) -> Result<Option<String>> {
    let value = value.trim();
    if value.chars().count() > 200 {
        return Err(AppError::invalid(format!(
            "{field} must be at most 200 characters"
        )));
    }
    if value.chars().any(char::is_control) {
        return Err(AppError::invalid(format!(
            "{field} must not contain control characters"
        )));
    }
    Ok((!value.is_empty()).then(|| value.to_string()))
}
