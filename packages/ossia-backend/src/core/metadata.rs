use super::*;
use serde_json::json;

impl Runtime {
    /// Update one active score's library metadata and notify document readers.
    pub fn update_metadata(&self, args: UpdateMetadataArgs) -> Result<DocumentSummary> {
        self.available()?;
        let catalog =
            args.composer.is_some() || args.instruments.is_some() || args.period.is_some();
        if args.title.is_none() && !catalog {
            return Err(AppError::invalid(
                "provide title, composer, instruments or period",
            ));
        }
        let _mutation = self.begin_document_mutation(args.document_id)?;
        if let Some(title) = &args.title {
            self.library.update_title(args.document_id, title)?;
        }
        let mut summary = self.library.update_catalog(
            args.document_id,
            args.composer.as_deref(),
            args.instruments.as_deref(),
            args.period.as_deref(),
        )?;
        summary.url = Some(self.document_url(args.document_id));
        self.emit("document-changed", Some(args.document_id), None, json!({}));
        Ok(summary)
    }
}
