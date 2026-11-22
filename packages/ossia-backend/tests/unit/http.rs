use super::*;
use axum::body::to_bytes;
use serde_json::json;

#[tokio::test]
async fn error_responses_hide_windows_verbatim_prefixes() {
    let mut error = AppError::new(
        ErrorCode::StorageError,
        r"cannot open \\?\E:\Projects\ossia\score.ly",
    );
    error.detail = Some(json!({"path": r"\\?\E:\Projects\ossia\score.ly"}));

    let response = AppErrorResponse(error).into_response();
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body: Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(
        body["error"]["message"],
        r"cannot open E:\Projects\ossia\score.ly"
    );
    assert_eq!(
        body["error"]["detail"]["path"],
        r"E:\Projects\ossia\score.ly"
    );
}
