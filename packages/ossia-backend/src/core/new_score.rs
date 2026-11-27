//! Blank scores created from the management window.
use super::*;
use std::fmt::Write as _;

/// LilyPond version declared by new scores; it matches the bundled engine.
const NEW_SCORE_VERSION: &str = "2.26.0";
/// How long each import of a new score may take; two imports fit the 45 s management call.
const CREATE_WAIT_MS: u64 = 20_000;

/// Tonic names for major keys from seven flats to seven sharps.
const MAJOR_TONICS: [&str; 15] = [
    "cf", "gf", "df", "af", "ef", "bf", "f", "c", "g", "d", "a", "e", "b", "fs", "cs",
];
/// Tonic names for minor keys from seven flats to seven sharps.
const MINOR_TONICS: [&str; 15] = [
    "af", "ef", "bf", "f", "c", "g", "d", "a", "e", "b", "fs", "cs", "gs", "ds", "as",
];

impl Runtime {
    /// Import a blank score built from the management dialog and record its catalog metadata.
    pub fn management_create_score(
        &self,
        args: ManagementCreateScoreArgs,
    ) -> Result<ManagementCreatedScore> {
        self.available()?;
        let source = new_score_source(&args)?;
        let title = args.title.trim().to_owned();
        let mut outcome = self.import_new_score(&title, &source, None)?;
        if outcome.document.is_none() {
            // Empty measures produce no MIDI notes, so the import asks to confirm a score without
            // playback. The source was generated here and holds no music yet, so confirm it unless
            // the conversion also reports losses or blocking problems.
            let report = &outcome.report;
            let confirmation = report
                .confirmation
                .clone()
                .filter(|_| {
                    report.status == ImportStatus::NeedsConfirmation
                        && report.losses.is_empty()
                        && report.blocking.is_empty()
                })
                .ok_or_else(|| {
                    report.playback_error.clone().unwrap_or_else(|| {
                        storage("new score import finished without creating a document")
                    })
                })?;
            outcome = self.import_new_score(&title, &source, Some(confirmation))?;
        }
        let document = outcome
            .document
            .ok_or_else(|| storage("new score import finished without creating a document"))?;
        self.update_metadata(UpdateMetadataArgs {
            document_id: document.document_id,
            title: None,
            composer: optional_text(&args.composer),
            instruments: Some(vec![instrument_name(args.instrument).into()]),
            period: None,
        })?;
        Ok(ManagementCreatedScore {
            document_id: document.document_id,
        })
    }

    /// Run one import of the new score's source and wait for its outcome.
    fn import_new_score(
        &self,
        title: &str,
        source: &str,
        confirmation: Option<ContentHash>,
    ) -> Result<ImportOutcome> {
        let job = self.import(
            ImportArgs {
                title: Some(title.to_owned()),
                format: Some(InputFormat::Ly),
                path: "new-score.ly".into(),
                confirmation,
                author: None,
                expect: None,
                reference: None,
                wait_ms: Some(CREATE_WAIT_MS),
            },
            source.as_bytes().to_vec(),
        )?;
        if matches!(job.state, JobState::Queued | JobState::Running) {
            return Err(AppError::new(
                ErrorCode::RuntimeUnavailable,
                "the new score is still being prepared; it will appear in the library when ready",
            ));
        }
        if !matches!(job.state, JobState::Done) {
            return Err(job
                .error
                .unwrap_or_else(|| storage("the new score could not be created")));
        }
        serde_json::from_value(
            job.result
                .ok_or_else(|| storage("new score import finished without a result"))?,
        )
        .map_err(storage)
    }
}

fn optional_text(value: &Option<String>) -> Option<String> {
    value
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

fn instrument_name(instrument: NewScoreInstrument) -> &'static str {
    match instrument {
        NewScoreInstrument::Piano => "Piano",
        NewScoreInstrument::Violin => "Violin",
        NewScoreInstrument::Viola => "Viola",
        NewScoreInstrument::Cello => "Cello",
    }
}

/// Validate one free-text field and quote it as a LilyPond string.
fn quoted(field: &str, value: &str, max: usize) -> Result<String> {
    if value.chars().count() > max {
        return Err(invalid(format!("{field} exceeds {max} characters")));
    }
    if value.chars().any(char::is_control) {
        return Err(invalid(format!("{field} must be a single line")));
    }
    Ok(format!(
        "\"{}\"",
        value.replace('\\', "\\\\").replace('"', "\\\"")
    ))
}

fn gcd(a: u32, b: u32) -> u32 {
    if b == 0 {
        a
    } else {
        gcd(b, a % b)
    }
}

/// One measure as a duration: a plain or dotted note value when one exists, else a scaled whole.
fn measure_duration(numerator: u8, denominator: u8) -> String {
    let divisor = gcd(numerator.into(), denominator.into());
    let (n, d) = (
        u32::from(numerator) / divisor,
        u32::from(denominator) / divisor,
    );
    match n {
        1 => d.to_string(),
        3 if d >= 2 => format!("{}.", d / 2),
        _ => format!("1*{numerator}/{denominator}"),
    }
}

