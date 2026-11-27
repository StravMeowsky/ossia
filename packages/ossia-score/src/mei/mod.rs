//! A quick MEI rendering of a LilyPond source for the editing preview.
//!
//! Element ids are `o` followed by the byte offset of the written event, so a click in the
//! preview selects the same source position that note input edits.
mod attrs;
mod commands;
mod walk;
mod write;

use crate::note_edit::{events::scan, top_level_expressions};
use crate::pitch::{self, NoteLanguage};
use crate::{tokenize, SourceMap, Token, TokenKind};
use ossia_types::AppError;
use std::collections::HashMap;
use write::Heads;

#[cfg(test)]
#[path = "../../tests/unit/mei.rs"]
mod tests;

/// MEI for a LilyPond source. Music the preview cannot place is left out.
pub fn source_to_mei(source: &str) -> Result<String, AppError> {
    let events = scan(source)?;
    let tokens: Vec<Token> = tokenize(source)
        .into_iter()
        .filter(|t| !t.trivia())
        .collect();
    let language = pitch::language_before(&tokens, source.len()).unwrap_or_default();
    let heads = heads(source, &tokens, &events, language);
    let timeline = walk::Walker::new(source, &events, language).run();
    Ok(write::write_mei(
        &timeline,
        &events,
        &heads,
        title(&tokens).as_deref(),
    ))
}

fn title(tokens: &[Token]) -> Option<String> {
    let at = tokens.iter().position(|t| t.raw == "title")?;
    let value = tokens.get(at + 2).filter(|t| t.kind == TokenKind::String)?;
    Some(value.raw.trim_matches('"').to_owned())
}

/// Sounding heads of every note and chord, read per top-level expression so relative octaves
/// follow the written order; anything the resolver cannot read falls back to absolute octaves.
fn heads(
    source: &str,
    tokens: &[Token],
    events: &[crate::note_edit::events::Event],
    language: NoteLanguage,
) -> HashMap<usize, Heads> {
    let map = SourceMap::new(source);
    let mut out: HashMap<usize, Heads> = HashMap::new();
    for (start, end) in top_level_expressions(tokens) {
        let Ok(span) = map.span(start, end) else {
            continue;
        };
        let Ok(result) = pitch::resolve_written_pitches(source, span) else {
            continue;
        };
        if result.unsupported.is_some() {
            continue;
        }
        for event in result.events {
            let heads = event.members.iter().map(|m| (m.di, m.alter)).collect();
            out.insert(event.span.byte_start, heads);
        }
    }
    for event in events.iter().filter(|e| e.pitched()) {
        out.entry(event.start).or_insert_with(|| {
            event
                .members
                .iter()
                .filter_map(|(start, end, _)| absolute(&source[*start..*end], language))
                .collect()
        });
    }
    out
}

/// A pitch read as if written in absolute octaves.
fn absolute(text: &str, language: NoteLanguage) -> Option<(i32, i32)> {
    let (step, alter) = crate::note_edit::spelling::parse_name(text, language)?;
    let octave: i32 = text
        .chars()
        .map(|c| match c {
            '\'' => 1,
            ',' => -1,
            _ => 0,
        })
        .sum();
    Some((step + 7 * octave, alter))
}
