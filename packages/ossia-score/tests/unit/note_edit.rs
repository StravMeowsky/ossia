use super::*;

const Q: NoteValue = NoteValue { log: 2, dots: 0 };
const H: NoteValue = NoteValue { log: 1, dots: 0 };
const E: NoteValue = NoteValue { log: 3, dots: 0 };

fn score(music: &str) -> String {
    format!("\\version \"2.26.0\"\nupper = \\relative c'' {{\n  \\key g \\major \\time 4/4\n  {music}\n}}\n")
}

fn edit(music: &str, at: &str, op: NoteEditOp) -> Result<(String, String), AppError> {
    let source = score(music);
    let target = source.find(at).expect("target text");
    let out = apply_note_edit(&source, target, &op)?;
    let start = source.find(music).unwrap();
    let end = out.source.len() - (source.len() - start - music.len());
    Ok((
        out.source[start..end].to_owned(),
        out.source[out.cursor..].chars().take(4).collect(),
    ))
}

fn music(music: &str, at: &str, op: NoteEditOp) -> String {
    edit(music, at, op).expect("edit").0
}

#[test]
fn enter_fills_a_rest_with_the_key_signature() {
    // F is sharp in G major; the note is the nearest F to the previous B.
    assert_eq!(
        music(
            "b4 r2. |",
            "r2.",
            NoteEditOp::Enter {
                step: 3,
                value: Q,
                chord: false
            }
        ),
        "b4 fis4 r2 |"
    );
}

#[test]
fn enter_keeps_the_following_note_at_its_pitch() {
    // Writing a low note before `d` would drag it down in relative mode.
    assert_eq!(
        music(
            "b4 r4 f4 g4 |",
            "r4",
            NoteEditOp::Enter {
                step: 2,
                value: Q,
                chord: false
            }
        ),
        "b4 e4 f,4 g4 |"
    );
}

#[test]
fn enter_overwrites_following_rests_for_a_longer_value() {
    assert_eq!(
        music(
            "g4 r4 r4 r4 |",
            "r4 r4 r4",
            NoteEditOp::Enter {
                step: 0,
                value: H,
                chord: false
            }
        ),
        "g4 c2 r4 |"
    );
}

#[test]
fn enter_refuses_to_overwrite_notes() {
    let err = edit(
        "g4 r4 a2 |",
        "r4",
        NoteEditOp::Enter {
            step: 0,
            value: H,
            chord: false,
        },
    );
    assert!(err.is_err());
}

#[test]
fn enter_writes_out_a_carried_duration() {
    assert_eq!(
        music(
            "g8 r8 r8 a b c d e |",
            "r8",
            NoteEditOp::Enter {
                step: 1,
                value: Q,
                chord: false
            }
        ),
        "g8 d4 a'8 b c d e |"
    );
}

#[test]
fn measure_rest_is_split_before_writing() {
    assert_eq!(
        music(
            "R1*3 |",
            "R1",
            NoteEditOp::Enter {
                step: 4,
                value: H,
                chord: false
            }
        ),
        "g2 r2 R1*2 |"
    );
}

#[test]
fn chord_tone_is_added_above() {
    assert_eq!(
        music(
            "g4 a b c |",
            "g4",
            NoteEditOp::Enter {
                step: 1,
                value: Q,
                chord: true
            }
        ),
        "<g d'>4 a b c |"
    );
}

#[test]
fn transpose_moves_by_semitones_and_octaves() {
    assert_eq!(
        music("g4 a b c |", "a b", NoteEditOp::Transpose { semitones: 1 }),
        "g4 ais b c |"
    );
    assert_eq!(
        music("g4 a b c |", "a b", NoteEditOp::Transpose { semitones: 12 }),
        "g4 a' b, c |"
    );
}

#[test]
fn delete_turns_a_note_into_a_rest_and_drops_its_tie() {
    assert_eq!(
        music("g4~ g4 a b |", "g4~", NoteEditOp::Delete),
        "r4 g4 a b |"
    );
}

