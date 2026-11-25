use super::{config::MAX_MULTIPART_BYTES, formats, Service};
use axum::{
    extract::{DefaultBodyLimit, FromRequest, Multipart, Request, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use ossia_types::{AppError, ErrorCode, JobState, OmrCapabilities, OmrJob, OmrSourceFormat};
use serde::Deserialize;
use serde_json::json;

pub(crate) fn router(service: Service) -> Router {
    let protected = Router::new()
        .route("/v1/capabilities", get(capabilities))
        .route("/v1/jobs", post(submit))
        .route("/v1/jobs/{id}", get(job))
        .route("/v1/jobs/{id}/result", get(result))
        .route("/v1/jobs/{id}/cancel", post(cancel))
        .route_layer(middleware::from_fn_with_state(service.clone(), authorize));
    Router::new()
        .route(
            "/v1/health",
            get(|| async {
                Json(json!({"ok": true, "serviceVersion": env!("CARGO_PKG_VERSION")}))
            }),
        )
        .merge(protected)
        .layer(DefaultBodyLimit::max(MAX_MULTIPART_BYTES))
        .with_state(service)
}

async fn authorize(State(service): State<Service>, request: Request, next: Next) -> Response {
    let supplied = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| {
            let (scheme, token) = value.split_once(' ')?;
            scheme.eq_ignore_ascii_case("bearer").then_some(token)
        })
        .unwrap_or_default();
    if !constant_time_eq(supplied.as_bytes(), service.token.as_bytes()) {
        return ApiError(AppError::new(
            ErrorCode::Unauthorized,
            "OMR bearer token is missing or invalid",
        ))
        .into_response();
    }
    next.run(request).await
}

async fn capabilities(State(service): State<Service>) -> Json<OmrCapabilities> {
    Json(service.capabilities.clone())
}

async fn submit(
    State(service): State<Service>,
    request: Request,
) -> ApiResult<(StatusCode, Json<OmrJob>)> {
    let mut multipart = Multipart::from_request(request, &())
        .await
        .map_err(|_| invalid("expected multipart/form-data"))?;
    let mut metadata = None;
    let mut file = None;
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| invalid("invalid or oversized multipart upload"))?
    {
        match field.name() {
            Some("metadata") if metadata.is_none() => {
                let text = field
                    .text()
                    .await
                    .map_err(|_| invalid("invalid OMR metadata"))?;
                metadata = Some(
                    serde_json::from_str::<UploadMetadata>(&text)
                        .map_err(|error| invalid(format!("invalid OMR metadata: {error}")))?,
                );
            }
            Some("file") if file.is_none() => {
                let filename = field.file_name().map(str::to_owned);
                let content_type = field.content_type().map(str::to_owned);
                let bytes = field
                    .bytes()
                    .await
                    .map_err(|_| invalid("invalid OMR file upload"))?;
                if bytes.len() as u64 > service.capabilities.max_upload_bytes {
                    return Err(ApiError(AppError::invalid("file exceeds 128 MiB")));
                }
                file = Some((filename, content_type, bytes.to_vec()));
            }
            _ => {
                return Err(ApiError(AppError::invalid(
                    "expected one metadata and one file field",
                )))
            }
        }
    }
    let metadata = metadata.ok_or_else(|| ApiError(AppError::invalid("missing metadata")))?;
    let (filename, content_type, bytes) =
        file.ok_or_else(|| ApiError(AppError::invalid("missing file")))?;
    formats::validate(
        metadata.source_format,
        filename.as_deref(),
        content_type.as_deref(),
        &bytes,
    )
    .map_err(ApiError)?;
    let service_for_job = service.clone();
    let languages = service.capabilities.languages.clone();
    let job = tokio::task::spawn_blocking(move || {
        service_for_job.queue.create(
            metadata.source_format,
            &bytes,
            &languages,
            &service_for_job.capabilities.audiveris_version,
        )
    })
    .await
    .map_err(|error| ApiError(AppError::new(ErrorCode::StorageError, error.to_string())))??;
    Ok((StatusCode::ACCEPTED, Json(job)))
}

