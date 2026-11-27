//! Note input: rewrite the LilyPond source of one selected note, chord or rest.
//!
//! Every edit replaces source text and then re-reads the pitches of the enclosing music
//! expression, so notes written in `\relative` mode keep their sounding pitch: octave marks of the
//! edited notes and of the next note are corrected until the analysis agrees.
mod context;
mod edits;
pub(crate) mod events;
mod input;
mod place;
pub(crate) mod spelling;

use crate::{pitch, tokenize, Mode, SourceMap, Token, TokenKind};
pub use events::NoteValue;
use events::{scan, values_for, Event, EventKind, Frac};
use ossia_types::AppError;
use spelling::{key_alter, key_fifths, marks, name, nearest, transposed, Spelled};

pub(crate) use context::top_level_expressions;
use context::*;
use edits::*;
use place::*;

#[cfg(test)]
#[path = "../../tests/unit/note_edit.rs"]
mod tests;

/// One note-input action on the selected event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NoteEditOp {
    /// Write a note of `step` (0 = C … 6 = B) with `value` at the selection, overwriting it and
    /// any following rests it needs; with `chord`, add the step above the selected note instead.
    Enter {
        step: u8,
        value: NoteValue,
        chord: bool,
    },
    /// Write a rest of `value` at the selection.
    Rest {
        value: NoteValue,
    },
    /// Move the selected note head, or every head of the selected chord, by semitones.
    /// Move the selected head, or every head of the selected chord, by steps of the key's scale.
    Step { steps: i8 },
    Transpose {
        semitones: i8,
    },
    /// Change the selected event's length; shortening leaves rests, lengthening uses rests.
    SetValue {
        value: NoteValue,
    },
    /// Turn a note or chord into a rest, or remove the selected head of a chord.
    Delete,
    ToggleTie,
    ToggleSlur,
    /// Toggle a dynamic such as `mf`, or a hairpin `<`, `>` or its end `!`.
    Dynamic {
        mark: String,
    },
}

/// The rewritten source and where the input cursor moves.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoteEditOutcome {
    pub source: String,
    /// Byte offset of the event the cursor selects next.
    pub cursor: usize,
}

