use crate::Result;
use ossia_types::{AppError, ErrorCode};
use std::{
    fmt,
    fs::{self, File},
    io::Read,
    net::SocketAddr,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

pub const MAX_UPLOAD_BYTES: u64 = 128 * 1024 * 1024;
pub const MAX_MULTIPART_BYTES: usize = 129 * 1024 * 1024;
pub const MAX_JOB_SECONDS: u64 = 30 * 60;
pub const LOG_LIMIT_BYTES: u64 = 16 * 1024 * 1024;
const MAX_HEAP_MIB: u64 = 1300;

pub struct Config {
    pub home: PathBuf,
    pub listen: SocketAddr,
    pub audiveris_bin: PathBuf,
    pub audiveris_version: String,
    pub token: String,
    pub languages: Vec<String>,
    pub tessdata_dir: PathBuf,
    pub max_upload_bytes: u64,
    pub max_job_seconds: u64,
    pub log_limit_bytes: u64,
}

impl fmt::Debug for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Config")
            .field("home", &self.home)
            .field("listen", &self.listen)
            .field("audiveris_bin", &self.audiveris_bin)
            .field("audiveris_version", &self.audiveris_version)
            .field("token", &"<redacted>")
            .field("languages", &self.languages)
            .field("max_upload_bytes", &self.max_upload_bytes)
            .field("max_job_seconds", &self.max_job_seconds)
            .finish()
    }
}

impl Config {
    pub fn discover(home: Option<PathBuf>, listen: Option<String>) -> Result<Self> {
        let home = home
            .or_else(|| std::env::var_os("OSSIA_OMR_HOME").map(PathBuf::from))
            .unwrap_or_else(|| PathBuf::from("/var/lib/ossia-omr"));
        if !home.is_absolute() {
            return Err(invalid("OSSIA_OMR_HOME must be absolute"));
        }
        let listen = listen
            .or_else(|| std::env::var("OSSIA_OMR_LISTEN").ok())
            .unwrap_or_else(|| "127.0.0.1:3212".into())
            .parse()
            .map_err(|_| invalid("OSSIA_OMR_LISTEN must be an IP socket address"))?;
        let token = std::env::var("OSSIA_OMR_TOKEN")
            .map_err(|_| AppError::new(ErrorCode::Unauthorized, "OSSIA_OMR_TOKEN is required"))?;
        validate_token(&token)?;
        let audiveris_bin = std::env::var_os("AUDIVERIS_BIN")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("/opt/audiveris/bin/Audiveris"));
        let audiveris_version = detect_audiveris_version(&audiveris_bin)?;
        let languages = parse_languages(
            &std::env::var("OSSIA_OMR_LANGUAGES").unwrap_or_else(|_| "eng,chi_sim".into()),
        )?;
        let max_job_seconds = std::env::var("OSSIA_OMR_MAX_JOB_SECONDS")
            .ok()
            .map(|value| {
                value
                    .parse::<u64>()
                    .map_err(|_| invalid("OSSIA_OMR_MAX_JOB_SECONDS must be a positive integer"))
            })
            .transpose()?
            .unwrap_or(MAX_JOB_SECONDS);
        if max_job_seconds == 0 {
            return Err(invalid(
                "OSSIA_OMR_MAX_JOB_SECONDS must be a positive integer",
            ));
        }
        Ok(Self {
            tessdata_dir: home.join("tessdata"),
            home,
            listen,
            audiveris_bin,
            audiveris_version,
            token,
            languages,
            max_upload_bytes: MAX_UPLOAD_BYTES,
            max_job_seconds,
            log_limit_bytes: LOG_LIMIT_BYTES,
        })
    }

    pub fn prepare(&self) -> Result<()> {
        fs::create_dir_all(self.home.join("jobs")).map_err(storage)?;
        fs::create_dir_all(&self.tessdata_dir).map_err(storage)?;
        check_tessdata(&self.tessdata_dir, &self.languages)?;
        check_heap_limit(&self.audiveris_bin)?;
        Ok(())
    }
}

fn invalid(message: impl Into<String>) -> AppError {
    AppError::invalid(message)
}

fn storage(error: impl fmt::Display) -> AppError {
    AppError::new(ErrorCode::StorageError, error.to_string())
}

fn validate_token(token: &str) -> Result<()> {
    if token.is_empty()
        || token.len() > 4096
        || token
            .bytes()
            .any(|byte| byte.is_ascii_control() || byte.is_ascii_whitespace())
    {
        return Err(AppError::new(
            ErrorCode::Unauthorized,
            "OSSIA_OMR_TOKEN must be a non-empty single-line bearer token",
        ));
    }
    Ok(())
}

