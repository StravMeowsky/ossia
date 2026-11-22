use crate::*;
use axum::{
    body::to_bytes,
    extract::{DefaultBodyLimit, OriginalUri, Request, State},
    http::{header, HeaderValue, StatusCode, Uri},
    middleware::{self, Next},
    response::{IntoResponse, Redirect, Response},
    routing::{any, get},
    Router,
};
use serde_json::{json, Value};
use std::{path::PathBuf, sync::Arc};
use tower_http::services::ServeFile;

const CLIENT_ID_HEADER: &str = "x-ossia-client-id";

mod api;
mod transfer;

use api::api;

#[derive(Clone)]
pub struct WebState {
    pub runtime: Arc<Runtime>,
    pub dev_origin: Option<String>,
    pub allowed_hosts: Vec<String>,
}

impl IntoResponse for AppErrorResponse {
    fn into_response(self) -> Response {
        let mut error = self.0;
        normalize_outbound_error(&mut error);
        let code = match error.code {
            ErrorCode::Unauthorized => StatusCode::UNAUTHORIZED,
            ErrorCode::Forbidden => StatusCode::FORBIDDEN,
            ErrorCode::NotFound => StatusCode::NOT_FOUND,
            ErrorCode::Conflict
            | ErrorCode::EditBusy
            | ErrorCode::DirtyWorkspace
            | ErrorCode::StaleSelection
            | ErrorCode::LibraryBusy => StatusCode::CONFLICT,
            ErrorCode::InvalidRequest | ErrorCode::InvalidInput | ErrorCode::InvalidEncoding => {
                StatusCode::BAD_REQUEST
            }
            ErrorCode::StorageError => StatusCode::INTERNAL_SERVER_ERROR,
            ErrorCode::RuntimeUnavailable => StatusCode::SERVICE_UNAVAILABLE,
            _ => StatusCode::UNPROCESSABLE_ENTITY,
        };
        (code, axum::Json(json!({"error":error}))).into_response()
    }
}

pub struct AppErrorResponse(pub AppError);

impl From<AppError> for AppErrorResponse {
    fn from(error: AppError) -> Self {
        Self(error)
    }
}

pub fn router(
    runtime: Arc<Runtime>,
    static_dir: Option<PathBuf>,
    dev_origin: Option<String>,
    allowed_hosts: Vec<String>,
) -> Router {
    let dev_redirect = dev_origin.clone();
    let state = WebState {
        runtime,
        dev_origin,
        allowed_hosts,
    };
    let mut app = Router::new()
        .route(
            "/health",
            get(|| async { axum::Json(json!({"alive":true,"protocolVersion":PROTOCOL_VERSION})) }),
        )
        .route("/api/v1/{*path}", any(api))
        .layer(DefaultBodyLimit::max(129 * 1024 * 1024));
    if let Some(path) = static_dir {
        app = app.route_service("/", ServeFile::new(path.join("index.html")));
        app = match dev_redirect {
            Some(origin) => app.route(
                "/scores",
                get(move |OriginalUri(uri): OriginalUri| {
                    let origin = origin.clone();
                    async move {
                        let target = uri
                            .path_and_query()
                            .map_or_else(|| origin.clone(), |path| format!("{origin}{path}"));
                        Redirect::temporary(&target)
                    }
                }),
            ),
            None => app.route_service("/scores", ServeFile::new(path.join("scores.html"))),
        };
        app = app.fallback_service(tower_http::services::ServeDir::new(path));
    }
    app.layer(middleware::from_fn_with_state(state.clone(), boundary))
        .with_state(state)
}

async fn boundary(State(state): State<WebState>, request: Request, next: Next) -> Response {
    let base = state.runtime.info.web_base_url.as_str();
    let expected_port = Uri::try_from(base)
        .ok()
        .and_then(|uri| uri.port_u16())
        .unwrap_or(80);
    let valid_host = request
        .headers()
        .get(header::HOST)
        .and_then(|host| host.to_str().ok())
        .is_some_and(|host| allowed_host(host, expected_port, &state.allowed_hosts));
    if !valid_host {
        return AppErrorResponse(AppError::new(ErrorCode::Forbidden, "invalid Host"))
            .into_response();
    }
    let origin = request.headers().get(header::ORIGIN).cloned();
    if let Some(origin) = &origin {
        let allowed = origin.to_str().ok().is_some_and(|s| {
            state.dev_origin.as_deref() == Some(s)
                || allowed_origin(s, expected_port, &state.allowed_hosts)
        });
        if !allowed {
            return AppErrorResponse(AppError::new(ErrorCode::Forbidden, "Origin is not allowed"))
                .into_response();
        }
    }
    let mut response = if request.method() == axum::http::Method::OPTIONS {
        (
            StatusCode::NO_CONTENT,
            [
                (header::ACCESS_CONTROL_ALLOW_METHODS, "GET, POST, PUT"),
                (
                    header::ACCESS_CONTROL_ALLOW_HEADERS,
                    "Content-Type, X-Ossia-Client-Id",
                ),
            ],
        )
            .into_response()
    } else {
        next.run(request).await
    };
    if let Some(origin) = origin {
        response
            .headers_mut()
            .insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, origin);
        response
            .headers_mut()
            .insert(header::VARY, HeaderValue::from_static("Origin"));
    }
    response.headers_mut().insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("no-referrer"),
    );
    response.headers_mut().insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    response
}

