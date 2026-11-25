use crate::{tokenize, Mode, SourceMap, Token, TokenKind};
use ossia_types::LocationCapability;
use std::collections::HashSet;

use super::rhythm;
use super::VoiceRegion;

pub(crate) struct Syntax {
    pub(crate) tokens: Vec<Token>,
    pub(crate) close: Vec<Option<usize>>,
    pub(crate) depth: Vec<usize>,
    pub(crate) malformed: bool,
}

impl Syntax {
    pub(crate) fn new(source: &str) -> Self {
        let tokens: Vec<_> = tokenize(source)
            .into_iter()
            .filter(|t| !t.trivia())
            .collect();
        let mut close = vec![None; tokens.len()];
        let mut depth = vec![0; tokens.len()];
        let mut stack: Vec<usize> = Vec::new();
        let mut malformed = false;
        for (i, t) in tokens.iter().enumerate() {
            depth[i] = stack.len();
            if t.raw == "{" || t.raw == "<<" {
                stack.push(i);
            } else if t.raw == "}" || t.raw == ">>" {
                if let Some(open) = stack.pop() {
                    if (tokens[open].raw == "{") != (t.raw == "}") {
                        malformed = true;
                    } else {
                        close[open] = Some(i);
                    }
                } else {
                    malformed = true;
                }
            }
        }
        malformed |= !stack.is_empty();
        Self {
            tokens,
            close,
            depth,
            malformed,
        }
    }

    pub(crate) fn expression(&self, from: usize, limit: usize) -> Option<(usize, usize)> {
        let t = self.tokens.get(from)?;
        if t.raw == "{" || t.raw == "<<" {
            return self.close[from]
                .filter(|&c| c < limit)
                .map(|c| (from, c + 1));
        }
        if matches!(t.command(), Some("new" | "context")) {
            let body = self.context_body(from, limit)?;
            return self.expression(body, limit).map(|(_, end)| (from, end));
        }
        if matches!(
            t.command(),
            Some(
                "relative"
                    | "absolute"
                    | "transpose"
                    | "chordmode"
                    | "notemode"
                    | "drummode"
                    | "lyricmode"
                    | "repeat"
            )
        ) {
            for j in from + 1..limit {
                if self.tokens[j].raw == "="
                    || self.tokens[j].raw == "}"
                    || self.tokens[j].raw == ">>"
                {
                    break;
                }
                if self.tokens[j].raw == "{" || self.tokens[j].raw == "<<" {
                    return self.close[j].filter(|&c| c < limit).map(|c| (j, c + 1));
                }
            }
            return None;
        }
        (t.kind == TokenKind::Cmd).then_some((from, from + 1))
    }

    pub(crate) fn context_body(&self, from: usize, limit: usize) -> Option<usize> {
        let mut body = from.checked_add(2)?;
        if self.tokens.get(body).is_some_and(|token| token.raw == "=") {
            body = body.checked_add(2)?;
        }
        if self.tokens.get(body).and_then(Token::command) == Some("with") {
            body = self.expression(body + 1, limit)?.1;
        }
        (body < limit).then_some(body)
    }
}

#[derive(Clone)]
pub(crate) struct Assignment {
    pub(crate) name: String,
    pub(crate) start: usize,
    pub(crate) rhs: usize,
    pub(crate) open: usize,
    pub(crate) end: usize,
}

pub(crate) fn unsupported_token(t: &Token) -> bool {
    matches!(
        t.command(),
        Some(
            "chordmode"
                | "repeat"
                | "unfoldRepeats"
                | "relative-includes"
                | "lyricmode"
                | "drummode"
        )
    ) || (t.kind == TokenKind::Scheme
        && (t.raw.starts_with("#(")
            || t.raw.starts_with("$(")
            || t.raw.starts_with("#{")
            || t.raw.starts_with("#'(")))
}

