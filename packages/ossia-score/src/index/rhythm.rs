use crate::{Mode, Token, TokenKind};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct MeterFraction {
    numerator: u64,
    denominator: u64,
}

impl MeterFraction {
    fn new(numerator: u64, denominator: u64) -> Option<Self> {
        if denominator == 0 {
            return None;
        }
        let divisor = gcd(numerator, denominator);
        Some(Self {
            numerator: numerator / divisor,
            denominator: denominator / divisor,
        })
    }

    fn add(self, other: Self) -> Option<Self> {
        let numerator = u128::from(self.numerator) * u128::from(other.denominator)
            + u128::from(other.numerator) * u128::from(self.denominator);
        let denominator = u128::from(self.denominator) * u128::from(other.denominator);
        Self::new(numerator.try_into().ok()?, denominator.try_into().ok()?)
    }

    fn multiply(self, other: Self) -> Option<Self> {
        let numerator = u128::from(self.numerator) * u128::from(other.numerator);
        let denominator = u128::from(self.denominator) * u128::from(other.denominator);
        Self::new(numerator.try_into().ok()?, denominator.try_into().ok()?)
    }

    fn exceeds(self, other: Self) -> bool {
        u128::from(self.numerator) * u128::from(other.denominator)
            > u128::from(other.numerator) * u128::from(self.denominator)
    }
}

fn gcd(mut left: u64, mut right: u64) -> u64 {
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    left
}

fn fraction(raw: &str) -> Option<MeterFraction> {
    let (numerator, denominator) = raw
        .split_once('/')
        .map_or((raw, "1"), |(numerator, denominator)| {
            (numerator.trim(), denominator.trim())
        });
    MeterFraction::new(numerator.parse().ok()?, denominator.parse().ok()?)
}

fn duration_number(raw: &str) -> Option<MeterFraction> {
    let (base, dots) = raw.split_once('.').map_or((raw, 0), |(base, dots)| {
        (base, dots.bytes().filter(|byte| *byte == b'.').count() + 1)
    });
    if dots == 0 {
        return MeterFraction::new(1, base.parse().ok()?);
    }
    let dots = u32::try_from(dots).ok()?;
    let denominator = u64::try_from(1u128.checked_shl(dots)?).ok()?;
    let numerator = u64::try_from((1u128.checked_shl(dots + 1)?) - 1).ok()?;
    let base = MeterFraction::new(1, base.parse().ok()?)?;
    base.multiply(MeterFraction::new(numerator, denominator)?)
}

fn note_duration(raw: &str) -> Option<MeterFraction> {
    let suffix_start = raw
        .rfind(|character: char| !character.is_ascii_digit() && character != '.')
        .map_or(0, |index| index + 1);
    duration_number(&raw[suffix_start..])
}

fn scaled_duration(
    events: &[Token],
    index: usize,
    base: MeterFraction,
) -> Option<(MeterFraction, usize, usize)> {
    let Some(star) = events
        .get(index..)
        .and_then(|tokens| tokens.iter().position(|token| !token.trivia()))
        .map(|offset| index + offset)
        .filter(|star| events[*star].kind == TokenKind::Star)
    else {
        return Some((base, events.get(index - 1)?.byte_end, index - 1));
    };
    let number_index = events[star + 1..]
        .iter()
        .position(|token| !token.trivia())
        .map(|offset| star + 1 + offset)?;
    let multiplier = fraction(&events[number_index].raw)?;
    Some((
        base.multiply(multiplier)?,
        events[number_index].byte_end,
        number_index,
    ))
}

#[derive(Clone, Copy)]
struct RhythmicAtom {
    byte_start: usize,
    byte_end: usize,
    duration: MeterFraction,
}

struct RhythmScan {
    atoms: Vec<RhythmicAtom>,
    time_signatures: Vec<(usize, MeterFraction)>,
}

