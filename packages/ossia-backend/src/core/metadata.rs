use super::*;
use serde_json::json;

impl Runtime {
    /// Update one active score's library metadata and notify document readers.
    pub fn update_metadata(&self, args: UpdateMetadataArgs) -> Result<DocumentSummary> {
        self.available()?;
        let _mutation = self.begin_document_mutation(args.document_id)?;
        let mut summary = self.library.update_title(args.document_id, &args.title)?;
        summary.url = Some(self.document_url(args.document_id));
        self.emit("document-changed", Some(args.document_id), None, json!({}));
        Ok(summary)
    }
}
