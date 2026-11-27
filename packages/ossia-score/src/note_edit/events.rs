//! Rhythmic events of a LilyPond source as note input sees them.
use crate::{tokenize, Mode, Token, TokenKind};
use ossia_types::AppError;

/// A non-negative duration as a fraction of a whole note.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Frac(pub(crate) u64, pub(crate) u64);

fn gcd(a: u64, b: u64) -> u64 {
    if b == 0 {
        a
    } else {
        gcd(b, a % b)
    }
}

impl PartialOrd for Frac {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Frac {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (self.0 * other.1).cmp(&(other.0 * self.1))
    }
}

impl Frac {
    pub(crate) fn new(n: u64, d: u64) -> Self {
        let g = gcd(n, d).max(1);
        Self(n / g, d / g)
    }
    pub(crate) fn add(self, o: Self) -> Self {
        Self::new(self.0 * o.1 + o.0 * self.1, self.1 * o.1)
    }
    /// `self - o`, or `None` when `o` is longer.
    pub(crate) fn sub(self, o: Self) -> Option<Self> {
        let (a, b) = (self.0 * o.1, o.0 * self.1);
        (a >= b).then(|| Self::new(a - b, self.1 * o.1))
    }
    pub(crate) fn mul(self, o: Self) -> Self {
        Self::new(self.0 * o.0, self.1 * o.1)
    }
    pub(crate) fn is_zero(self) -> bool {
        self.0 == 0
    }
}

/// A written note value: `log` 0 is a whole note, 6 a sixty-fourth.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NoteValue {
    pub log: u8,
    pub dots: u8,
}

impl NoteValue {
    pub(crate) fn length(self) -> Frac {
        let base = 1u64 << self.log;
        // A dotted value adds halves: 1 + 1/2 + 1/4 ... = (2^(d+1) - 1) / 2^d.
        Frac::new((1 << (self.dots + 1)) - 1, base << self.dots)
    }
    pub(crate) fn text(self) -> String {
        format!("{}{}", 1u32 << self.log, ".".repeat(self.dots.into()))
    }
    fn parse(text: &str) -> Option<Self> {
        let digits: String = text.chars().take_while(char::is_ascii_digit).collect();
        let rest = &text[digits.len()..];
        if digits.is_empty() || !rest.chars().all(|c| c == '.') {
            return None;
        }
        let value: u32 = digits.parse().ok()?;
        let log = (0..=7u8).find(|log| 1u32 << log == value)?;
        Some(Self {
            log,
            dots: u8::try_from(rest.len()).ok()?,
        })
    }
}

