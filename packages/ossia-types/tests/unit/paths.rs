use super::*;
use crate::{AppError, ErrorCode};
use serde_json::json;

#[test]
fn extended_windows_paths_are_removed_from_display_text() {
    assert_eq!(
        normalize_path(r"\\?\E:\Projects\ossia\.ossia"),
        r"E:\Projects\ossia\.ossia"
    );
    assert_eq!(
        normalize_path(r"\\?\UNC\server\share\score.ly"),
        r"\\server\share\score.ly"
    );
    assert_eq!(
        normalize_path("//?/E:/Projects/ossia/.ossia"),
        if cfg!(windows) {
            r"E:\Projects\ossia\.ossia"
        } else {
            "E:/Projects/ossia/.ossia"
        }
    );
    assert_eq!(
        normalize_path("//?/UNC/server/share/score.ly"),
        if cfg!(windows) {
            r"\\server\share\score.ly"
        } else {
            "//server/share/score.ly"
        }
    );
    assert_eq!(
        normalize_path("https://example.com/a/b"),
        "https://example.com/a/b"
    );
    assert_eq!(normalize_path("relative/path"), "relative/path");
}

#[cfg(windows)]
#[test]
fn windows_display_paths_use_backslashes() {
    assert_eq!(
        normalize_path(r"\\?\E:/Projects/ossia/.ossia"),
        r"E:\Projects\ossia\.ossia"
    );
    assert_eq!(
        normalize_path(r"E:\Projects/ossia\.ossia"),
        r"E:\Projects\ossia\.ossia"
    );
}

#[test]
fn outbound_normalization_covers_arrays_and_agent_conflict_targets() {
    let mut value = json!({
        "sourcePath": r"\\?\E:\Projects\ossia\score.ly",
        "content": r"literal \\?\E:\Projects\ossia\score.ly",
        "nested": {"directory": r"\\?\E:\Projects\ossia\exports"},
        "paths": [r"\\?\E:\Projects\ossia\one.svg", r"\\?\E:\Projects\ossia\two.svg"],
        "agents": [{
            "conflicts": [{
                "target": r"\\?\E:\Projects\ossia\plugin",
                "message": r"modified \\?\E:\Projects\ossia\plugin\index.js"
            }]
        }]
    });

    normalize_outbound_paths(&mut value);

    assert_eq!(value["sourcePath"], r"E:\Projects\ossia\score.ly");
    assert_eq!(value["content"], r"literal \\?\E:\Projects\ossia\score.ly");
    assert_eq!(value["nested"]["directory"], r"E:\Projects\ossia\exports");
    assert_eq!(value["paths"][0], r"E:\Projects\ossia\one.svg");
    assert_eq!(value["paths"][1], r"E:\Projects\ossia\two.svg");
    assert_eq!(
        value["agents"][0]["conflicts"][0]["target"],
        r"E:\Projects\ossia\plugin"
    );
    assert_eq!(
        value["agents"][0]["conflicts"][0]["message"],
        r"modified E:\Projects\ossia\plugin\index.js"
    );
}

#[test]
fn outbound_errors_normalize_messages_and_detail_paths() {
    let mut error = AppError::new(
        ErrorCode::StorageError,
        r"failed to read \\?\E:\Projects\ossia\score.ly",
    );
    error.detail = Some(json!({
        "path": r"\\?\E:\Projects\ossia\score.ly",
        "paths": [r"\\?\E:\Projects\ossia\one.svg"],
    }));

    normalize_outbound_error(&mut error);

    assert_eq!(error.message, r"failed to read E:\Projects\ossia\score.ly");
    assert_eq!(
        error.detail.as_ref().unwrap()["path"],
        r"E:\Projects\ossia\score.ly"
    );
    assert_eq!(
        error.detail.as_ref().unwrap()["paths"][0],
        r"E:\Projects\ossia\one.svg"
    );
}