#[test]
fn step_moves_within_the_key() {
    // In G major, E steps up to F sharp and C steps down to B.
    assert_eq!(music("g4 e c b |", "e c b", NoteEditOp::Step { steps: 1 }), "g4 fis c b |");
    assert_eq!(music("g4 a c b |", "c b", NoteEditOp::Step { steps: -1 }), "g4 a b b |");
    // A chromatic note returns to the key.
    assert_eq!(music("g4 f c b |", "f c b", NoteEditOp::Step { steps: 1 }), "g4 g c, b |");
}

#[test]
fn chord_heads_keep_their_pitches() {
    // Moving the lowest head would move the heads written relative to it.
    assert_eq!(
        music(
            "<g b d>4 a |",
            "g b",
            NoteEditOp::Transpose { semitones: -12 }
        ),
        "<g, b' d>4 a' |"
    );
    assert_eq!(
        music(
            "<g b>4 a |",
            "b>",
            NoteEditOp::Enter {
                step: 1,
                value: Q,
                chord: true
            }
        ),
        "<g b d>4 a |"
    );
}

#[test]
fn delete_removes_one_chord_tone() {
    assert_eq!(
        music("<g b d>4 a |", "b d>", NoteEditOp::Delete),
        "<g d'>4 a |"
    );
}

#[test]
fn ties_and_slurs_toggle() {
    assert_eq!(
        music("g4 g a b |", "g4", NoteEditOp::ToggleTie),
        "g4~ g a b |"
    );
    assert_eq!(
        music("g4~ g a b |", "g4", NoteEditOp::ToggleTie),
        "g4 g a b |"
    );
    assert_eq!(
        music("g4 a b c |", "g4", NoteEditOp::ToggleSlur),
        "g4( a) b c |"
    );
    assert_eq!(
        music("g4( a) b c |", "g4", NoteEditOp::ToggleSlur),
        "g4 a b c |"
    );
}

#[test]
fn dynamics_replace_each_other() {
    let mark = |m: &str| NoteEditOp::Dynamic { mark: m.into() };
    assert_eq!(music("g4 a b c |", "g4", mark("p")), "g4\\p a b c |");
    assert_eq!(music("g4\\p a b c |", "g4", mark("f")), "g4\\f a b c |");
    assert_eq!(music("g4\\f a b c |", "g4", mark("f")), "g4 a b c |");
    assert_eq!(music("g8 (a) b c |", "g8", mark("p")), "g8 (\\p a) b c |");
}

#[test]
fn set_value_shortens_into_rests() {
    assert_eq!(
        music("g2 a4 b |", "g2", NoteEditOp::SetValue { value: E }),
        "g8 r4. a4 b |"
    );
}

/// Run every action on every event of the scores in `OSSIA_NOTE_EDIT_SCORES`.
#[test]
#[ignore = "needs a folder of scores"]
fn stress_real_scores() {
    let Ok(dir) = std::env::var("OSSIA_NOTE_EDIT_SCORES") else {
        return;
    };
    let mut tally: std::collections::BTreeMap<String, usize> = Default::default();
    for path in std::fs::read_dir(dir).unwrap() {
        let path = path.unwrap().path();
        let source = std::fs::read_to_string(&path).unwrap();
        let events = match events::scan(&source) {
            Ok(events) => events,
            Err(e) => {
                *tally
                    .entry(format!("scan failed {}: {}", path.display(), e.message))
                    .or_default() += 1;
                continue;
            }
        };
        for event in events.iter().step_by(3) {
            let ops: Vec<NoteEditOp> = if event.pitched() {
                vec![
                    NoteEditOp::Transpose { semitones: 1 },
                    NoteEditOp::Transpose { semitones: -12 },
                    NoteEditOp::Delete,
                    NoteEditOp::ToggleTie,
                    NoteEditOp::Dynamic { mark: "mf".into() },
                    NoteEditOp::SetValue { value: E },
                ]
            } else {
                vec![NoteEditOp::Enter {
                    step: 2,
                    value: E,
                    chord: false,
                }]
            };
            for op in ops {
                let key = match apply_note_edit(&source, event.start, &op) {
                    Ok(out) => match (&op, analysis(&out.source, event.start)) {
                        (NoteEditOp::Transpose { semitones }, Ok(after)) => {
                            let before = analysis(&source, event.start).unwrap();
                            let old = before.iter().find(|e| e.span.byte_start == event.start);
                            let new = after.iter().find(|e| e.span.byte_start == event.start);
                            let moved = |e: &pitch::PitchEvent| -> Vec<i32> {
                                e.pitches
                                    .iter()
                                    .map(|p| p + i32::from(*semitones))
                                    .collect()
                            };
                            if old.map(moved) == new.map(|e| e.pitches.clone()) {
                                "ok".into()
                            } else {
                                format!("WRONG PITCH {}", path.display())
                            }
                        }
                        (_, Ok(_)) => "ok".into(),
                        (_, Err(e)) => format!("BROKEN {}", e.message),
                    },
                    Err(e) => format!("{:?}: {}", std::mem::discriminant(&op), e.message),
                };
                *tally.entry(key).or_default() += 1;
            }
        }
    }
    for (key, count) in tally {
        eprintln!("{count:6} {key}");
    }
}

