use super::*;
use serde_json::json;

#[test]
fn missing_input_reports_operation_and_path() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("missing.musicxml");
    let error = read_input_file(path.to_str().unwrap(), "convert").unwrap_err();
    assert!(error.message.contains("convert"), "{error:?}");
    assert!(error.message.contains("missing.musicxml"), "{error:?}");
}

#[test]
fn rpc_responses_normalize_paths_in_results() {
    let response = rpc_response(Ok(json!({
        "sourcePath": r"\\?\E:\Projects\ossia\score.ly",
        "files": [{"path": r"\\?\E:\Projects\ossia\page-1.svg"}],
        "content": r"literal \\?\E:\Projects\ossia\score.ly"
    })));
    let result = response.result.unwrap();

    assert!(response.error.is_none());
    assert_eq!(result["sourcePath"], r"E:\Projects\ossia\score.ly");
    assert_eq!(result["files"][0]["path"], r"E:\Projects\ossia\page-1.svg");
    assert_eq!(result["content"], r"literal \\?\E:\Projects\ossia\score.ly");
}

#[test]
fn rpc_responses_normalize_paths_in_errors() {
    let mut error = AppError::new(
        ErrorCode::StorageError,
        r"cannot open \\?\E:\Projects\ossia\score.ly",
    );
    error.detail = Some(json!({"path": r"\\?\E:\Projects\ossia\score.ly"}));

    let response = rpc_response(Err(error));
    let error = response.error.unwrap();

    assert!(response.result.is_none());
    assert_eq!(error.message, r"cannot open E:\Projects\ossia\score.ly");
    assert_eq!(
        error.detail.as_ref().unwrap()["path"],
        r"E:\Projects\ossia\score.ly"
    );
}
