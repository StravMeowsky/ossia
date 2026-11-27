//! Note input on the score page: one keyboard action on the selected note, chord or rest.
use crate::*;

/// A written note value: `log` 0 is a whole note, 7 a 128th; `dots` adds augmentation dots.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema, ts_rs::TS,
)]
#[serde(rename_all = "camelCase")]
pub struct NoteInputValue {
    pub log: u8,
    pub dots: u8,
}

/// One note-input action.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(
    tag = "kind",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
pub enum NoteInputAction {
    /// Write a note of `step` (0 = C … 6 = B); with `chord`, add it above the selected note.
    Enter {
        step: u8,
        value: NoteInputValue,
        chord: bool,
    },
    Rest {
        value: NoteInputValue,
    },
    /// Move by steps of the key's scale.
    Step {
        steps: i8,
    },
    /// Move by semitones.
    Transpose {
        semitones: i8,
    },
    SetValue {
        value: NoteInputValue,
    },
    /// Turn a note into a rest, or remove the selected head of a chord.
    Delete,
    ToggleTie,
    ToggleSlur,
    /// Toggle a dynamic such as `mf`, or a hairpin `<`, `>` or `!`.
    Dynamic {
        mark: String,
    },
    /// Put back an earlier source, for undo and redo.
    Restore {
        source: String,
    },
}

/// Start note input, or take back the note-input edit a closed page or a restart left behind.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NoteInputBeginArgs {
    pub base_version_id: Option<VersionId>,
    pub expected_source_hash: ContentHash,
}

/// Author recorded on edits made by note input on the score page.
pub const NOTE_INPUT_AUTHOR: &str = "user";

/// Apply a note-input action to the active edit, or read the preview when `action` is absent.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NoteInputArgs {
    pub edit_id: EditId,
    pub expected_source_hash: ContentHash,
    /// Byte offset of the selected event in the source.
    pub target: u32,
    #[serde(default)]
    #[ts(optional)]
    pub action: Option<NoteInputAction>,
}

/// The edited source, where the cursor moved, and a quick MEI rendering for the preview.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub struct NoteInputOutcome {
    pub source_hash: ContentHash,
    /// The full source after the action, kept by the page for undo.
    pub source: String,
    /// Byte offset of the event selected next.
    pub cursor: u32,
    pub mei: String,
}
