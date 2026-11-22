use super::config::OmrConfig;
use crate::{invalid, storage, Result};
use ossia_engine::RunContext;
use ossia_types::{AppError, Diagnostic, ErrorCode, JobId, JobState, OmrJob, OmrSourceFormat};
use reqwest::{
    blocking::{
        multipart::{Form, Part},
        Client, Response,
    },
    StatusCode,
};
use serde::{de::DeserializeOwned, Deserialize};
use serde_json::json;
use std::time::Duration;

pub(crate) struct OmrClient {
    http: Client,
    config: OmrConfig,
}

impl OmrClient {
    pub fn new(config: OmrConfig) -> Result<Self> {
        let http = Client::builder()
            .connect_timeout(Duration::from_secs(15))
            .timeout(Duration::from_secs(30 * 60))
            .user_agent(format!("ossia-omr-client/{}", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|error| {
                AppError::new(
                    ErrorCode::RuntimeUnavailable,
                    format!("cannot create OMR HTTP client: {error}"),
                )
            })?;
        Ok(Self { http, config })
    }

    pub fn transcribe(
        &self,
        format: OmrSourceFormat,
        input: &[u8],
        context: &RunContext,
    ) -> Result<(Vec<u8>, Vec<Diagnostic>)> {
        let metadata = json!({"sourceFormat": format}).to_string();
        let part = Part::bytes(input.to_vec())
            .file_name(extension(format))
            .mime_str(mime(format))
            .map_err(|error| invalid(format!("cannot build OMR upload: {error}")))?;
        let form = Form::new().text("metadata", metadata).part("file", part);
        context.check()?;
        let response = self
            .http
            .post(format!("{}/v1/jobs", self.config.url))
            .bearer_auth(&self.config.token)
            .multipart(form)
            .send()
            .map_err(http_error)?;
        let job: OmrJob = decode(response)?;
        let result = self.wait(&job.job_id, context);
        match result {
            Ok((bytes, diagnostics)) => Ok((bytes, diagnostics)),
            Err(error) => {
                self.cancel_best_effort(job.job_id);
                Err(error)
            }
        }
    }

    fn wait(&self, id: &JobId, context: &RunContext) -> Result<(Vec<u8>, Vec<Diagnostic>)> {
        loop {
            context.check()?;
            let response = self
                .http
                .get(format!("{}/v1/jobs/{id}", self.config.url))
                .bearer_auth(&self.config.token)
                .send()
                .map_err(http_error)?;
            let job: OmrJob = decode(response)?;
            match job.state {
                JobState::Queued | JobState::Running => {
                    std::thread::sleep(Duration::from_secs(1));
                }
                JobState::Done => return self.download(id, job.diagnostics),
                JobState::Error => {
                    return Err(job.error.unwrap_or_else(|| {
                        AppError::new(ErrorCode::CompileFailed, "remote OMR job failed")
                    }))
                }
                JobState::Cancelled => {
                    return Err(AppError::new(
                        ErrorCode::Cancelled,
                        "remote OMR job was cancelled",
                    ))
                }
                JobState::Interrupted => {
                    return Err(AppError::new(
                        ErrorCode::RuntimeUnavailable,
                        "remote OMR job was interrupted by a service restart",
                    ))
                }
            }
        }
    }

    fn download(
        &self,
        id: &JobId,
        diagnostics: Vec<Diagnostic>,
    ) -> Result<(Vec<u8>, Vec<Diagnostic>)> {
        let response = self
            .http
            .get(format!("{}/v1/jobs/{id}/result", self.config.url))
            .bearer_auth(&self.config.token)
            .send()
            .map_err(http_error)?;
        if !response.status().is_success() {
            return Err(decode_error(response));
        }
        let bytes = response.bytes().map_err(http_error)?.to_vec();
        if bytes.is_empty() {
            return Err(AppError::new(
                ErrorCode::CompileFailed,
                "remote OMR returned an empty MusicXML result",
            ));
        }
        Ok((bytes, diagnostics))
    }

    fn cancel_best_effort(&self, id: JobId) {
        let http = self.http.clone();
        let url = format!("{}/v1/jobs/{id}/cancel", self.config.url);
        let token = self.config.token.clone();
        std::thread::spawn(move || {
            let _ = http
                .post(url)
                .bearer_auth(token)
                .timeout(Duration::from_secs(2))
                .send();
        });
    }
}

fn extension(format: OmrSourceFormat) -> &'static str {
    match format {
        OmrSourceFormat::Pdf => "input.pdf",
        OmrSourceFormat::Png => "input.png",
        OmrSourceFormat::Jpeg => "input.jpg",
        OmrSourceFormat::Tiff => "input.tiff",
    }
}

