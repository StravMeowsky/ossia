//! Pitch names, key signatures and octave choice for note input.
use crate::pitch::NoteLanguage;
use crate::{Mode, Token, TokenKind};

/// A spelled pitch: diatonic index (`c` = 0, `c'` = 7) and alteration in semitones.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Spelled {
    pub(crate) di: i32,
    pub(crate) alter: i32,
}

const STEP_SEMITONES: [i32; 7] = [0, 2, 4, 5, 7, 9, 11];

impl Spelled {
    pub(crate) fn midi(self) -> i32 {
        48 + 12 * self.di.div_euclid(7)
            + STEP_SEMITONES[self.di.rem_euclid(7) as usize]
            + self.alter
    }
    pub(crate) fn step(self) -> i32 {
        self.di.rem_euclid(7)
    }
}

/// The name of a pitch without octave marks.
pub(crate) fn name(step: i32, alter: i32, language: NoteLanguage) -> String {
    let letter = ["c", "d", "e", "f", "g", "a", "b"][step.rem_euclid(7) as usize];
    match language {
        NoteLanguage::English => {
            let suffix = match alter {
                -2 => "ff",
                -1 => "f",
                1 => "s",
                2 => "ss",
                _ => "",
            };
            format!("{letter}{suffix}")
        }
        NoteLanguage::Nederlands | NoteLanguage::Deutsch => {
            let german = language == NoteLanguage::Deutsch;
            if step == 6 && german {
                return match alter {
                    -2 => "heses".into(),
                    -1 => "b".into(),
                    1 => "his".into(),
                    2 => "hisis".into(),
                    _ => "h".into(),
                };
            }
            match (letter, alter) {
                ("e", -1) => "es".into(),
                ("e", -2) => "eses".into(),
                ("a", -1) => "as".into(),
                ("a", -2) => "asas".into(),
                (_, -2) => format!("{letter}eses"),
                (_, -1) => format!("{letter}es"),
                (_, 1) => format!("{letter}is"),
                (_, 2) => format!("{letter}isis"),
                _ => letter.into(),
            }
        }
    }
}

/// Octave marks as written text.
pub(crate) fn marks(count: i32) -> String {
    if count >= 0 {
        "'".repeat(count as usize)
    } else {
        ",".repeat((-count) as usize)
    }
}

/// Fifths of the last `\key` in music before `before`; C major when there is none.
pub(crate) fn key_fifths(tokens: &[Token], before: usize, language: NoteLanguage) -> i32 {
    let mut fifths = 0;
    for (i, token) in tokens.iter().enumerate() {
        if token.byte_start >= before {
            break;
        }
        if token.mode != Mode::Music || token.raw != "\\key" {
            continue;
        }
        let mut rest = tokens[i + 1..]
            .iter()
            .filter(|t| !matches!(t.kind, TokenKind::Ws | TokenKind::Comment));
        let (Some(tonic), Some(mode)) = (rest.next(), rest.next()) else {
            continue;
        };
        let Some((step, alter)) = parse_name(&tonic.raw, language) else {
            continue;
        };
        // Fifths of a major tonic: C=0, G=1 ... and a semitone up adds seven fifths.
        let major = [0, 2, 4, -1, 1, 3, 5][step as usize] + 7 * alter;
        fifths = if mode.raw == "\\minor" {
            major - 3
        } else {
            major
        };
    }
    fifths
}

/// Alteration a key signature gives a step.
pub(crate) fn key_alter(fifths: i32, step: i32) -> i32 {
    // Sharps are added in the order F C G D A E B; flats in the reverse order.
    const ORDER: [i32; 7] = [3, 0, 4, 1, 5, 2, 6];
    let position = ORDER.iter().position(|s| *s == step).unwrap_or(0) as i32;
    if fifths > position {
        1 + (fifths - 7 > position) as i32
    } else if -fifths > 6 - position {
        -1 - (-fifths - 7 > 6 - position) as i32
    } else {
        0
    }
}

/// Step and alteration of a pitch name in the given language.
pub(crate) fn parse_name(text: &str, language: NoteLanguage) -> Option<(i32, i32)> {
    let letters: String = text.chars().take_while(char::is_ascii_lowercase).collect();
    (0..7)
        .flat_map(|step| (-2..=2).map(move |alter| (step, alter)))
        .find(|(step, alter)| name(*step, *alter, language) == letters)
        // Accept the long Dutch spellings `ees` and `aes` as well.
        .or(match letters.as_str() {
            "ees" => Some((2, -1)),
            "aes" => Some((5, -1)),
            _ => None,
        })
}

/// The spelling of `step` closest to `near`, with the key's alteration.
pub(crate) fn nearest(step: i32, near: i32, fifths: i32) -> Spelled {
    let delta = (step - near.rem_euclid(7) + 3).rem_euclid(7) - 3;
    Spelled {
        di: near + delta,
        alter: key_alter(fifths, step),
    }
}

/// Respell after moving `semitones`: octaves keep the spelling; semitones prefer naturals,
/// then sharps going up or flats going down.
pub(crate) fn transposed(pitch: Spelled, semitones: i32) -> Spelled {
    if semitones % 12 == 0 {
        return Spelled {
            di: pitch.di + 7 * (semitones / 12),
            alter: pitch.alter,
        };
    }
    let target = pitch.midi() + semitones;
    let mut options: Vec<Spelled> = (pitch.di - 2..=pitch.di + 2)
        .flat_map(|di| (-1..=1).map(move |alter| Spelled { di, alter }))
        .filter(|candidate| candidate.midi() == target)
        .collect();
    options.sort_by_key(|candidate| {
        let preferred = if semitones > 0 { 1 } else { -1 };
        (candidate.alter != 0, candidate.alter != preferred)
    });
    options.into_iter().next().unwrap_or(pitch)
}
