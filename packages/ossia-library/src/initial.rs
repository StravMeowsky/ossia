use crate::{storage, storage_error, Library, Result};
use ossia_types::ContentHash;
use std::fs;

impl Library {
    /// Return whether this library was newly created and still needs its starter scores.
    pub fn initial_documents_pending(&self) -> Result<bool> {
        let path = self.store.path(storage::INITIAL_DOCUMENTS_MARKER)?;
        match fs::symlink_metadata(&path) {
            Ok(metadata) if metadata.file_type().is_symlink() => Err(storage_error(
                "initial-documents marker must not be a symlink",
            )),
            Ok(metadata) if metadata.is_file() => Ok(true),
            Ok(_) => Err(storage_error(
                "initial-documents marker must be a regular file",
            )),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
            Err(error) => Err(storage_error(error)),
        }
    }

    /// Mark the starter-score batch as complete.
    pub fn finish_initial_documents(&self) -> Result<()> {
        let path = self.store.path(storage::INITIAL_DOCUMENTS_MARKER)?;
        match fs::symlink_metadata(&path) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(storage_error(
                    "initial-documents marker must not be a symlink",
                ));
            }
            Ok(metadata) if metadata.is_file() => {}
            Ok(_) => {
                return Err(storage_error(
                    "initial-documents marker must be a regular file",
                ));
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(storage_error(error)),
        }
        fs::remove_file(&path).map_err(storage_error)?;
        storage::sync_directory(
            path.parent()
                .ok_or_else(|| storage_error("initial-documents marker has no parent"))?,
        )
    }

    /// Return whether any library document, active or archived, originally had this source.
    pub fn has_initial_content(&self, hash: &ContentHash) -> Result<bool> {
        let exists: i64 = self
            .db()?
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM documents WHERE initial_hash=?1)",
                [hash.to_string()],
                |row| row.get(0),
            )
            .map_err(storage_error)?;
        Ok(exists != 0)
    }
}