/// The beat unit of a metronome mark: a dotted note in compound meters.
fn tempo_unit(numerator: u8, denominator: u8) -> String {
    if denominator >= 8 && numerator > 3 && numerator.is_multiple_of(3) {
        format!("{}.", denominator / 2)
    } else {
        denominator.to_string()
    }
}

/// Build the LilyPond source of a blank score.
pub(crate) fn new_score_source(args: &ManagementCreateScoreArgs) -> Result<String> {
    let title = args.title.trim();
    if title.is_empty() {
        return Err(invalid("title must not be empty"));
    }
    let title = quoted("title", title, 200)?;
    let composer = optional_text(&args.composer)
        .map(|composer| quoted("composer", &composer, 200))
        .transpose()?;
    if !(-7..=7).contains(&args.fifths) {
        return Err(invalid("fifths must be between -7 and 7"));
    }
    if !(1..=32).contains(&args.time_numerator) {
        return Err(invalid("time numerator must be between 1 and 32"));
    }
    if ![1, 2, 4, 8, 16, 32].contains(&args.time_denominator) {
        return Err(invalid("time denominator must be 1, 2, 4, 8, 16 or 32"));
    }
    if !(1..=999).contains(&args.measures) {
        return Err(invalid("measures must be between 1 and 999"));
    }
    if args.tempo_bpm.is_some_and(|bpm| !(20..=400).contains(&bpm)) {
        return Err(invalid("tempo must be between 20 and 400 beats per minute"));
    }
    let tempo_text = optional_text(&args.tempo_text)
        .map(|text| quoted("tempo", &text, 60))
        .transpose()?;

    let index = usize::try_from(i16::from(args.fifths) + 7).map_err(storage)?;
    let (tonic, mode) = match args.mode {
        NewScoreMode::Major => (MAJOR_TONICS[index], "major"),
        NewScoreMode::Minor => (MINOR_TONICS[index], "minor"),
    };
    let time = format!("{}/{}", args.time_numerator, args.time_denominator);
    let measure = measure_duration(args.time_numerator, args.time_denominator);
    let rests = if args.measures == 1 {
        format!("R{measure}")
    } else {
        format!("R{measure}*{}", args.measures)
    };
    let tempo = match (&tempo_text, args.tempo_bpm) {
        (None, None) => None,
        (Some(text), None) => Some(format!("\\tempo {text}")),
        (text, Some(bpm)) => Some(format!(
            "\\tempo {}{} = {bpm}",
            text.as_deref()
                .map(|text| format!("{text} "))
                .unwrap_or_default(),
            tempo_unit(args.time_numerator, args.time_denominator),
        )),
    };

    // Each staff variable: name, relative start, clef, and whether it carries the tempo.
    let staves: &[(&str, &str, &str, bool)] = match args.instrument {
        NewScoreInstrument::Piano => &[
            ("upper", "c''", "treble", true),
            ("lower", "c", "bass", false),
        ],
        NewScoreInstrument::Violin => &[("violin", "c''", "treble", true)],
        NewScoreInstrument::Viola => &[("viola", "c'", "alto", true)],
        NewScoreInstrument::Cello => &[("cello", "c", "bass", true)],
    };

    let mut text = String::new();
    let _ = writeln!(text, "\\version \"{NEW_SCORE_VERSION}\"");
    let _ = writeln!(text, "\\language \"english\"\n");
    let _ = writeln!(text, "\\header {{");
    let _ = writeln!(text, "  title = {title}");
    if let Some(composer) = &composer {
        let _ = writeln!(text, "  composer = {composer}");
    }
    let _ = writeln!(text, "}}\n");
    for (name, start, clef, carries_tempo) in staves {
        let _ = writeln!(text, "{name} = \\relative {start} {{");
        let _ = writeln!(text, "  \\clef {clef} \\key {tonic} \\{mode} \\time {time}");
        if *carries_tempo {
            if let Some(tempo) = &tempo {
                let _ = writeln!(text, "  {tempo}");
            }
        }
        let _ = writeln!(text, "  {rests}");
        let _ = writeln!(text, "  \\bar \"|.\"");
        let _ = writeln!(text, "}}\n");
    }
    let _ = writeln!(text, "\\score {{");
    match args.instrument {
        NewScoreInstrument::Piano => {
            let _ = writeln!(text, "  \\new PianoStaff <<");
            let _ = writeln!(text, "    \\new Staff = \"upper\" \\upper");
            let _ = writeln!(text, "    \\new Staff = \"lower\" \\lower");
            let _ = writeln!(text, "  >>");
        }
        _ => {
            let _ = writeln!(text, "  \\new Staff \\{}", staves[0].0);
        }
    }
    let _ = writeln!(text, "  \\layout {{ }}");
    let _ = writeln!(text, "  \\midi {{ }}");
    let _ = writeln!(text, "}}");
    Ok(text)
}

#[cfg(test)]
#[path = "../../tests/unit/core/new_score.rs"]
mod tests;
