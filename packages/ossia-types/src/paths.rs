use crate::AppError;
use serde_json::Value;
use std::path::Path;

/// Convert a filesystem path to the form shown to users and external tools.
pub fn path_text(path: impl AsRef<Path>) -> String {
    normalize_path(&path.as_ref().to_string_lossy())
}

/// Remove Windows extended-length prefixes and use native separators on Windows.
pub fn normalize_path(value: &str) -> String {
    let normalized = value
        .replace(r"\\?\UNC\", r"\\")
        .replace(r"\\?\", "")
        .replace("//?/UNC/", "//")
        .replace("//?/", "");
    if cfg!(windows) && is_windows_absolute(&normalized) {
        normalized.replace('/', r"\")
    } else {
        normalized
    }
}

/// Normalize path-valued fields in one outbound JSON response.
pub fn normalize_outbound_paths(value: &mut Value) {
    match value {
        Value::Object(fields) => {
            for (key, value) in fields {
                if is_path_field(key) {
                    normalize_path_value(value);
                    continue;
                }
                normalize_outbound_paths(value);
            }
        }
        Value::Array(items) => {
            for item in items {
                normalize_outbound_paths(item);
            }
        }
        _ => {}
    }
}

/// Normalize path text in an error that is about to leave the process.
pub fn normalize_outbound_error(error: &mut AppError) {
    error.message = normalize_path(&error.message);
    if let Some(detail) = error.detail.as_mut() {
        normalize_path_value(detail);
    }
}

fn is_path_field(key: &str) -> bool {
    matches!(
        key,
        "path"
            | "paths"
            | "home"
            | "root"
            | "executable"
            | "configRoot"
            | "configPath"
            | "sourcePath"
            | "directory"
            | "omrSource"
            | "outputPath"
            | "outputDirectory"
            | "engineRoot"
            | "docsRoot"
            | "staticDir"
            | "libraryRoot"
            | "sharedRoot"
            | "cliPath"
            | "target"
            | "message"
    )
}

fn normalize_path_value(value: &mut Value) {
    match value {
        Value::String(text) => *text = normalize_path(text),
        Value::Array(items) => {
            for item in items {
                normalize_path_value(item);
            }
        }
        Value::Object(_) => normalize_outbound_paths(value),
        _ => {}
    }
}

fn is_windows_absolute(value: &str) -> bool {
    let bytes = value.as_bytes();
    let drive = bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && matches!(bytes[2], b'\\' | b'/');
    drive || value.starts_with(r"\\") || value.starts_with("//")
}

#[cfg(test)]
#[path = "../tests/unit/paths.rs"]
mod tests;