fn mime(format: OmrSourceFormat) -> &'static str {
    match format {
        OmrSourceFormat::Pdf => "application/pdf",
        OmrSourceFormat::Png => "image/png",
        OmrSourceFormat::Jpeg => "image/jpeg",
        OmrSourceFormat::Tiff => "image/tiff",
    }
}

fn decode<T: DeserializeOwned>(response: Response) -> Result<T> {
    if response.status().is_success() {
        response.json().map_err(|error| {
            AppError::new(
                ErrorCode::RuntimeUnavailable,
                format!("invalid remote OMR response: {error}"),
            )
        })
    } else {
        Err(decode_error(response))
    }
}

#[derive(Deserialize)]
struct ErrorBody {
    error: AppError,
}

fn decode_error(response: Response) -> AppError {
    let status = response.status();
    match response.json::<ErrorBody>() {
        Ok(body) => body.error,
        Err(_) => AppError::new(status_code(status), status_message(status)),
    }
}

fn status_code(status: StatusCode) -> ErrorCode {
    match status {
        StatusCode::UNAUTHORIZED => ErrorCode::Unauthorized,
        StatusCode::FORBIDDEN => ErrorCode::Forbidden,
        StatusCode::NOT_FOUND => ErrorCode::NotFound,
        StatusCode::CONFLICT => ErrorCode::Conflict,
        StatusCode::UNPROCESSABLE_ENTITY => ErrorCode::InvalidInput,
        _ => ErrorCode::RuntimeUnavailable,
    }
}

fn status_message(status: StatusCode) -> String {
    format!("remote OMR request failed with status {status}")
}

fn http_error(error: reqwest::Error) -> AppError {
    if error.is_timeout() {
        AppError::new(
            ErrorCode::RuntimeUnavailable,
            "remote OMR request timed out",
        )
    } else if error.is_connect() {
        AppError::new(
            ErrorCode::RuntimeUnavailable,
            "cannot connect to the remote OMR service",
        )
    } else {
        storage(format!("remote OMR request failed: {error}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        extract::State,
        http::{header, HeaderMap, StatusCode},
        response::{IntoResponse, Response},
        routing::{get, post},
        Json, Router,
    };
    use ossia_types::ContentHash;
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };

    #[derive(Clone)]
    struct TestState {
        job: OmrJob,
        polls: Arc<AtomicUsize>,
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn polls_a_mock_service_and_downloads_musicxml() {
        let job = OmrJob {
            job_id: JobId::new(),
            state: JobState::Queued,
            phase: "queued".into(),
            queue_position: Some(0),
            source_format: OmrSourceFormat::Pdf,
            input_hash: ContentHash::of(b"%PDF-1"),
            output_hash: None,
            audiveris_version: None,
            languages: vec!["eng".into()],
            diagnostics: vec![Diagnostic {
                severity: ossia_types::Severity::Warning,
                message: "mock notice".into(),
                raw_position: None,
                position: None,
            }],
            error: None,
        };
        let state = TestState {
            job,
            polls: Arc::new(AtomicUsize::new(0)),
        };
        let app = Router::new()
            .route("/v1/jobs", post(submit))
            .route("/v1/jobs/{id}", get(status))
            .route("/v1/jobs/{id}/result", get(result))
            .with_state(state);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        let url = format!("http://{address}");
        let (bytes, diagnostics) = tokio::task::spawn_blocking(move || {
            let client = OmrClient::new(OmrConfig {
                url,
                token: "test-token".into(),
            })
            .unwrap();
            client.transcribe(
                OmrSourceFormat::Pdf,
                b"%PDF-1",
                &RunContext::new(Duration::from_secs(10)),
            )
        })
        .await
        .unwrap()
        .unwrap();
        assert_eq!(bytes, b"<score-partwise/>");
        assert_eq!(diagnostics.len(), 1);
        server.abort();
    }

    async fn submit(headers: HeaderMap, State(state): State<TestState>) -> Response {
        if headers
            .get(header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            != Some("Bearer test-token")
        {
            return StatusCode::UNAUTHORIZED.into_response();
        }
        Json(state.job).into_response()
    }

    async fn status(State(state): State<TestState>) -> Json<OmrJob> {
        let mut job = state.job.clone();
        if state.polls.fetch_add(1, Ordering::AcqRel) == 0 {
            job.state = JobState::Running;
            job.phase = "audiveris".into();
            job.queue_position = None;
        } else {
            job.state = JobState::Done;
            job.phase = "finished".into();
        }
        Json(job)
    }

    async fn result() -> Response {
        (
            [(
                header::CONTENT_TYPE,
                "application/vnd.recordare.musicxml+xml",
            )],
            "<score-partwise/>",
        )
            .into_response()
    }
}
