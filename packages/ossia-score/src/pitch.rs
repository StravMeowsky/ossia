use crate::{
    index::{unsupported_token, Syntax},
    tokenize, Mode, SourceMap, TokenKind,
};
use ossia_types::{AppError, SourceSpan};
use serde::{Deserialize, Serialize};

mod commands;
mod names;

#[cfg(test)]
#[path = "../tests/unit/pitch.rs"]
mod tests;

pub(crate) use commands::known_command;
use commands::safe_stem_direction_alias;
pub use names::midi_pitch_name;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PitchMember {
    pub raw: String,
    pub midi: i32,
    pub di: i32,
    pub alter: i32,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PitchEventKind {
    Note,
    Chord,
    Rest,
    Skip,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PitchEvent {
    pub kind: PitchEventKind,
    pub raw: String,
    pub span: SourceSpan,
    pub pitches: Vec<i32>,
    pub members: Vec<PitchMember>,
    pub ref_di: i32,
    pub out_of_range: bool,
    #[serde(default)]
    pub voice: Option<usize>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PitchResult {
    pub events: Vec<PitchEvent>,
    pub end_ref_di: Option<i32>,
    /// When present, the prediction is incomplete; events may cover only supported branches.
    pub unsupported: Option<String>,
}

#[derive(Clone, Copy)]
struct Pitch {
    step: i32,
    alter: i32,
    marks: i32,
}

/// Note-name language in effect for a source region. Other LilyPond
/// languages are outside the verified pitch subset.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub(crate) enum NoteLanguage {
    #[default]
    Nederlands,
    English,
    Deutsch,
}

impl NoteLanguage {
    /// The language selected by `\language "name"` or `\include "name.ly"`.
    fn from_directive(command: &str, argument: &str) -> Option<Self> {
        let name = argument.strip_prefix('"')?.strip_suffix('"')?;
        let name = match command {
            "language" => name,
            "include" => name.strip_suffix(".ly")?,
            _ => return None,
        };
        match name {
            "nederlands" => Some(Self::Nederlands),
            "english" => Some(Self::English),
            "deutsch" => Some(Self::Deutsch),
            _ => None,
        }
    }
}

/// A note-language directive at `tokens[index]`: `Some(Ok(language))` for a
/// supported one, `Some(Err(()))` for `\language` with any other name.
pub(crate) fn language_directive(
    tokens: &[crate::Token],
    index: usize,
) -> Option<Result<NoteLanguage, ()>> {
    let command = tokens[index].command()?;
    if !matches!(command, "language" | "include") {
        return None;
    }
    let argument = tokens[index + 1..].iter().find(|t| !t.trivia())?;
    match NoteLanguage::from_directive(command, &argument.raw) {
        Some(language) => Some(Ok(language)),
        None if command == "language" => Some(Err(())),
        None => None,
    }
}

/// The note language in effect before byte `before`, or `None` when an
/// unsupported `\language` precedes it.
pub(crate) fn language_before(tokens: &[crate::Token], before: usize) -> Option<NoteLanguage> {
    let mut language = NoteLanguage::default();
    for index in 0..tokens.len() {
        if tokens[index].byte_start >= before {
            break;
        }
        match language_directive(tokens, index) {
            Some(Ok(next)) => language = next,
            Some(Err(())) => return None,
            None => {}
        }
    }
    Some(language)
}

pub(crate) fn transpose_interval(from: &str, to: &str, language: NoteLanguage) -> Option<i32> {
    let from = parse_pitch(from, language)?;
    let to = parse_pitch(to, language)?;
    Some(midi(di(to), to.alter) - midi(di(from), from.alter))
}

/// English names: `s`/`sharp` raise, `f`/`flat` lower, `ss`/`x`/`ff` double.
fn english_alteration(tail: &str) -> (i32, &str) {
    for (suffix, alter) in [
        ("-sharpsharp", 2),
        ("-flatflat", -2),
        ("sharpsharp", 2),
        ("flatflat", -2),
        ("-sharp", 1),
        ("-flat", -1),
        ("sharp", 1),
        ("flat", -1),
        ("ss", 2),
        ("ff", -2),
        ("x", 2),
        ("s", 1),
        ("f", -1),
    ] {
        if let Some(rest) = tail.strip_prefix(suffix) {
            return (alter, rest);
        }
    }
    (0, tail)
}

fn parse_pitch(s: &str, language: NoteLanguage) -> Option<Pitch> {
    let german = language == NoteLanguage::Deutsch;
    let step_byte = *s.as_bytes().first()?;
    let step = match step_byte {
        b'c' => 0,
        b'd' => 1,
        b'e' => 2,
        b'f' => 3,
        b'g' => 4,
        b'a' => 5,
        b'b' => 6,
        b'h' if language != NoteLanguage::English => 6,
        _ => return None,
    };
    let mut tail = &s[1..];
    if language == NoteLanguage::English {
        let (alter, rest) = english_alteration(tail);
        return finish_pitch(step, alter, rest);
    }
    if german && step_byte == b'b' && tail.starts_with(|c: char| c.is_ascii_alphabetic()) {
        return None;
    }
    let mut alter = if german && step_byte == b'b' { -1 } else { 0 };
    for (acc, n) in [
        ("isis", 2),
        ("eses", -2),
        ("ses", i32::MAX),
        ("is", 1),
        ("es", -1),
        ("ss", 2),
        ("ff", -2),
        ("f", -1),
    ] {
        if let Some(rest) = tail.strip_prefix(acc) {
            if n == i32::MAX {
                return None;
            }
            alter = n;
            tail = rest;
            break;
        }
    }
    // LilyPond's default Dutch names use a single -s for flats on the vowels
    // "a" and "e" (as, es), while English-style -s remains a sharp elsewhere.
    if let Some(rest) = tail.strip_prefix('s') {
        alter = if matches!(step_byte, b'a' | b'e') {
            -1
        } else {
            1
        };
        tail = rest;
    }
    finish_pitch(step, alter, tail)
}

/// Octave marks, an optional `!`, then nothing but a written duration.
fn finish_pitch(step: i32, alter: i32, mut tail: &str) -> Option<Pitch> {
    let mut marks = 0i32;
    while let Some(c) = tail.chars().next().filter(|c| *c == '\'' || *c == ',') {
        marks = marks.checked_add(if c == '\'' { 1 } else { -1 })?;
        tail = &tail[1..];
    }
    tail = tail.strip_prefix('!').unwrap_or(tail);
    if !tail.is_empty()
        && (!tail.as_bytes()[0].is_ascii_digit()
            || !tail.bytes().all(|b| b.is_ascii_digit() || b == b'.'))
    {
        return None;
    }
    Some(Pitch { step, alter, marks })
}
fn di(p: Pitch) -> i32 {
    p.marks.saturating_mul(7).saturating_add(p.step)
}
fn midi(di: i32, alter: i32) -> i32 {
    48i32
        .saturating_add(12i32.saturating_mul(di.div_euclid(7)))
        .saturating_add([0, 2, 4, 5, 7, 9, 11][di.rem_euclid(7) as usize])
        .saturating_add(alter)
}
fn unsupported(reason: impl Into<String>) -> PitchResult {
    PitchResult {
        events: Vec::new(),
        end_ref_di: None,
        unsupported: Some(reason.into()),
    }
}

/// Resolve sounding pitches in one complete music expression while retaining
/// literal source tokens. Opaque music returns no prediction. Note-on timing,
/// ties, articulations and duration are outside this interface.
pub fn resolve_pitches(source: &str, span: SourceSpan) -> Result<PitchResult, AppError> {
    resolve(source, span, false)
}

/// Resolve pitches in written order for editing: repeats are read once and simultaneous voices
/// one after another, which is how `\relative` chains them.
pub(crate) fn resolve_written_pitches(
    source: &str,
    span: SourceSpan,
) -> Result<PitchResult, AppError> {
    resolve(source, span, true)
}

fn resolve(source: &str, span: SourceSpan, written: bool) -> Result<PitchResult, AppError> {
    let map = SourceMap::new(source);
    let text = map.slice(span)?;
    let document_tokens = tokenize(source);
    let mut language = NoteLanguage::default();
    for (i, token) in document_tokens.iter().enumerate() {
        if let Some(directive) = language_directive(&document_tokens, i) {
            let Ok(next) = directive else {
                return Ok(unsupported(
                    "document note language is outside the verified pitch subset",
                ));
            };
            if (span.byte_start..span.byte_end).contains(&token.byte_start) {
                return Ok(unsupported("note language changes inside pitch expression"));
            }
            if token.byte_start < span.byte_start {
                language = next;
            }
            continue;
        }
        if token.command() == Some("include") {
            let next = document_tokens[i + 1..].iter().find(|t| !t.trivia());
            if !next.is_some_and(|t| pitch_neutral_include(&t.raw)) {
                return Ok(unsupported(
                    "included definitions may change pitch semantics",
                ));
            }
        }
    }
    let syntax = Syntax::new(text);
    if syntax.malformed {
        return Err(AppError::invalid("unbalanced pitch expression"));
    }
    let tokens = tokenize(text);
    let mut active = true;
    for t in &tokens {
        // Scheme inside markup is text, but executable Scheme inside music is opaque.
        if t.mode == Mode::Text {
            continue;
        }
        // Written order reads through repeats and Scheme values; strict analysis does not.
        if unsupported_token(t) && !written {
            return Ok(unsupported(format!(
                "{} is outside the verified pitch subset",
                t.raw
            )));
        }
    }
    if !written {
        if let Some(result) = resolve_simultaneous(source, span, &syntax)? {
            return Ok(result);
        }
    }
    let mut events = Vec::new();
    let mut ref_di = 0i32;
    let mut relative = false;
    let mut transpose = 0i32;
    let mut depth = 0usize;
    // Depth, relative mode, reference and transposition before the block, and whether the block
    // is the body of `\relative`, `\absolute` or `\transpose`.
    let mut states: Vec<(usize, bool, i32, i32, bool)> = Vec::new();
    let mut pending: Option<(bool, i32)> = None;
    let mut pending_transpose = None;
    let mut chord: Option<(usize, Vec<PitchMember>)> = None;
    let mut i = 0;
    while i < tokens.len() {
        let t = &tokens[i];
        if t.mode == Mode::Text {
            i += 1;
            continue;
        }
        if t.command() == Some("markup") {
            // Unbraced markup commands may take bare identifiers as arguments without leaving music mode.
            // Resume at the next music atom so following notes keep the active relative pitch state.
            active = false;
            i += 1;
            continue;
        }
        if !active {
            if t.mode == Mode::Music
                && matches!(
                    t.kind,
                    TokenKind::Note | TokenKind::Rest | TokenKind::Skip | TokenKind::ChordO
                )
            {
                active = true;
            }
            if t.raw == "{" {
                i += 1;
                active = true;
                continue;
            }
            if t.kind == TokenKind::Ws
                || t.kind == TokenKind::Comment
                || t.kind == TokenKind::Cmd
                || t.kind == TokenKind::Num
                || t.kind == TokenKind::Ident
                || t.kind == TokenKind::String
                || t.kind == TokenKind::Fing
            {
                i += 1;
                continue;
            }
            if !active && !written {
                return Ok(unsupported("unbounded markup expression"));
            }
            // Written order reads on: the markup ended where something else begins.
            active = true;
        }
        if let Some(cmd) = t.command() {
            if cmd == "relative" {
                let next = (i + 1..tokens.len()).find(|&j| !tokens[j].trivia());
                let Some(j) = next else {
                    return Ok(unsupported("relative has no reference pitch"));
                };
                let Some(p) = parse_pitch(&tokens[j].raw, language) else {
                    return Ok(unsupported(
                        "relative reference is outside the verified subset",
                    ));
                };
                pending = Some((true, di(p)));
                i = j + 1;
                continue;
            }
            if cmd == "transpose" {
                let mut pitches = (i + 1..tokens.len()).filter(|&j| !tokens[j].trivia());
                let Some(from_index) = pitches.next() else {
                    return Ok(unsupported("transpose has no source pitch"));
                };
                let Some(to_index) = pitches.next() else {
                    return Ok(unsupported("transpose has no target pitch"));
                };
                let Some(from) = parse_pitch(&tokens[from_index].raw, language) else {
                    return Ok(unsupported(
                        "transpose source is outside the verified pitch subset",
                    ));
                };
                let Some(to) = parse_pitch(&tokens[to_index].raw, language) else {
                    return Ok(unsupported(
                        "transpose target is outside the verified pitch subset",
                    ));
                };
                pending_transpose =
                    Some(midi(di(to), to.alter).saturating_sub(midi(di(from), from.alter)));
                i = to_index + 1;
                continue;
            }
            if cmd == "absolute" {
                pending = Some((false, ref_di));
                i += 1;
                continue;
            }
            if cmd == "tweak" {
                let mut skipped = 0;
                i += 1;
                while i < tokens.len() && skipped < 2 {
                    if tokens[i].trivia() {
                        i += 1;
                        continue;
                    }
                    i += 1;
                    skipped += 1;
                }
                continue;
            }
            if written && matches!(cmd, "repeat" | "new" | "context") {
                // Skip the repeat kind and count, or the context type and an optional name.
                let mut skip = 2;
                let mut j = i + 1;
                while j < tokens.len() && skip > 0 {
                    if !tokens[j].trivia() {
                        skip -= 1;
                        if cmd != "repeat" && skip == 1 {
                            let named = tokens[j + 1..].iter().find(|t| !t.trivia());
                            if named.is_none_or(|t| t.raw != "=") {
                                skip = 0;
                            }
                        }
                    }
                    j += 1;
                }
                if cmd != "repeat" {
                    // `= "name"` follows the context type.
                    while j < tokens.len() && (tokens[j].trivia() || tokens[j].raw == "=") {
                        j += 1;
                    }
                    if j < tokens.len() && tokens[j].kind == TokenKind::String {
                        j += 1;
                    }
                }
                i = j;
                continue;
            }
            if written && cmd == "alternative" {
                i += 1;
                continue;
            }
            // Editing reads past commands it does not know; strict analysis stops there.
            if !written && !known_command(cmd) && !safe_stem_direction_alias(source, cmd) {
                return Ok(unsupported(format!("unresolved music command: \\{cmd}")));
            }
            i += 1;
            continue;
        }
        if written && (t.raw == "<<" || t.raw == ">>") {
            i += 1;
            continue;
        }
        if t.raw == "<<" || t.raw == ">>" {
            return Ok(unsupported(
                "simultaneous music needs separate pitch states",
            ));
        }
        if t.kind == TokenKind::Brace {
            if t.raw == "{" {
                depth += 1;
                let wrapper = pending.is_some() || pending_transpose.is_some();
                states.push((depth, relative, ref_di, transpose, wrapper));
                // An enclosing `\relative` does not reach into transposed music.
                if pending.is_none() && pending_transpose.is_some() {
                    relative = false;
                }
                if let Some((rel, new_ref)) = pending.take() {
                    relative = rel;
                    ref_di = new_ref;
                }
                if let Some(offset) = pending_transpose.take() {
                    transpose = transpose.saturating_add(offset);
                }
            } else {
                if let Some((_, rel, old_ref, old_transpose, wrapper)) =
                    states.pop_if(|s| s.0 == depth)
                {
                    // The outer expression exposes its final reference; nested wrappers restore the
                    // parent, while plain nested blocks such as tuplets pass their last pitch on.
                    if depth > 1 && wrapper {
                        relative = rel;
                        ref_di = old_ref;
                        transpose = old_transpose;
                    }
                }
                depth = depth.saturating_sub(1);
            }
            i += 1;
            continue;
        }
        if t.kind == TokenKind::ChordO {
            if chord.is_some() {
                return Err(AppError::invalid("nested chord"));
            }
            chord = Some((t.byte_start, Vec::new()));
            i += 1;
            continue;
        }
        if t.kind == TokenKind::ChordC {
            let Some((start, members)) = chord.take() else {
                return Err(AppError::invalid("unmatched chord close"));
            };
            if members.is_empty() {
                return Ok(unsupported("empty chord"));
            }
            ref_di = members[0].di;
            let mut pitches: Vec<_> = members.iter().map(|p| p.midi).collect();
            pitches.sort_unstable();
            pitches.dedup();
            let event_span = map.span(span.byte_start + start, span.byte_start + t.byte_end)?;
            events.push(PitchEvent {
                kind: PitchEventKind::Chord,
                raw: map.slice(event_span)?.into(),
                span: event_span,
                out_of_range: pitches.iter().any(|n| !(0..=127).contains(n)),
                pitches,
                members,
                ref_di,
                voice: None,
            });
            i += 1;
            continue;
        }
        if t.mode == Mode::Music
            && matches!(t.kind, TokenKind::Note | TokenKind::Rest | TokenKind::Skip)
        {
            let event_span =
                map.span(span.byte_start + t.byte_start, span.byte_start + t.byte_end)?;
            if matches!(t.kind, TokenKind::Rest | TokenKind::Skip) {
                events.push(PitchEvent {
                    kind: if t.kind == TokenKind::Rest {
                        PitchEventKind::Rest
                    } else {
                        PitchEventKind::Skip
                    },
                    raw: t.raw.clone(),
                    span: event_span,
                    pitches: Vec::new(),
                    members: Vec::new(),
                    ref_di,
                    out_of_range: false,
                    voice: None,
                });
            } else {
                let Some(p) = parse_pitch(&t.raw, language) else {
                    return Ok(unsupported(format!(
                        "unsupported pitch spelling: {}",
                        t.raw
                    )));
                };
                let d = if relative {
                    let delta = (p.step - ref_di.rem_euclid(7) + 3).rem_euclid(7) - 3;
                    ref_di
                        .saturating_add(delta)
                        .saturating_add(p.marks.saturating_mul(7))
                } else {
                    di(p)
                };
                let n = midi(d, p.alter).saturating_add(transpose);
                ref_di = d;
                let member = PitchMember {
                    raw: t.raw.clone(),
                    midi: n,
                    di: d,
                    alter: p.alter,
                };
                if let Some((_, members)) = &mut chord {
                    members.push(member);
                } else {
                    events.push(PitchEvent {
                        kind: PitchEventKind::Note,
                        raw: t.raw.clone(),
                        span: event_span,
                        pitches: vec![n],
                        members: vec![member],
                        ref_di,
                        out_of_range: !(0..=127).contains(&n),
                        voice: None,
                    });
                }
            }
        }
        i += 1;
    }
    if chord.is_some() {
        return Err(AppError::invalid("unclosed chord"));
    }
    if pending.is_some() {
        return Ok(unsupported("pitch wrapper has no music body"));
    }
    Ok(PitchResult {
        events,
        end_ref_di: Some(ref_di),
        unsupported: None,
    })
}

fn pitch_neutral_include(raw: &str) -> bool {
    matches!(
        raw,
        "\"english.ly\"" | "\"nederlands.ly\"" | "\"deutsch.ly\""
    ) || (raw.starts_with("\"predefined-guitar-") && raw.ends_with("-fretboards.ly\""))
        || raw == "\"predefined-guitar-fretboards.ly\""
}

fn resolve_simultaneous(
    source: &str,
    span: SourceSpan,
    syntax: &Syntax,
) -> Result<Option<PitchResult>, AppError> {
    let tokens = &syntax.tokens;
    let Some(first) = (0..tokens.len()).find(|&i| tokens[i].raw == "<<") else {
        return Ok(None);
    };
    let Some(last) = syntax.close[first] else {
        return Ok(None);
    };
    if tokens[..first]
        .iter()
        .chain(tokens[last + 1..].iter())
        .any(music_token)
        || tokens[..first]
            .iter()
            .any(|token| matches!(token.command(), Some("relative" | "transpose")))
    {
        return Ok(None);
    }
    let base = syntax.depth[first].saturating_add(1);
    let mut starts = vec![first + 1];
    starts.extend(
        (first + 1..last)
            .filter(|&i| syntax.depth[i] == base && tokens[i].command() == Some("\\"))
            .map(|i| i + 1),
    );
    let mut ends = starts[1..]
        .iter()
        .map(|start| start.saturating_sub(1))
        .collect::<Vec<_>>();
    ends.push(last);
    let map = SourceMap::new(source);
    let mut events = Vec::new();
    let mut unsupported = None;
    for (branch, (start, end)) in starts.into_iter().zip(ends).enumerate() {
        let Some((body_start, body_end)) = branch_body(tokens, syntax, start, end) else {
            unsupported.get_or_insert("simultaneous music branch is not a music expression".into());
            continue;
        };
        let branch_span = map.span(
            span.byte_start.saturating_add(body_start),
            span.byte_start.saturating_add(body_end),
        )?;
        let result = resolve_pitches(source, branch_span)?;
        if let Some(reason) = result.unsupported {
            unsupported.get_or_insert(reason);
        }
        for mut event in result.events {
            event.voice = Some(branch + 1);
            events.push(event);
        }
    }
    if events.is_empty() && unsupported.is_none() {
        unsupported = Some("simultaneous music has no verifiable notes".into());
    }
    Ok(Some(PitchResult {
        events,
        end_ref_di: None,
        unsupported,
    }))
}

fn music_token(token: &crate::Token) -> bool {
    matches!(
        token.kind,
        crate::TokenKind::Note
            | crate::TokenKind::Rest
            | crate::TokenKind::Skip
            | crate::TokenKind::ChordO
    )
}

fn branch_body(
    tokens: &[crate::Token],
    syntax: &Syntax,
    start: usize,
    end: usize,
) -> Option<(usize, usize)> {
    let first = (start..end).find(|&i| !tokens[i].trivia())?;
    let last = (start..end).rev().find(|&i| !tokens[i].trivia())?;
    if tokens[first].raw == "{" && tokens[last].raw == "}" && syntax.close[first] != Some(last) {
        return None;
    }
    Some((tokens[first].byte_start, tokens[last].byte_end))
}
