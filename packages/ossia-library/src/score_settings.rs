use crate::*;

#[cfg(test)]
#[path = "../tests/unit/score_settings.rs"]
mod tests;

impl Library {
    /// Read the settings shared by a score's versions.
    pub fn score_settings(&self, document: DocumentId) -> Result<ScoreSettings> {
        let conn = self.db()?;
        db::document(&conn, document)?;
        let body: Option<String> = conn
            .query_row(
                "SELECT body FROM score_settings WHERE document_id=?1",
                [document.to_string()],
                |row| row.get(0),
            )
            .optional()
            .map_err(storage_error)?;
        Ok(body.map(db::decode).transpose()?.unwrap_or_default())
    }

    /// Replace settings only if the caller has the current revision.
    pub fn save_score_settings(
        &self,
        document: DocumentId,
        mut settings: ScoreSettings,
    ) -> Result<ScoreSettings> {
        if settings.instruments.len() > 1024
            || settings
                .instruments
                .keys()
                .any(|key| key.is_empty() || key.len() > 1024)
        {
            return Err(AppError::invalid("invalid score instrument settings"));
        }
        let mut conn = self.db()?;
        let tx = conn.transaction().map_err(storage_error)?;
        db::document(&tx, document)?;
        let revision: Option<i64> = tx
            .query_row(
                "SELECT revision FROM score_settings WHERE document_id=?1",
                [document.to_string()],
                |row| row.get(0),
            )
            .optional()
            .map_err(storage_error)?;
        if u64::try_from(revision.unwrap_or(0)).map_err(storage_error)? != settings.revision {
            return Err(conflict(
                "score settings have changed; reload before saving",
            ));
        }
        settings.revision = settings
            .revision
            .checked_add(1)
            .filter(|revision| *revision <= i64::MAX as u64)
            .ok_or_else(|| conflict("score settings revision overflow"))?;
        tx.execute(
            "INSERT INTO score_settings(document_id,revision,body) VALUES (?1,?2,?3) ON CONFLICT(document_id) DO UPDATE SET revision=excluded.revision,body=excluded.body",
            params![document.to_string(), settings.revision as i64, db::encode(&settings)?],
        ).map_err(storage_error)?;
        tx.commit().map_err(storage_error)?;
        Ok(settings)
    }
}