pub(crate) fn region(
    source: &str,
    name: String,
    range: std::ops::Range<usize>,
    rhs: usize,
    open: usize,
    syntax: &Syntax,
    decorations: &HashSet<String>,
) -> VoiceRegion {
    let std::ops::Range { start, end } = range;
    let map = SourceMap::new(source);
    let tokens = &syntax.tokens;
    let byte_end = tokens[end - 1].byte_end;
    let span = map.span(tokens[start].byte_start, byte_end).unwrap();
    let music_span = map.span(tokens[rhs].byte_start, byte_end).unwrap();
    let body_start = if tokens[open].raw == "{" {
        tokens[open].byte_end
    } else {
        tokens[open].byte_start
    };
    let body_end = if tokens[end - 1].raw == "}" {
        tokens[end - 1].byte_start
    } else {
        byte_end
    };
    let mut repeat_boundaries = RepeatBoundaries::default();
    let mut reason = None;
    for i in rhs..end {
        let token = &tokens[i];
        if token.mode == Mode::Text || !unsupported_token(token) {
            continue;
        }
        if token.command() == Some("repeat") {
            if let Some((start, close)) = volta_body(syntax, i, end) {
                repeat_boundaries.starts.insert(tokens[start].byte_end);
                repeat_boundaries.ends.insert(tokens[close].byte_start);
                continue;
            }
        }
        reason.get_or_insert_with(|| format!("unsupported music structure: {}", token.raw));
    }
    if tokens[rhs..end].iter().any(|t| {
        t.mode != Mode::Text
            && t.command().is_some_and(|cmd| {
                !crate::pitch::known_command(cmd)
                    && !matches!(
                        cmd,
                        "markup"
                            | "compressMMRests"
                            | "notemode"
                            | "skip"
                            | "repeat"
                            | "grace"
                            | "acciaccatura"
                            | "appoggiatura"
                            | "slashedGrace"
                    )
                    && !decorations.contains(cmd)
            })
    }) {
        reason.get_or_insert("unresolved music command or variable".into());
    }
    let mut measure_groups = Vec::new();
    let base_depth = syntax.depth[open].saturating_add(1);
    let simultaneous =
        (open + 1..end - 1).find(|&i| tokens[i].raw == "<<" && syntax.depth[i] == base_depth);
    if let Some(seq) = simultaneous {
        if let Some(seq_end) = syntax.close[seq] {
            let mut starts = vec![seq + 1];
            starts.extend(
                (seq + 1..seq_end)
                    .filter(|&i| {
                        syntax.depth[i] == base_depth.saturating_add(1)
                            && tokens[i].command() == Some("\\")
                    })
                    .map(|i| i + 1),
            );
            let mut ends = starts[1..]
                .iter()
                .map(|start| start.saturating_sub(1))
                .collect::<Vec<_>>();
            ends.push(seq_end);
            for (start, end) in starts.into_iter().zip(ends) {
                let Some((body_start, body_end)) = expression_body(tokens, syntax, start, end)
                else {
                    reason.get_or_insert(
                        "simultaneous music branch is not a music expression".into(),
                    );
                    break;
                };
                let (measures, branch_reason) =
                    parse_measure_body(source, body_start, body_end, &map, &repeat_boundaries);
                if let Some(branch_reason) = branch_reason {
                    reason.get_or_insert(branch_reason);
                }
                measure_groups.push(measures);
            }
        } else {
            reason.get_or_insert("unbalanced simultaneous music".into());
        }
        if measure_groups.is_empty() {
            measure_groups.push(Vec::new());
        }
    } else {
        let (measures, body_reason) =
            parse_measure_body(source, body_start, body_end, &map, &repeat_boundaries);
        if let Some(body_reason) = body_reason {
            reason.get_or_insert(body_reason);
        }
        measure_groups.push(measures);
    }
    if reason.is_some() {
        measure_groups.iter_mut().for_each(Vec::clear);
    }
    let measures = measure_groups.iter().flatten().cloned().collect();
    VoiceRegion {
        name,
        span,
        music_span,
        measures,
        measure_groups,
        capability: if reason.is_some() {
            LocationCapability::AnchorOnly
        } else {
            LocationCapability::Measure
        },
        reason,
    }
}