/// Split a length into written values, longest first; dotted values are used when they fit.
pub(crate) fn values_for(mut length: Frac) -> Option<Vec<NoteValue>> {
    let mut out = Vec::new();
    while !length.is_zero() {
        let value = (0..=7u8)
            .flat_map(|log| {
                [1u8, 0]
                    .into_iter()
                    .map(move |dots| NoteValue { log, dots })
            })
            .find(|value| length.sub(value.length()).is_some())?;
        length = length.sub(value.length())?;
        out.push(value);
    }
    Some(out)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EventKind {
    Note,
    Chord,
    Rest,
    Skip,
    /// `R` full-measure rest, possibly multiplied.
    MeasureRest,
}

/// A pitch token: start, end of the pitch name and octave marks, end of the token.
pub(crate) type Member = (usize, usize, usize);

/// One rhythmic event in source order.
#[derive(Debug, Clone)]
pub(crate) struct Event {
    pub(crate) kind: EventKind,
    pub(crate) start: usize,
    /// End of the note, rest or chord-close token, before any multiplier.
    pub(crate) end: usize,
    /// End including a `*n` multiplier.
    pub(crate) full_end: usize,
    /// Pitch tokens: start, end of the pitch name and octave marks, end of the token.
    pub(crate) members: Vec<Member>,
    /// Byte range of the written duration digits and dots, when written.
    pub(crate) written: Option<(usize, usize)>,
    /// Length of one occurrence before multipliers.
    pub(crate) value: Frac,
    /// Multipliers after the value: scale and repeat count.
    pub(crate) scale: Frac,
    pub(crate) count: u64,
    pub(crate) in_tuplet: bool,
    /// Brace block this event belongs to, as the byte offset of its `{`.
    pub(crate) block: usize,
}

impl Event {
    pub(crate) fn length(&self) -> Frac {
        self.value.mul(self.scale).mul(Frac::new(self.count, 1))
    }
    pub(crate) fn pitched(&self) -> bool {
        matches!(self.kind, EventKind::Note | EventKind::Chord)
    }
}

/// End of a pitch name with its octave marks and cautionary accidental at `start`.
fn pitch_end(source: &str, start: usize) -> usize {
    let bytes = source.as_bytes();
    let mut i = start;
    while i < bytes.len() && bytes[i].is_ascii_lowercase() {
        i += 1;
    }
    while i < bytes.len() && matches!(bytes[i], b'\'' | b',' | b'!' | b'?') {
        i += 1;
    }
    i
}

fn multiplier(tokens: &[Token], mut i: usize) -> Option<(Frac, u64, usize, usize)> {
    let mut factors = Vec::new();
    let mut end = 0;
    while tokens.get(i).is_some_and(|t| t.kind == TokenKind::Star) {
        let number = tokens.get(i + 1).filter(|t| t.kind == TokenKind::Num)?;
        let (n, d) = number.raw.split_once('/').unwrap_or((&number.raw, "1"));
        factors.push(Frac::new(n.parse().ok()?, d.parse().ok()?));
        end = number.byte_end;
        i += 2;
    }
    if factors.is_empty() {
        return None;
    }
    // A trailing whole number repeats the event; anything before it scales the value.
    let last = *factors.last()?;
    let (scale, count) = if last.1 == 1 {
        (
            factors[..factors.len() - 1]
                .iter()
                .fold(Frac::new(1, 1), |a, f| a.mul(*f)),
            last.0,
        )
    } else {
        (factors.iter().fold(Frac::new(1, 1), |a, f| a.mul(*f)), 1)
    };
    Some((scale, count, end, i))
}

/// Pitches written as arguments of commands, such as `\key g` or `\relative c''`.
fn is_pitch_argument(tokens: &[Token], i: usize) -> bool {
    const ONE: [&str; 5] = [
        "\\key",
        "\\relative",
        "\\fixed",
        "\\octaveCheck",
        "\\transposition",
    ];
    if tokens[i].kind != TokenKind::Note {
        return false;
    }
    let command = |k: usize| tokens.get(k).map(|t| t.raw.as_str());
    (i >= 1 && command(i - 1).is_some_and(|c| ONE.contains(&c) || c == "\\transpose"))
        || (i >= 2 && command(i - 2) == Some("\\transpose"))
}

/// Scan every rhythmic event, resolving durations that are carried over from earlier events.
pub(crate) fn scan(source: &str) -> Result<Vec<Event>, AppError> {
    let tokens: Vec<Token> = tokenize(source)
        .into_iter()
        .filter(|t| !matches!(t.kind, TokenKind::Ws | TokenKind::Comment))
        .collect();
    let mut events = Vec::new();
    let mut current = NoteValue { log: 2, dots: 0 };
    let mut blocks: Vec<(usize, bool)> = Vec::new();
    let mut tuplet_pending = false;
    let mut chord: Option<(usize, Vec<Member>)> = None;
    let mut i = 0;
    while i < tokens.len() {
        let t = &tokens[i];
        if t.kind == TokenKind::Cmd && matches!(t.raw.as_str(), "\\tuplet" | "\\times") {
            tuplet_pending = true;
        }
        if t.kind == TokenKind::Brace {
            if t.raw == "{" {
                let outer = blocks.last().is_some_and(|b| b.1);
                blocks.push((t.byte_start, outer || tuplet_pending));
                tuplet_pending = false;
            } else {
                blocks.pop();
            }
            i += 1;
            continue;
        }
        if t.mode != Mode::Music {
            i += 1;
            continue;
        }
        if is_pitch_argument(&tokens, i) {
            i += 1;
            continue;
        }
        let (block, in_tuplet) = blocks.last().copied().unwrap_or((0, false));
        match t.kind {
            TokenKind::ChordO => chord = Some((t.byte_start, Vec::new())),
            TokenKind::Note if chord.is_some() => {
                let end = pitch_end(source, t.byte_start).min(t.byte_end);
                if let Some((_, members)) = &mut chord {
                    members.push((t.byte_start, end, t.byte_end));
                }
            }
            TokenKind::ChordC | TokenKind::Note | TokenKind::Rest | TokenKind::Skip => {
                let (kind, start, members, text_start) = match t.kind {
                    TokenKind::ChordC => {
                        let (start, members) = chord
                            .take()
                            .ok_or_else(|| AppError::invalid("unmatched chord close"))?;
                        (EventKind::Chord, start, members, t.byte_start + 1)
                    }
                    TokenKind::Note => {
                        let end = pitch_end(source, t.byte_start).min(t.byte_end);
                        (
                            EventKind::Note,
                            t.byte_start,
                            vec![(t.byte_start, end, t.byte_end)],
                            end,
                        )
                    }
                    TokenKind::Rest if t.raw.starts_with('R') => (
                        EventKind::MeasureRest,
                        t.byte_start,
                        vec![],
                        t.byte_start + 1,
                    ),
                    TokenKind::Rest => (EventKind::Rest, t.byte_start, vec![], t.byte_start + 1),
                    _ => (EventKind::Skip, t.byte_start, vec![], t.byte_start + 1),
                };
                let text = &source[text_start..t.byte_end];
                let written = if text.is_empty() {
                    None
                } else {
                    current = NoteValue::parse(text).ok_or_else(|| {
                        AppError::unsupported(format!("unsupported duration: {}", t.raw))
                    })?;
                    Some((text_start, t.byte_end))
                };
                let (scale, count, full_end, next) =
                    multiplier(&tokens, i + 1).unwrap_or((Frac::new(1, 1), 1, t.byte_end, i + 1));
                events.push(Event {
                    kind,
                    start,
                    end: t.byte_end,
                    full_end,
                    members,
                    written,
                    value: current.length(),
                    scale,
                    count,
                    in_tuplet,
                    block,
                });
                i = next;
                continue;
            }
            _ => {}
        }
        i += 1;
    }
    Ok(events)
}