fn parse_languages(value: &str) -> Result<Vec<String>> {
    let mut languages = Vec::new();
    for language in value.split([',', '+']) {
        let language = language.trim();
        if language.is_empty()
            || !language
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
        {
            return Err(invalid(
                "OSSIA_OMR_LANGUAGES must be a comma- or plus-separated language list",
            ));
        }
        if !languages.iter().any(|existing| existing == language) {
            languages.push(language.to_string());
        }
    }
    if languages.is_empty() {
        return Err(invalid("OSSIA_OMR_LANGUAGES must not be empty"));
    }
    Ok(languages)
}

fn detect_audiveris_version(executable: &Path) -> Result<String> {
    let mut child = Command::new(executable)
        .arg("-version")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| {
            AppError::new(
                ErrorCode::RuntimeIncompatible,
                format!("cannot start Audiveris: {error}"),
            )
        })?;
    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        match child.try_wait().map_err(storage)? {
            Some(_) => break,
            None if Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(AppError::new(
                    ErrorCode::RuntimeIncompatible,
                    "Audiveris version check timed out",
                ));
            }
            None => std::thread::sleep(Duration::from_millis(50)),
        }
    }
    let output = child.wait_with_output().map_err(storage)?;
    let text = format!(
        "{} {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    if !text.contains("5.11.0") {
        return Err(AppError::new(
            ErrorCode::RuntimeIncompatible,
            "Audiveris 5.11.0 is required",
        ));
    }
    Ok("5.11.0".into())
}

fn check_tessdata(directory: &Path, languages: &[String]) -> Result<()> {
    for language in languages {
        let path = directory.join(format!("{language}.traineddata"));
        let metadata = fs::metadata(&path).map_err(|error| {
            AppError::new(
                ErrorCode::RuntimeIncompatible,
                format!("missing OCR language file {}: {error}", path.display()),
            )
        })?;
        if !metadata.is_file() || metadata.len() == 0 {
            return Err(AppError::new(
                ErrorCode::RuntimeIncompatible,
                format!("OCR language file is invalid: {}", path.display()),
            ));
        }
        let mut file = File::open(&path).map_err(storage)?;
        let mut probe = [0u8; 1];
        file.read_exact(&mut probe).map_err(storage)?;
    }
    Ok(())
}

fn check_heap_limit(executable: &Path) -> Result<()> {
    let executable = executable.canonicalize().map_err(storage)?;
    let root = executable
        .parent()
        .and_then(Path::parent)
        .ok_or_else(|| invalid("Audiveris executable has no application root"))?;
    let path = root.join("lib/app/Audiveris.cfg");
    let text = fs::read_to_string(&path).map_err(|error| {
        AppError::new(
            ErrorCode::RuntimeIncompatible,
            format!(
                "cannot read Audiveris heap configuration {}: {error}",
                path.display()
            ),
        )
    })?;
    let limit = heap_limit(&text).ok_or_else(|| {
        AppError::new(
            ErrorCode::RuntimeIncompatible,
            "Audiveris.cfg has no -Xmx heap limit",
        )
    })?;
    if limit > MAX_HEAP_MIB {
        return Err(AppError::new(
            ErrorCode::RuntimeIncompatible,
            format!("Audiveris heap limit must be at most {MAX_HEAP_MIB} MiB"),
        ));
    }
    Ok(())
}

fn heap_limit(value: &str) -> Option<u64> {
    value
        .split_whitespace()
        .find_map(|token| token.split_once("-Xmx").map(|(_, value)| value))
        .and_then(parse_mebibytes)
}

fn parse_mebibytes(value: &str) -> Option<u64> {
    let (digits, multiplier) = match value.as_bytes().last()?.to_ascii_lowercase() {
        b'k' => (&value[..value.len() - 1], 1.0 / 1024.0),
        b'm' => (&value[..value.len() - 1], 1.0),
        b'g' => (&value[..value.len() - 1], 1024.0),
        _ => (value, 1.0 / (1024.0 * 1024.0)),
    };
    Some((digits.parse::<u64>().ok()? as f64 * multiplier) as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_rejects_line_breaks() {
        assert!(validate_token("secret").is_ok());
        assert!(validate_token("secret\nnext").is_err());
        assert!(validate_token("").is_err());
    }

    #[test]
    fn languages_are_normalized_and_deduplicated() {
        assert_eq!(
            parse_languages(" eng,chi_sim ").unwrap(),
            ["eng", "chi_sim"]
        );
        assert!(parse_languages("eng/chi").is_err());
    }

    #[test]
    fn heap_units_are_mebibytes() {
        assert_eq!(heap_limit("java-options=-Xmx1G"), Some(1024));
        assert_eq!(parse_mebibytes("1g"), Some(1024));
        assert_eq!(parse_mebibytes("1300M"), Some(1300));
        assert_eq!(parse_mebibytes("1048576"), Some(1));
    }
}