fn normalized_host(host: &str) -> String {
    host.trim_start_matches('[')
        .trim_end_matches(']')
        .to_ascii_lowercase()
}

fn request_host(host: &str) -> Option<(String, u16)> {
    let authority: axum::http::uri::Authority = host.parse().ok()?;
    Some((normalized_host(authority.host()), authority.port_u16()?))
}

fn is_loopback_name(host: &str) -> bool {
    matches!(host, "localhost" | "127.0.0.1" | "::1")
}

fn allowed_host(host: &str, port: u16, allowed_hosts: &[String]) -> bool {
    request_host(host).is_some_and(|(name, actual)| {
        actual == port
            && (is_loopback_name(&name)
                || allowed_hosts
                    .iter()
                    .any(|allowed| normalized_host(allowed) == name))
    })
}

fn origin_host_port(origin: &str) -> Option<(String, u16)> {
    let uri = Uri::try_from(origin).ok()?;
    if uri.scheme_str() != Some("http") {
        return None;
    }
    Some((normalized_host(uri.host()?), uri.port_u16()?))
}

fn allowed_origin(origin: &str, port: u16, allowed_hosts: &[String]) -> bool {
    origin_host_port(origin).is_some_and(|(host, actual)| {
        actual == port
            && (is_loopback_name(&host)
                || allowed_hosts
                    .iter()
                    .any(|allowed| normalized_host(allowed) == host))
    })
}

fn id<T: std::str::FromStr>(text: &str) -> Result<T> {
    text.parse().map_err(|_| invalid("invalid identifier"))
}

async fn body(request: Request, limit: usize) -> Result<Value> {
    let bytes = to_bytes(request.into_body(), limit)
        .await
        .map_err(|_| invalid("invalid or oversized JSON request"))?;
    serde_json::from_slice(&bytes).map_err(|e| invalid(e.to_string()))
}

fn bind_document(data: &Value, document: DocumentId) -> Result<()> {
    if data.get("documentId").and_then(Value::as_str) != Some(document.to_string().as_str()) {
        return Err(invalid("body documentId must match route"));
    }
    Ok(())
}

fn page_args(query: &str) -> Result<ListArgs> {
    let mut limit = None;
    let mut offset = 0;
    for pair in query.split('&').filter(|pair| !pair.is_empty()) {
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        match key {
            "limit" => limit = Some(value.parse().map_err(|_| invalid("invalid limit"))?),
            "offset" => offset = value.parse().map_err(|_| invalid("invalid offset"))?,
            _ => {}
        }
    }
    Ok(ListArgs { limit, offset })
}

fn job_args(query: &str, job_id: JobId, action: JobAction) -> Result<JobArgs> {
    let mut args = JobArgs {
        detail: DetailLevel::Summary,
        job_id,
        action,
        wait_ms: None,
        section: None,
        offset: 0,
        limit: None,
        measure_range: None,
        voice: None,
        track: None,
        channel: None,
    };
    for pair in query.split('&').filter(|pair| !pair.is_empty()) {
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        match key {
            "detail" => {
                args.detail = match value {
                    "summary" => DetailLevel::Summary,
                    "full" => DetailLevel::Full,
                    _ => return Err(invalid("invalid detail")),
                }
            }
            "waitMs" => args.wait_ms = Some(value.parse().map_err(|_| invalid("invalid waitMs"))?),
            "section" => {
                args.section = Some(match value {
                    "performance" => AnalysisSection::Performance,
                    "source" => AnalysisSection::Source,
                    "differences" => AnalysisSection::Differences,
                    _ => return Err(invalid("invalid section")),
                })
            }
            "offset" => args.offset = value.parse().map_err(|_| invalid("invalid offset"))?,
            "limit" => args.limit = Some(value.parse().map_err(|_| invalid("invalid limit"))?),
            _ => {}
        }
    }
    Ok(args)
}

#[cfg(test)]
#[path = "../tests/unit/http.rs"]
mod tests;