#[derive(Default)]
struct RepeatBoundaries {
    starts: HashSet<usize>,
    ends: HashSet<usize>,
}

fn volta_body(syntax: &Syntax, repeat: usize, limit: usize) -> Option<(usize, usize)> {
    let start = repeat + 3;
    let [kind, count, open] = syntax.tokens.get(repeat + 1..=start)? else {
        return None;
    };
    if !matches!(kind.raw.as_str(), "volta" | "\"volta\"")
        || count.raw.parse::<u32>().ok()? < 2
        || open.raw != "{"
    {
        return None;
    }
    let close = syntax.close[start].filter(|close| *close < limit)?;
    // Folded sections use explicit bar checks to establish their internal boundaries.
    syntax.tokens[start + 1..close]
        .iter()
        .enumerate()
        .any(|(i, token)| {
            token.kind == TokenKind::Bar && syntax.depth[start + 1 + i] == syntax.depth[start] + 1
        })
        .then_some((start, close))
}

fn expression_body(
    tokens: &[Token],
    syntax: &Syntax,
    start: usize,
    end: usize,
) -> Option<(usize, usize)> {
    let first = (start..end).find(|&i| !tokens[i].trivia())?;
    let last = (start..end).rev().find(|&i| !tokens[i].trivia())?;
    if tokens[first].raw == "{" && tokens[last].raw == "}" {
        if syntax.close[first] != Some(last) {
            return None;
        }
        return Some((tokens[first].byte_end, tokens[last].byte_start));
    }
    Some((tokens[first].byte_start, tokens[last].byte_end))
}

fn parse_measure_body(
    source: &str,
    body_start: usize,
    body_end: usize,
    map: &SourceMap<'_>,
    repeats: &RepeatBoundaries,
) -> (Vec<super::Measure>, Option<String>) {
    let body = &source[body_start..body_end];
    let events = tokenize(&format!("{{ {body} }}"));
    let mut measures: Vec<super::Measure> = Vec::new();
    let mut reason = None;
    let mut from = body_start;
    let mut next = 1u32;
    // The temporary brace puts plain inline context bodies into music mode. Offsets shift by two bytes.
    let mut has_music = false;
    let mut trailing_music = false;
    let mut marked = HashSet::new();
    for (index, t) in events.iter().enumerate() {
        if t.byte_start < 2 || t.byte_end > body.len() + 2 {
            continue;
        }
        let token_start = body_start + t.byte_start - 2;
        let token_end = body_start + t.byte_end - 2;
        if repeats.starts.contains(&token_end) {
            if has_music {
                reason.get_or_insert("repeat starts without an explicit measure boundary".into());
            } else {
                from = token_end;
            }
        }
        let closes_repeat = repeats.ends.contains(&token_start);
        if closes_repeat
            && has_music
            && rhythm::fits_in_one_measure(&events[..index], from - body_start + 2) != Some(true)
        {
            reason.get_or_insert("repeat ends without a reliable measure boundary".into());
        }
        if t.mode == Mode::Music
            && matches!(
                t.kind,
                TokenKind::Note | TokenKind::Rest | TokenKind::Skip | TokenKind::ChordO
            )
        {
            has_music = true;
            trailing_music = true;
        }
        // Printed numbers can be repeated or offset by a pickup in folded music.
        if t.kind == TokenKind::Comment && t.mode == Mode::Music && repeats.starts.is_empty() {
            let marker = t.raw.strip_prefix('%').unwrap_or("").trim();
            if !marker.is_empty()
                && marker.bytes().all(|b| b.is_ascii_digit())
                && marker.parse::<u32>().is_err()
            {
                reason.get_or_insert("measure marker exceeds supported range".into());
            }
            if let Some(n) = t
                .raw
                .strip_prefix('%')
                .and_then(|s| s.trim().parse::<u32>().ok())
            {
                if let Some(last) = measures.last_mut() {
                    if n < last.measure_start {
                        reason.get_or_insert("non-monotonic measure marker".into());
                    } else {
                        last.measure_end = n;
                        next = n.saturating_add(1);
                        marked.insert(last.span.byte_start);
                        let end = body_start + t.byte_end - 2;
                        last.span = map.span(last.span.byte_start, end).unwrap();
                        from = end;
                    }
                }
            }
        }
        let boundary = if closes_repeat {
            Some(t.byte_start)
        } else if t.kind == TokenKind::Bar && t.mode == Mode::Music {
            Some(t.byte_end)
        } else if t.command() == Some("bar") && t.mode != Mode::Text {
            Some(
                events[index + 1..]
                    .iter()
                    .find(|next| !next.trivia())
                    .map_or(t.byte_end, |next| next.byte_end),
            )
        } else {
            None
        };
        if let Some(boundary) = boundary.filter(|_| has_music) {
            let to = body_start + boundary - 2;
            measures.push(super::Measure {
                measure_start: next,
                measure_end: next,
                span: map.span(from, to).unwrap(),
            });
            next = next.saturating_add(1);
            from = to;
            trailing_music = false;
            has_music = false;
        }
        if closes_repeat {
            from = token_end;
        }
    }
    if trailing_music && has_music {
        if let Some(boundaries) = rhythm::implicit_measure_boundaries(&events, body_start, from) {
            for boundary in boundaries {
                measures.push(super::Measure {
                    measure_start: next,
                    measure_end: next,
                    span: map.span(from, boundary).unwrap(),
                });
                next = next.saturating_add(1);
                from = boundary;
            }
            trailing_music = false;
        }
    }
    if measures.is_empty() || trailing_music {
        reason.get_or_insert("music without explicit complete measure boundaries".into());
    }
    let measures = expand_scaled_measures(&events, body_start, map, measures, &marked)
        .unwrap_or_else(|| {
            reason.get_or_insert("ambiguous multiplied rest or skip".into());
            Vec::new()
        });
    (measures, reason)
}

