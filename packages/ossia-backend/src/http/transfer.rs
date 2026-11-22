use super::WebState;
use crate::*;
use axum::{
    extract::{FromRequest, Multipart, Request},
    http::StatusCode,
    response::{
        sse::{Event, KeepAlive, Sse},
        IntoResponse, Response,
    },
};
use serde_json::{json, Value};
use std::{convert::Infallible, sync::Arc, time::Duration};

pub(super) async fn upload(runtime: Arc<Runtime>, request: Request) -> Result<Response> {
    let mut multipart = Multipart::from_request(request, &())
        .await
        .map_err(|_| invalid("expected multipart/form-data"))?;
    let mut metadata = None;
    let mut bytes = None;
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
                    .map_err(|_| invalid("invalid metadata"))?;
                let mut data: Value =
                    serde_json::from_str(&text).map_err(|_| invalid("metadata must be JSON"))?;
                if !data.is_object() {
                    return Err(invalid("metadata must be an object"));
                }
                if data.get("path").is_some() {
                    return Err(invalid("page uploads cannot import local paths"));
                }
                data["path"] = json!("");
                metadata = Some(decode::<ImportArgs>(data)?);
            }
            Some("file") if bytes.is_none() => {
                let data = field
                    .bytes()
                    .await
                    .map_err(|_| invalid("invalid file upload"))?;
                if data.len() > 128 * 1024 * 1024 {
                    return Err(invalid("file exceeds 128 MiB"));
                }
                bytes = Some(data.to_vec());
            }
            _ => return Err(invalid("expected one metadata and one file field")),
        }
    }
    let job = runtime.import(
        metadata.ok_or_else(|| invalid("missing metadata"))?,
        bytes.ok_or_else(|| invalid("missing file"))?,
    )?;
    let mut job = value(job)?;
    normalize_outbound_paths(&mut job);
    Ok((StatusCode::ACCEPTED, axum::Json(job)).into_response())
}

pub(super) fn events(state: WebState, query: &str) -> Result<Response> {
    let mut instance = None;
    let mut after = None;
    let mut document = None;
    for pair in query.split('&') {
        if let Some((key, value)) = pair.split_once('=') {
            match key {
                "instanceId" => instance = Some(value.to_string()),
                "after" => {
                    after = Some(
                        value
                            .parse::<u64>()
                            .map_err(|_| invalid("invalid offset"))?,
                    )
                }
                "documentId" => {
                    document = Some(
                        value
                            .parse::<DocumentId>()
                            .map_err(|_| invalid("invalid identifier"))?,
                    )
                }
                _ => {}
            }
        }
    }
    let instance = instance.ok_or_else(|| invalid("events requires snapshot instanceId"))?;
    let mut after = after.ok_or_else(|| invalid("events requires snapshot cursor after"))?;
    state.runtime.events(document, &instance, after)?;
    let stream = async_stream::stream! {loop{
        let notified=state.runtime.changed.notified();
        match state.runtime.events(document,&instance,after){Ok(events)=>{for event in events{after=event.seq;yield Ok::<_,Infallible>(Event::default().id(event.seq.to_string()).event("runtime").data(event_data(&event)));}},Err(_)=>{yield Ok(Event::default().event("resync-required").data("{}"));break;}}
        if state.runtime.stopping.load(std::sync::atomic::Ordering::Acquire){break;}
        tokio::select!{_=notified=>{},_=tokio::time::sleep(Duration::from_secs(1))=>{}}
    }};
    Ok(Sse::new(stream)
        .keep_alive(KeepAlive::new().interval(Duration::from_secs(15)))
        .into_response())
}

pub(super) fn event_data(event: &RuntimeEvent) -> String {
    let mut value = serde_json::to_value(event).expect("event serialization");
    normalize_outbound_paths(&mut value);
    serde_json::to_string(&value).expect("event serialization")
}

#[cfg(test)]
#[path = "../../tests/unit/http_transfer.rs"]
mod tests;
