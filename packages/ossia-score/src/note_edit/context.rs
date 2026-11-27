//! The scanned source around a selection: events, attached notation and pitches.
use super::*;

pub(super) const DYNAMICS: [&str; 10] =
    ["ppp", "pp", "p", "mp", "mf", "f", "ff", "fff", "sf", "sfz"];
pub(super) const HAIRPINS: [&str; 3] = ["<", ">", "!"];
pub(super) const POST_COMMANDS: [&str; 16] = [
    "fermata",
    "trill",
    "prall",
    "mordent",
    "turn",
    "staccato",
    "staccatissimo",
    "accent",
    "tenuto",
    "marcato",
    "arpeggio",
    "glissando",
    "sustainOn",
    "sustainOff",
    "(",
    ")",
];

pub(super) fn invalid(message: &str) -> AppError {
    AppError::invalid(message)
}

pub(super) struct Context<'a> {
    pub(super) source: &'a str,
    pub(super) tokens: Vec<Token>,
    pub(super) events: Vec<Event>,
    pub(super) language: pitch::NoteLanguage,
}

impl<'a> Context<'a> {
    pub(super) fn new(source: &'a str) -> Result<Self, AppError> {
        let tokens: Vec<Token> = tokenize(source)
            .into_iter()
            .filter(|t| !matches!(t.kind, TokenKind::Ws | TokenKind::Comment))
            .collect();
        let events = scan(source)?;
        let language = pitch::language_before(&tokens, source.len())
            .ok_or_else(|| AppError::unsupported("the note language is not supported"))?;
        Ok(Self {
            source,
            tokens,
            events,
            language,
        })
    }

    /// End of the notation attached to the event ending at `end`: ties, articulations, slurs,
    /// dynamics and beams.
    pub(super) fn post_end(&self, end: usize) -> usize {
        let mut last = end;
        for token in self.tokens.iter().filter(|t| t.byte_start >= end) {
            let attached = match token.kind {
                TokenKind::Tie | TokenKind::Fing | TokenKind::Artic | TokenKind::Beam => true,
                TokenKind::Num => self.source[..token.byte_start].ends_with(':'),
                TokenKind::Raw => matches!(token.raw.as_str(), "(" | ")"),
                TokenKind::Cmd => {
                    let name = &token.raw[1..];
                    DYNAMICS.contains(&name)
                        || HAIRPINS.contains(&name)
                        || POST_COMMANDS.contains(&name)
                }
                _ => false,
            };
            if !attached || token.mode != Mode::Music {
                break;
            }
            last = token.byte_end;
        }
        last
    }

    pub(super) fn post_tokens(&self, event: &Event) -> Vec<&Token> {
        let end = self.post_end(event.full_end);
        self.tokens
            .iter()
            .filter(|t| t.byte_start >= event.full_end && t.byte_end <= end)
            .collect()
    }

    pub(super) fn has_bar_between(&self, start: usize, end: usize) -> bool {
        self.tokens
            .iter()
            .any(|t| t.kind == TokenKind::Bar && t.byte_start >= start && t.byte_end <= end)
    }

    /// The pitch a typed letter starts from: the previous note, else the staff's middle.
    pub(super) fn previous_di(&self, before: usize, analysis: &[pitch::PitchEvent]) -> i32 {
        if let Some(event) = analysis
            .iter()
            .rev()
            .find(|e| e.span.byte_start < before && !e.members.is_empty())
        {
            return event.members[0].di;
        }
        let clef = self
            .tokens
            .iter()
            .enumerate()
            .rfind(|(_, t)| t.byte_start < before && t.raw == "\\clef")
            .and_then(|(i, _)| self.tokens.get(i + 1))
            .map(|t| t.raw.trim_matches('"').to_owned());
        match clef.as_deref() {
            Some("bass" | "F") => 1,
            Some("alto" | "C") => 7,
            Some("tenor") => 5,
            _ => 13,
        }
    }
}

/// Pitches of the top-level expression (an assignment or a `\score` block) containing `at`.
pub(super) fn analysis(source: &str, at: usize) -> Result<Vec<pitch::PitchEvent>, AppError> {
    let tokens: Vec<Token> = tokenize(source)
        .into_iter()
        .filter(|t| !t.trivia())
        .collect();
    let (start, end) = top_level_expressions(&tokens)
        .into_iter()
        .find(|(start, end)| *start <= at && at < *end)
        .ok_or_else(|| invalid("the selection is not inside the score's music"))?;
    let map = SourceMap::new(source);
    let result = pitch::resolve_written_pitches(source, map.span(start, end)?)?;
    if let Some(reason) = result.unsupported {
        return Err(AppError::unsupported(format!(
            "note input does not support this passage: {reason}"
        )));
    }
    Ok(result.events)
}

/// Byte ranges of each top-level assignment value and `\score` block.
pub(crate) fn top_level_expressions(tokens: &[Token]) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < tokens.len() {
        let t = &tokens[i];
        let starts = t.mode == Mode::Top && ((t.raw == "=" && i > 0) || t.raw == "\\score");
        if !starts {
            i += 1;
            continue;
        }
        let first = i + 1;
        let Some(open) = (first..tokens.len())
            .take_while(|&j| j - first < 8)
            .find(|&j| matches!(tokens[j].raw.as_str(), "{" | "<<"))
        else {
            i += 1;
            continue;
        };
        let mut depth = 0i32;
        let mut close = open;
        for (j, token) in tokens.iter().enumerate().skip(open) {
            match token.raw.as_str() {
                "{" | "<<" => depth += 1,
                "}" | ">>" => depth -= 1,
                _ => {}
            }
            if depth == 0 {
                close = j;
                break;
            }
        }
        let start = if t.raw == "\\score" { open } else { first };
        out.push((tokens[start].byte_start, tokens[close].byte_end));
        i = close + 1;
    }
    out
}

/// Find the event at a byte offset, and the head inside it when the offset is on one.
pub(super) fn locate(events: &[Event], at: usize) -> Result<(usize, Option<usize>), AppError> {
    let index = events
        .iter()
        .position(|e| e.start <= at && at < e.full_end.max(e.end))
        .ok_or_else(|| invalid("select a note, chord or rest"))?;
    let head = events[index]
        .members
        .iter()
        .position(|(start, _, end)| *start <= at && at < *end);
    Ok((index, head))
}

pub(super) fn head_pitches(
    analysis: &[pitch::PitchEvent],
    event: &Event,
) -> Result<Vec<Spelled>, AppError> {
    let found = analysis
        .iter()
        .find(|e| e.span.byte_start == event.start)
        .ok_or_else(|| AppError::unsupported("the selected notes could not be analysed"))?;
    Ok(found
        .members
        .iter()
        .map(|m| Spelled {
            di: m.di,
            alter: m.alter,
        })
        .collect())
}

/// Duration text of an event, written or empty.
pub(super) fn written(ctx: &Context, event: &Event) -> String {
    event
        .written
        .map(|(s, e)| ctx.source[s..e].to_owned())
        .unwrap_or_default()
}