#[test]
fn plain_nested_blocks_pass_their_pitch_on() {
    // Checked against LilyPond: `b` follows the tuplet's `e`, not the `a` before it.
    let source = "m = \\relative c'' { c4 { g g } a \\tuplet 3/2 { e e e } b }\n";
    let pitches: Vec<i32> = analysis(source, 25)
        .unwrap()
        .iter()
        .flat_map(|e| e.pitches.clone())
        .collect();
    assert_eq!(pitches, vec![72, 67, 67, 69, 64, 64, 64, 59]);
}

#[test]
fn a_following_chord_keeps_its_pitches() {
    assert_eq!(
        music(
            "r4 <e g>2 r4 |",
            "r4 <",
            NoteEditOp::Enter {
                step: 3,
                value: Q,
                chord: false
            }
        ),
        "fis,4 <e' g>2 r4 |"
    );
    assert_eq!(
        music(
            "<b d>4 <e g>2 r4 |",
            "<b d>",
            NoteEditOp::Transpose { semitones: -12 }
        ),
        "<b, d>4 <e' g>2 r4 |"
    );
}

#[test]
fn accents_are_not_chord_closes() {
    assert_eq!(
        music(
            "g4-> a^> b_> c4_> |",
            "a^",
            NoteEditOp::Transpose { semitones: 1 }
        ),
        "g4-> ais^> b_> c4_> |"
    );
}

#[test]
fn typing_into_a_blank_score_follows_the_cursor() {
    let mut source = String::from(
        "\\version \"2.26.0\"\n\\language \"english\"\n\
         upper = \\relative c'' {\n  \\clef treble \\key g \\major \\time 3/4\n  R2.*4\n  \\bar \"|.\"\n}\n\
         \\score { \\new Staff \\upper }\n",
    );
    let mut cursor = source.find("R2.").unwrap();
    // G A B as quarters, then F (sharp in G major) as a dotted half, then C an octave up.
    let keys: [(u8, NoteValue); 5] = [
        (4, Q),
        (5, Q),
        (6, Q),
        (3, NoteValue { log: 1, dots: 1 }),
        (0, Q),
    ];
    for (step, value) in keys {
        let out = apply_note_edit(&source, cursor, &NoteEditOp::Enter { step, value, chord: false })
            .unwrap();
        source = out.source;
        cursor = out.cursor;
    }
    let out = apply_note_edit(&source, cursor, &NoteEditOp::Transpose { semitones: 12 }).unwrap();
    assert!(
        out.source.contains("  g4 a4 b4 fs2. c'4 r2 R2.\n"),
        "{}",
        out.source
    );
    // The preview of the result has four measures with the notes in place.
    let mei = crate::source_to_mei(&out.source).unwrap();
    assert_eq!(mei.matches("<measure ").count(), 4, "{mei}");
    assert!(mei.contains(r#"pname="f" oct="4" accid.ges="s" dur="2" dots="1""#), "{mei}");
}
