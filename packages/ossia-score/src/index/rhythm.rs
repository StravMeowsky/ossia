use crate::{Mode, Token, TokenKind};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct MeterFraction {
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
    mut duration: MeterFraction,
) -> Option<(MeterFraction, usize, usize)> {
    let mut last = index - 1;
    while let Some(star) = events[last + 1..]
        .iter()
        .position(|token| !token.trivia())
        .map(|offset| last + 1 + offset)
        .filter(|star| events[*star].kind == TokenKind::Star)
    {
        last = events[star + 1..]
            .iter()
            .position(|token| !token.trivia())?
            + star
            + 1;
        duration = duration.multiply(fraction(&events[last].raw)?)?;
    }
    Some((duration, events[last].byte_end, last))
}

#[derive(Clone, Copy)]
struct RhythmicAtom {
    byte_start: usize,
    byte_end: usize,
    duration: MeterFraction,
    compact: bool,
    explicit: bool,
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
            let (length, byte_end, last_index) =
                scaled_duration(events, number_index + 1, duration_number(&number.raw)?)?;
            duration = length;
            atoms.push(RhythmicAtom {
                byte_start: token.byte_start,
                byte_end,
                duration,
                compact: true,
                explicit: true,
            });
            index = last_index + 1;
            continue;
        }
        if token.kind == TokenKind::ChordO {
            let close = events[index + 1..]
                .iter()
                .position(|next| next.kind == TokenKind::ChordC)
                .map(|offset| index + 1 + offset)?;
            let written = note_duration(&events[close].raw);
            let (length, byte_end, last_index) =
                scaled_duration(events, close + 1, written.unwrap_or(duration))?;
            duration = length;
            atoms.push(RhythmicAtom {
                byte_start: token.byte_start,
                byte_end,
                duration,
                compact: false,
                explicit: written.is_some(),
            });
            index = last_index + 1;
            continue;
        }
        if matches!(
            token.kind,
            TokenKind::Note | TokenKind::Rest | TokenKind::Skip
        ) {
            let written = note_duration(&token.raw);
            let (length, byte_end, last_index) =
                scaled_duration(events, index + 1, written.unwrap_or(duration))?;
            duration = length;
            atoms.push(RhythmicAtom {
                byte_start: token.byte_start,
                byte_end,
                duration,
                compact: token.kind == TokenKind::Skip || token.raw.starts_with('R'),
                explicit: written.is_some(),
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

/// The measure length every voice inherits when the score has at most one
/// distinct time signature. LilyPond defaults to 4/4 without `\time`. Mixed
/// meters or a hand-set `measureLength` make it depend on position, so it stays unknown.
pub(super) fn inherited_meter(tokens: &[Token]) -> Option<MeterFraction> {
    let mut meter = None;
    for (index, token) in tokens.iter().enumerate() {
        if token.raw.contains("measureLength") {
            return None;
        }
        if token.command() != Some("time") {
            continue;
        }
        let signature = fraction(&tokens[index + 1..].iter().find(|t| !t.trivia())?.raw)?;
        match meter {
            None => meter = Some(signature),
            Some(known) if known != signature => return None,
            Some(_) => {}
        }
    }
    meter.or_else(|| MeterFraction::new(1, 1))
}

/// Measure length at `before`: the voice's own latest `\time`, else the inherited meter.
fn meter_before(
    events: &[Token],
    before: usize,
    inherited: Option<MeterFraction>,
) -> Option<Option<MeterFraction>> {
    let previous = events[..before]
        .iter()
        .rposition(|token| token.mode == Mode::Music && token.command() == Some("time"));
    Some(match previous {
        Some(time) => Some(fraction(
            &events[time + 1..].iter().find(|t| !t.trivia())?.raw,
        )?),
        None => inherited,
    })
}

pub(super) fn implicit_measure_boundaries(
    events: &[Token],
    body_start: usize,
    from: usize,
    inherited: Option<MeterFraction>,
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
    let mut explicitly_timed = inherited.is_some();
    if let Some(meter) = inherited {
        measure = meter;
    }
    for (position, signature) in time_signatures {
        if position < first_tail.byte_start {
            measure = signature;
            explicitly_timed = true;
        } else {
            return None;
        }
    }
    let zero = MeterFraction::new(0, 1)?;
    let mut elapsed = zero;
    let mut boundaries = Vec::new();
    for atom in tail {
        if atom.compact && atom.explicit && elapsed == zero && atom.duration.exceeds(measure) {
            // A multiplied full-bar rest or skip; expand_scaled_measures splits it later.
            let ratio = atom
                .duration
                .multiply(MeterFraction::new(measure.denominator, measure.numerator)?)?;
            if !explicitly_timed || ratio.denominator != 1 {
                return None;
            }
            boundaries.push(body_start.checked_add(atom.byte_end)?.checked_sub(2)?);
            continue;
        }
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

pub(super) fn fits_in_one_measure(events: &[Token], from: usize) -> Option<bool> {
    let start = events.partition_point(|token| token.byte_start < from);
    let tail = &events[start..];
    let scan = rhythmic_atoms(tail)?;
    let first = scan.atoms.first()?;
    if !first.explicit {
        return None;
    }
    let mut measure = MeterFraction::new(1, 1)?;
    if let Some(time) = events[..start]
        .iter()
        .rposition(|token| token.command() == Some("time"))
    {
        let signature = events[time + 1..].iter().find(|token| !token.trivia())?;
        measure = fraction(&signature.raw)?;
    }
    for &(position, signature) in &scan.time_signatures {
        if position >= first.byte_start {
            return None;
        }
        measure = signature;
    }
    let length = scan
        .atoms
        .into_iter()
        .try_fold(MeterFraction::new(0, 1)?, |sum, atom| {
            sum.add(atom.duration)
        })?;
    Some(!length.exceeds(measure))
}

/// Split an explicitly bounded source interval at complete measures. A single
/// full-bar rest or skip can be the source for several consecutive measures.
pub(super) fn scaled_measure_boundaries(
    events: &[Token],
    from: usize,
    to: usize,
    inherited: Option<MeterFraction>,
) -> Option<Vec<(usize, u32)>> {
    let start = events.partition_point(|token| token.byte_start < from);
    let end = events.partition_point(|token| token.byte_end <= to);
    let scan = rhythmic_atoms(events.get(start..end)?)?;
    let first = scan.atoms.first()?;
    if !first.explicit {
        return None;
    }
    let known = meter_before(events, start, inherited)?;
    let mut meter = known.or_else(|| MeterFraction::new(1, 1))?;
    let explicitly_timed = known.is_some() || !scan.time_signatures.is_empty();
    for (position, signature) in scan.time_signatures {
        if position >= first.byte_start {
            return None;
        }
        meter = signature;
    }
    let zero = MeterFraction::new(0, 1)?;
    let mut elapsed = zero;
    let mut boundaries = Vec::new();
    for atom in scan.atoms {
        if atom.duration.exceeds(meter) {
            if !explicitly_timed || !atom.compact || elapsed != zero {
                return None;
            }
            let ratio = atom
                .duration
                .multiply(MeterFraction::new(meter.denominator, meter.numerator)?)?;
            if ratio.denominator != 1 {
                return None;
            }
            boundaries.push((atom.byte_end, u32::try_from(ratio.numerator).ok()?));
        } else {
            elapsed = elapsed.add(atom.duration)?;
            if elapsed.exceeds(meter) {
                return None;
            }
            if elapsed == meter {
                boundaries.push((atom.byte_end, 1));
                elapsed = zero;
            }
        }
    }
    match boundaries.last_mut() {
        Some(last) if elapsed == zero => last.0 = to,
        None if elapsed != zero => boundaries.push((to, 1)),
        _ => return None,
    }
    Some(boundaries)
}