async fn job(
    State(service): State<Service>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> ApiResult<Json<OmrJob>> {
    let id = id
        .parse()
        .map_err(|_| ApiError(AppError::new(ErrorCode::NotFound, "OMR job not found")))?;
    Ok(Json(service.queue.get(id).map_err(ApiError)?))
}

async fn result(
    State(service): State<Service>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> ApiResult<Response> {
    let id = id
        .parse()
        .map_err(|_| ApiError(AppError::new(ErrorCode::NotFound, "OMR job not found")))?;
    let job = service.queue.get(id).map_err(ApiError)?;
    if job.state != JobState::Done {
        return Err(ApiError(AppError::new(
            ErrorCode::Conflict,
            "OMR result is not ready",
        )));
    }
    let bytes = tokio::fs::read(service.queue.output_dir(id).join("result.xml"))
        .await
        .map_err(|error| {
            ApiError(AppError::new(
                ErrorCode::StorageError,
                format!("OMR result is unavailable: {error}"),
            ))
        })?;
    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/vnd.recordare.musicxml+xml"),
    );
    Ok((headers, bytes).into_response())
}

async fn cancel(
    State(service): State<Service>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> ApiResult<Json<OmrJob>> {
    let id = id
        .parse()
        .map_err(|_| ApiError(AppError::new(ErrorCode::NotFound, "OMR job not found")))?;
    Ok(Json(service.cancel(id).map_err(ApiError)?))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct UploadMetadata {
    source_format: OmrSourceFormat,
}

struct ApiError(AppError);

impl From<AppError> for ApiError {
    fn from(error: AppError) -> Self {
        Self(error)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = match self.0.code {
            ErrorCode::Unauthorized => StatusCode::UNAUTHORIZED,
            ErrorCode::NotFound => StatusCode::NOT_FOUND,
            ErrorCode::Conflict => StatusCode::CONFLICT,
            ErrorCode::InvalidRequest => StatusCode::BAD_REQUEST,
            ErrorCode::InvalidInput | ErrorCode::InvalidEncoding | ErrorCode::UnsupportedSource => {
                StatusCode::UNPROCESSABLE_ENTITY
            }
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        (status, Json(json!({"error": self.0}))).into_response()
    }
}

type ApiResult<T> = std::result::Result<T, ApiError>;

fn invalid(message: impl Into<String>) -> ApiError {
    ApiError(AppError::invalid(message))
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    let mut difference = left.len() ^ right.len();
    let length = left.len().max(right.len());
    for index in 0..length {
        let left = left.get(index).copied().unwrap_or_default();
        let right = right.get(index).copied().unwrap_or_default();
        difference |= usize::from(left ^ right);
    }
    difference == 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::process::ProcessConfig;
    use crate::queue::Queue;
    use reqwest::multipart::{Form, Part};
    use std::{
        collections::HashMap,
        path::PathBuf,
        sync::{atomic::AtomicBool, Arc, Mutex},
    };

    #[test]
    fn bearer_comparison_handles_length_and_content() {
        assert!(constant_time_eq(b"token", b"token"));
        assert!(!constant_time_eq(b"token", b"tokens"));
        assert!(!constant_time_eq(b"token", b"tokeN"));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn health_is_public_and_capabilities_and_jobs_require_bearer_auth() {
        let root = tempfile::tempdir().unwrap();
        let service = Service {
            queue: Queue::open(root.path()).unwrap(),
            token: Arc::from("test-token"),
            capabilities: OmrCapabilities {
                service_version: "test".into(),
                audiveris_version: "5.11.0".into(),
                formats: vec![OmrSourceFormat::Png],
                languages: vec!["eng".into()],
                max_upload_bytes: 128 * 1024 * 1024,
                max_job_seconds: 1800,
            },
            cancels: Arc::new(Mutex::new(HashMap::<_, Arc<AtomicBool>>::new())),
            process: ProcessConfig {
                audiveris_bin: PathBuf::from("/bin/true"),
                tessdata_dir: root.path().join("tessdata"),
                languages: vec!["eng".into()],
                max_job_seconds: 1800,
                log_limit_bytes: 16 * 1024 * 1024,
            },
        };
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let _ = axum::serve(listener, router(service)).await;
        });
        let client = reqwest::Client::new();
        assert!(client
            .get(format!("http://{address}/v1/health"))
            .send()
            .await
            .unwrap()
            .status()
            .is_success());
        assert_eq!(
            client
                .get(format!("http://{address}/v1/capabilities"))
                .send()
                .await
                .unwrap()
                .status(),
            StatusCode::UNAUTHORIZED
        );
        let capabilities: OmrCapabilities = client
            .get(format!("http://{address}/v1/capabilities"))
            .bearer_auth("test-token")
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        assert_eq!(capabilities.audiveris_version, "5.11.0");

        let form = Form::new()
            .text("metadata", r#"{"sourceFormat":"png"}"#)
            .part(
                "file",
                Part::bytes(png(1, 1))
                    .file_name("input.png")
                    .mime_str("image/png")
                    .unwrap(),
            );
        let job: OmrJob = client
            .post(format!("http://{address}/v1/jobs"))
            .bearer_auth("test-token")
            .multipart(form)
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        assert_eq!(job.state, JobState::Queued);
        assert_eq!(job.queue_position, Some(0));
        assert_eq!(job.audiveris_version.as_deref(), Some("5.11.0"));
        server.abort();
    }

    fn png(width: u32, height: u32) -> Vec<u8> {
        let mut bytes = b"\x89PNG\r\n\x1a\n".to_vec();
        bytes.extend(13u32.to_be_bytes());
        bytes.extend(b"IHDR");
        bytes.extend(width.to_be_bytes());
        bytes.extend(height.to_be_bytes());
        bytes.extend([8, 6, 0, 0, 0]);
        bytes.extend([0; 4]);
        bytes.extend(0u32.to_be_bytes());
        bytes.extend(b"IEND");
        bytes.extend([0; 4]);
        bytes
    }
}
