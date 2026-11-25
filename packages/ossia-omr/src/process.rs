use super::{CancelSignal, Result};
use ossia_types::{AppError, Diagnostic, ErrorCode, Severity};
use std::{
    collections::BTreeSet,
    fs::{self, File},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::atomic::Ordering,
    time::{Duration, Instant},
};

#[derive(Clone)]
pub(crate) struct ProcessConfig {
    pub audiveris_bin: PathBuf,
    pub tessdata_dir: PathBuf,
    pub languages: Vec<String>,
    pub max_job_seconds: u64,
    pub log_limit_bytes: u64,
}

pub(crate) fn run(
    job_dir: &Path,
    input: &Path,
    config: &ProcessConfig,
    cancel: &CancelSignal,
) -> Result<Vec<Diagnostic>> {
    let output = job_dir.join("out");
    let home = job_dir.join("home");
    let xdg = job_dir.join("config");
    let cache = job_dir.join("cache");
    let temporary = job_dir.join("tmp");
    for directory in [&output, &home, &xdg, &cache, &temporary] {
        fs::create_dir_all(directory).map_err(storage)?;
    }
    let log_path = output.join("audiveris.log");
    let log = File::create(&log_path).map_err(storage)?;
    let language = format!(
        "org.audiveris.omr.text.Language.defaultSpecification={}",
        config.languages.join("+")
    );
    let mut command = Command::new(&config.audiveris_bin);
    command
        .arg("-batch")
        .arg("-transcribe")
        .arg("-export")
        .arg("-save")
        .arg("-swap")
        .arg("-constant")
        .arg("org.audiveris.omr.sheet.BookManager.useCompression=false")
        .arg("-constant")
        .arg("org.audiveris.omr.sheet.BookManager.useOpus=false")
        .arg("-constant")
        .arg("org.audiveris.omr.sheet.BookManager.useSeparateBookFolders=false")
        .arg("-constant")
        .arg(language)
        .arg("-output")
        .arg(&output)
        .arg("--")
        .arg(input)
        .current_dir(job_dir)
        .env("HOME", &home)
        .env("USERPROFILE", &home)
        .env("XDG_CONFIG_HOME", &xdg)
        .env("XDG_CACHE_HOME", &cache)
        .env("TESSDATA_PREFIX", &config.tessdata_dir)
        .env("TMPDIR", &temporary)
        .env("TMP", &temporary)
        .env("TEMP", &temporary)
        .env("LANG", "C.UTF-8")
        .stdin(Stdio::null())
        .stdout(log.try_clone().map_err(storage)?)
        .stderr(log);
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    let mut child = command.spawn().map_err(|error| AppError {
        code: ErrorCode::CompileFailed,
        message: "cannot start Audiveris".into(),
        detail: Some(serde_json::json!({
            "reason": "omr-spawn",
            "error": error.to_string(),
        })),
    })?;
    let deadline = Instant::now() + Duration::from_secs(config.max_job_seconds);
    let status = loop {
        if cancel.load(Ordering::Acquire) {
            terminate(&mut child);
            return Err(AppError::new(ErrorCode::Cancelled, "OMR job cancelled"));
        }
        if Instant::now() >= deadline {
            terminate(&mut child);
            return Err(failure(
                "omr-timeout",
                &format!(
                    "Audiveris exceeded the {} second job limit",
                    config.max_job_seconds
                ),
            ));
        }
        if fs::metadata(&log_path).map_err(storage)?.len() > config.log_limit_bytes {
            terminate(&mut child);
            return Err(failure(
                "omr-log-limit",
                "Audiveris log exceeded the configured limit",
            ));
        }
        if let Some(status) = child.try_wait().map_err(storage)? {
            break status;
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    let mut bytes = fs::read(&log_path).map_err(storage)?;
    bytes.truncate(config.log_limit_bytes as usize);
    let log = String::from_utf8_lossy(&bytes);
    let diagnostics = diagnostics(&log, job_dir);
    if !status.success() {
        return Err(AppError {
            code: ErrorCode::CompileFailed,
            message: "Audiveris failed to transcribe the input".into(),
            detail: Some(serde_json::json!({
                "reason": "omr-process",
                "diagnostics": diagnostics,
            })),
        });
    }
    Ok(diagnostics)
}

fn terminate(child: &mut Child) {
    #[cfg(unix)]
    unsafe {
        libc::kill(-(child.id() as i32), libc::SIGKILL);
    }
    let _ = child.kill();
    let _ = child.wait();
}

fn diagnostics(log: &str, job_dir: &Path) -> Vec<Diagnostic> {
    let home = job_dir.parent().and_then(Path::parent).unwrap_or(job_dir);
    let job = job_dir.to_string_lossy();
    let home = home.to_string_lossy();
    let mut messages = BTreeSet::new();
    for line in log.lines() {
        let lower = line.to_ascii_lowercase();
        if !(lower.contains("warn") || lower.contains("error")) {
            continue;
        }
        let message = redact(line, &job, &home);
        if !message.is_empty() {
            messages.insert(message.chars().take(4096).collect::<String>());
        }
        if messages.len() >= 64 {
            break;
        }
    }
    messages
        .into_iter()
        .map(|message| Diagnostic {
            severity: if message.to_ascii_lowercase().contains("error") {
                Severity::Error
            } else {
                Severity::Warning
            },
            message,
            raw_position: None,
            position: None,
        })
        .collect()
}

fn redact(line: &str, job: &str, home: &str) -> String {
    line.replace(job, "<job>")
        .replace(home, "<omr-home>")
        .split_whitespace()
        .map(redact_absolute_token)
        .collect::<Vec<_>>()
        .join(" ")
}

fn redact_absolute_token(token: &str) -> String {
    let core = token.trim_matches(|character: char| {
        matches!(
            character,
            '(' | ')' | '[' | ']' | '{' | '}' | '<' | '>' | ',' | ';' | ':' | '"' | '\''
        )
    });
    let absolute = core.starts_with('/')
        || core
            .as_bytes()
            .get(1)
            .is_some_and(|character| *character == b':')
            && core.as_bytes().first().is_some_and(u8::is_ascii_alphabetic);
    if absolute {
        token.replace(core, "<path>")
    } else {
        token.to_string()
    }
}

fn failure(reason: &str, message: &str) -> AppError {
    AppError {
        code: ErrorCode::CompileFailed,
        message: message.into(),
        detail: Some(serde_json::json!({"reason": reason})),
    }
}

fn storage(error: impl std::fmt::Display) -> AppError {
    AppError::new(ErrorCode::StorageError, error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagnostics_are_deduplicated_and_redacted() {
        let root = Path::new("/var/lib/ossia-omr/jobs/abc");
        let log = "WARN Reading /var/lib/ossia-omr/jobs/abc/out/a.xml\n\
                   WARN Reading /var/lib/ossia-omr/jobs/abc/out/a.xml\n\
                   INFO ignored";
        let diagnostics = diagnostics(log, root);
        assert_eq!(diagnostics.len(), 1);
        assert!(!diagnostics[0].message.contains("/var/lib"));
    }
}