fn rhythmic_atoms(events: &[Token]) -> Option<RhythmScan> {
    let mut atoms = Vec::new();
    let mut time_signatures = Vec::new();
    let mut duration = MeterFraction::new(1, 4)?;
    let mut index = 0;
    while index < events.len() {
        let token = &events[index];
        if token.trivia() || token.mode != Mode::Music {
            index += 1;
            continue;
        }
        if token.command() == Some("time") {
            let signature = events[index + 1..]
                .iter()
                .find(|next| !next.trivia())
                .and_then(|signature| fraction(&signature.raw))?;
            time_signatures.push((token.byte_start, signature));
            index += 1;
            continue;
        }
        if token.command().is_some_and(|command| {
            matches!(
                command,
                "partial"
                    | "tuplet"
                    | "times"
                    | "scaleDurations"
                    | "grace"
                    | "acciaccatura"
                    | "appoggiatura"
                    | "slashedGrace"
                    | "afterGrace"
                    | "cadenzaOn"
                    | "cadenzaOff"
                    | "breve"
                    | "longa"
                    | "maxima"
                    | "alternative"
            )
        }) {
            return None;
        }
        if token.command() == Some("skip") {
            let (number_index, number) = events
                .iter()
                .enumerate()
                .skip(index + 1)
                .find(|(_, next)| !next.trivia())
                .filter(|(_, next)| next.kind == TokenKind::Num)?;
            duration = duration_number(&number.raw)?;
            let (duration, byte_end, last_index) =
                scaled_duration(events, number_index + 1, duration)?;
            atoms.push(RhythmicAtom {
                byte_start: token.byte_start,
                byte_end,
                duration,
            });
            index = last_index + 1;
            continue;
        }
        if token.kind == TokenKind::ChordO {
            let close = events[index + 1..]
                .iter()
                .position(|next| next.kind == TokenKind::ChordC)
                .map(|offset| index + 1 + offset)?;
            duration = note_duration(&events[close].raw).unwrap_or(duration);
            let (duration, byte_end, last_index) = scaled_duration(events, close + 1, duration)?;
            atoms.push(RhythmicAtom {
                byte_start: token.byte_start,
                byte_end,
                duration,
            });
            index = last_index + 1;
            continue;
        }
        if matches!(
            token.kind,
            TokenKind::Note | TokenKind::Rest | TokenKind::Skip
        ) {
            duration = note_duration(&token.raw).unwrap_or(duration);
            let (duration, byte_end, last_index) = scaled_duration(events, index + 1, duration)?;
            atoms.push(RhythmicAtom {
                byte_start: token.byte_start,
                byte_end,
                duration,
            });
            index = last_index + 1;
            continue;
        }
        index += 1;
    }
    Some(RhythmScan {
        atoms,
        time_signatures,
    })
}

pub(super) fn implicit_measure_boundaries(
    events: &[Token],
    body_start: usize,
    from: usize,
) -> Option<Vec<usize>> {
    let RhythmScan {
        atoms,
        time_signatures,
    } = rhythmic_atoms(events)?;
    let relative_start = from.checked_sub(body_start)?.checked_add(2)?;
    let tail: Vec<_> = atoms
        .into_iter()
        .filter(|atom| atom.byte_end > relative_start)
        .collect();
    let first_tail = tail.first()?;
    if first_tail.byte_start < relative_start {
        return None;
    }
    let mut measure = MeterFraction::new(1, 1)?;
    for (position, signature) in time_signatures {
        if position < first_tail.byte_start {
            measure = signature;
        } else {
            return None;
        }
    }
    let mut elapsed = MeterFraction::new(0, 1)?;
    let mut boundaries = Vec::new();
    for atom in tail {
        elapsed = elapsed.add(atom.duration)?;
        if elapsed == measure {
            boundaries.push(body_start.checked_add(atom.byte_end)?.checked_sub(2)?);
            elapsed = MeterFraction::new(0, 1)?;
        } else if elapsed.exceeds(measure) {
            return None;
        }
    }
    (elapsed.numerator == 0 && !boundaries.is_empty()).then_some(boundaries)
}