fn expand_scaled_measures(
    events: &[Token],
    body_start: usize,
    map: &SourceMap<'_>,
    measures: Vec<super::Measure>,
    marked: &HashSet<usize>,
) -> Option<Vec<super::Measure>> {
    let tokens = events.iter().filter(|t| !t.trivia()).collect::<Vec<_>>();
    let scaled = tokens
        .windows(2)
        .filter(|w| {
            matches!(w[0].kind, TokenKind::Rest | TokenKind::Skip)
                && w[0].mode == Mode::Music
                && w[1].kind == TokenKind::Star
        })
        .map(|w| w[0].byte_start)
        .collect::<Vec<_>>();
    if scaled.is_empty() {
        return Some(measures);
    }
    let mut result = Vec::new();
    let mut next = 1u32;
    for measure in measures {
        let mut from = measure.span.byte_start;
        let to = measure.span.byte_end;
        let start = from - body_start + 2;
        let end = to - body_start + 2;
        let scaled_here = scaled
            .get(scaled.partition_point(|byte| *byte < start))
            .is_some_and(|byte| *byte < end);
        if scaled_here && !marked.contains(&from) {
            for (boundary, count) in rhythm::scaled_measure_boundaries(events, start, end)? {
                let boundary = body_start + boundary - 2;
                let last = next.checked_add(count - 1)?;
                result.push(super::Measure {
                    measure_start: next,
                    measure_end: last,
                    span: map.span(from, boundary).ok()?,
                });
                next = last.checked_add(1)?;
                from = boundary;
            }
        } else {
            let last = if marked.contains(&from) {
                measure.measure_end
            } else {
                next
            };
            if last < next {
                return None;
            }
            result.push(super::Measure {
                measure_start: next,
                measure_end: last,
                span: measure.span,
            });
            next = last.checked_add(1)?;
        }
    }
    Some(result)
}
