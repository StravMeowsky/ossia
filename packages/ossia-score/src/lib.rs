#![doc = include_str!("../README.md")]
mod index;
mod instruments;
mod layout;
mod lexer;
mod mei;
mod midi;
mod midi_concat;
mod midi_range;
pub use mei::source_to_mei;
mod note_edit;
pub use note_edit::{apply_note_edit, NoteEditOp, NoteEditOutcome, NoteValue};
mod pitch;
mod position;
mod selection_source;
pub use selection_source::*;
mod selection;
pub use selection::*;

pub use index::*;
mod identity;
pub use identity::*;
mod crop;
pub use crop::*;
pub use instruments::*;
pub use layout::*;
pub use lexer::{serialize, tokenize, Mode, Token, TokenKind};
pub use midi::*;
pub use midi_concat::*;
pub use midi_range::*;
pub use ossia_types::{
    AppError, ContentHash, ErrorCode, LocatedRegion, LocationCapability, Locator, MeasureRange,
    SourceSpan,
};
pub use pitch::*;
pub use position::*;

mod analysis;
pub use analysis::*;

mod playback;
pub use playback::*;

mod verification;
pub use verification::*;
