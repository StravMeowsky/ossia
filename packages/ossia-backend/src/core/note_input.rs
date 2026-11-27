//! Keyboard note input in the score page's edit mode.
use super::*;
use ossia_score::{apply_note_edit, source_to_mei, NoteEditOp};

impl Runtime {
    /// Begin note input, or resume the note-input edit left by a closed page or a restart.
    ///
    /// An edit by anyone else, such as an agent, keeps the document busy.
    pub fn note_input_begin(
        &self,
        document: DocumentId,
        args: NoteInputBeginArgs,
    ) -> Result<EditSession> {
        self.available()?;
        let _mutation = self.begin_document_mutation(document)?;
        self.observe(document)?;
        let edit = match self.library.active_edit(document)? {
            None => self.library.begin_edit(BeginEditRequest {
                document_id: document,
                base_version_id: args.base_version_id,
                expected_source_hash: &args.expected_source_hash,
                author: NOTE_INPUT_AUTHOR,
                adopt_workspace: false,
            })?,
            Some(edit) if edit.author != NOTE_INPUT_AUTHOR => {
                return Err(AppError::new(
                    ErrorCode::EditBusy,
                    "another editing task is changing this score",
                ));
            }
            Some(edit) if edit.status == EditStatus::Interrupted => self.library.resume_edit(
                edit.edit_id,
                args.base_version_id,
                &args.expected_source_hash,
            )?,
            Some(edit) => edit,
        };
        self.emit("document-changed", Some(document), None, json!({}));
        Ok(edit)
    }

    /// Apply one note-input action to an active edit, or read its preview when there is none.
    ///
    /// Writes skip the LilyPond preview: the page draws the returned MEI instead, and the
    /// engraved score is rendered once when the edit is checked and committed.
    pub fn note_input(
        &self,
        document: DocumentId,
        args: NoteInputArgs,
    ) -> Result<NoteInputOutcome> {
        self.available()?;
        self.owned_edit(document, args.edit_id)?;
        let source = self
            .library
            .source_text(document, &args.expected_source_hash)?;
        let target = args.target as usize;
        let (next, cursor) = match &args.action {
            None => (source.clone(), target),
            Some(NoteInputAction::Restore { source }) => (source.clone(), target),
            Some(action) => {
                let op = NoteEditOp::try_from(action)?;
                let outcome = apply_note_edit(&source, target, &op)?;
                (outcome.source, outcome.cursor)
            }
        };
        let source_hash = if next == source {
            args.expected_source_hash
        } else {
            self.write_source(WriteSourceArgs {
                edit_id: args.edit_id,
                expected_source_hash: args.expected_source_hash,
                change: SourceWriteChange::Replace { text: next.clone() },
                preview: false,
            })?
            .source_hash
        };
        let mei = source_to_mei(&next)?;
        Ok(NoteInputOutcome {
            source_hash,
            cursor: u32::try_from(cursor).map_err(|_| invalid("source is too large"))?,
            source: next,
            mei,
        })
    }
}