/// Apply one note-input action to the event at byte offset `target`.
pub fn apply_note_edit(
    source: &str,
    target: usize,
    op: &NoteEditOp,
) -> Result<NoteEditOutcome, AppError> {
    let ctx = Context::new(source)?;
    let (mut index, mut head) = locate(&ctx.events, target)?;
    // At the input position, as in MuseScore, marks and moves apply to the note just entered
    // while the cursor stays where the next note goes.
    let marks_note = matches!(
        op,
        NoteEditOp::Transpose { .. }
            | NoteEditOp::Step { .. }
            | NoteEditOp::ToggleTie
            | NoteEditOp::ToggleSlur
            | NoteEditOp::Dynamic { .. }
            | NoteEditOp::Enter { chord: true, .. }
    );
    let mut stay = None;
    if marks_note && !ctx.events[index].pitched() {
        let previous = index
            .checked_sub(1)
            .filter(|p| ctx.events[*p].pitched() && ctx.events[*p].block == ctx.events[index].block);
        if let Some(previous) = previous {
            stay = Some(ctx.events[index].start);
            (index, head) = (previous, None);
        }
    }
    let event = &ctx.events[index];
    let rewrites_length = matches!(
        op,
        NoteEditOp::Enter { chord: false, .. }
            | NoteEditOp::Rest { .. }
            | NoteEditOp::SetValue { .. }
    );
    if event.kind == EventKind::MeasureRest {
        if !rewrites_length {
            return Err(invalid("write a note on the rest first"));
        }
        let expanded = split_measure_rest(&ctx, event)?;
        return apply_note_edit(&expanded, event.start, op);
    }
    if event.scale != Frac::new(1, 1) || event.count != 1 {
        if rewrites_length && event.count == 1 && !event.pitched() {
            // A scaled rest, such as one measure of 5/4, is overwritten like any rest.
        } else {
            return Err(AppError::unsupported("multiplied notes are not supported"));
        }
    }
    let pitches = analysis(source, target)?;
    let fifths = key_fifths(&ctx.tokens, event.start, ctx.language);
    let (changes, cursor) = match op {
        NoteEditOp::Enter {
            step,
            value,
            chord: false,
        } => {
            let pitch = nearest(
                i32::from(*step),
                ctx.previous_di(event.start, &pitches),
                fifths,
            );
            let text = format!(
                "{}{}",
                name(pitch.step(), pitch.alter, ctx.language),
                value.text()
            );
            overwrite(&ctx, index, value.length(), text, vec![(0, pitch)])?
        }
        NoteEditOp::Rest { value } => overwrite(
            &ctx,
            index,
            value.length(),
            format!("r{}", value.text()),
            vec![],
        )?,
        NoteEditOp::Enter {
            step, chord: true, ..
        } => {
            if !event.pitched() {
                return Err(invalid("select a note to add a chord tone"));
            }
            let current = head_pitches(&pitches, event)?;
            let top = current.iter().map(|p| p.di).max().unwrap_or(0);
            let mut pitch = nearest(i32::from(*step), top, fifths);
            if pitch.di <= top {
                pitch.di += 7;
            }
            let mut heads = own_heads(&ctx, event, current);
            heads.push((name(pitch.step(), pitch.alter, ctx.language), pitch));
            let change = rebuild(&ctx, event, heads);
            (vec![change], Cursor::At(cursor_of(event)))
        }
        NoteEditOp::Transpose { .. } | NoteEditOp::Step { .. } => {
            if !event.pitched() {
                return Err(invalid("select a note to move it"));
            }
            let current = head_pitches(&pitches, event)?;
            let heads = own_heads(&ctx, event, current)
                .into_iter()
                .enumerate()
                .map(|(k, (text, pitch))| {
                    if head.is_some_and(|h| h != k) {
                        return (text, pitch);
                    }
                    let moved = match op {
                        NoteEditOp::Step { steps } => {
                            let di = pitch.di + i32::from(*steps);
                            Spelled {
                                di,
                                alter: key_alter(fifths, di.rem_euclid(7)),
                            }
                        }
                        NoteEditOp::Transpose { semitones } => {
                            transposed(pitch, i32::from(*semitones))
                        }
                        _ => pitch,
                    };
                    (name(moved.step(), moved.alter, ctx.language), moved)
                })
                .collect();
            let changes = vec![rebuild(&ctx, event, heads)];
            (changes, Cursor::At(cursor_of(event)))
        }
        NoteEditOp::SetValue { value } => set_value(&ctx, index, *value, &pitches)?,
        NoteEditOp::Delete => {
            if !event.pitched() {
                return Err(invalid("select a note to delete"));
            }
            let change = match head.filter(|_| event.members.len() > 1) {
                Some(k) => {
                    let mut heads = own_heads(&ctx, event, head_pitches(&pitches, event)?);
                    heads.remove(k);
                    rebuild(&ctx, event, heads)
                }
                None => {
                    let mut end = event.end;
                    // A rest cannot be tied.
                    if let Some(tie) = ctx
                        .post_tokens(event)
                        .iter()
                        .find(|t| t.kind == TokenKind::Tie)
                    {
                        if tie.byte_start == event.end {
                            end = tie.byte_end;
                        }
                    }
                    Change {
                        start: event.start,
                        end,
                        text: format!("r{}", value_text(&ctx, event)?),
                        heads: vec![],
                    }
                }
            };
            (vec![change], Cursor::At(event.start))
        }
        NoteEditOp::ToggleTie => {
            if !event.pitched() {
                return Err(invalid("select a note to tie"));
            }
            let change = match ctx
                .post_tokens(event)
                .iter()
                .find(|t| t.kind == TokenKind::Tie)
            {
                Some(tie) => Change {
                    start: tie.byte_start,
                    end: tie.byte_end,
                    text: String::new(),
                    heads: vec![],
                },
                None => Change {
                    start: event.end,
                    end: event.end,
                    text: "~".into(),
                    heads: vec![],
                },
            };
            (vec![change], Cursor::At(cursor_of(event)))
        }
        NoteEditOp::ToggleSlur => (toggle_slur(&ctx, index)?, Cursor::At(cursor_of(event))),
        NoteEditOp::Dynamic { mark } => {
            let family: &[&str] = if HAIRPINS.contains(&mark.as_str()) {
                &HAIRPINS
            } else if DYNAMICS.contains(&mark.as_str()) {
                &DYNAMICS
            } else {
                return Err(invalid("unknown dynamic"));
            };
            let existing: Vec<&Token> = ctx
                .post_tokens(event)
                .into_iter()
                .filter(|t| t.kind == TokenKind::Cmd && family.contains(&&t.raw[1..]))
                .collect();
            let same = existing.iter().any(|t| t.raw[1..] == *mark);
            let mut changes: Vec<Change> = existing
                .iter()
                .map(|t| Change {
                    start: t.byte_start,
                    end: t.byte_end,
                    text: String::new(),
                    heads: vec![],
                })
                .collect();
            if !same {
                let at = ctx.post_end(event.full_end);
                // A command name would swallow letters written straight after it.
                let joined = source[at..].starts_with(|c: char| c.is_ascii_alphanumeric());
                changes.push(Change {
                    start: at,
                    end: at,
                    text: format!("\\{mark}{}", if joined { " " } else { "" }),
                    heads: vec![],
                });
            }
            (changes, Cursor::At(cursor_of(event)))
        }
    };
    finish(&ctx, &pitches, changes, stay.map_or(cursor, Cursor::At))
}
