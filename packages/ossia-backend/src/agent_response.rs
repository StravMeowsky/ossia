//! Compact MCP and CLI responses without changing backend or browser contracts.

use crate::{storage, Result};
use serde_json::{json, Map, Value};

fn object(value: &mut Value) -> Result<&mut Map<String, Value>> {
    value
        .as_object_mut()
        .ok_or_else(|| storage("invalid Agent response object"))
}

fn remove(value: &mut Value, fields: &[&str]) -> Result<()> {
    let value = object(value)?;
    for field in fields {
        value.remove(*field);
    }
    Ok(())
}

fn compact_render(render: &mut Value) -> Result<()> {
    let render = object(render)?;
    let pages = render
        .remove("pages")
        .and_then(|v| v.as_array().map(Vec::len))
        .ok_or_else(|| storage("check result is missing render pages"))?;
    let pdf = render
        .remove("pdf")
        .ok_or_else(|| storage("check result is missing PDF status"))?;
    let midi = render
        .remove("midi")
        .ok_or_else(|| storage("check result is missing MIDI status"))?;
    render.remove("engineBuildId");
    render.remove("optionsHash");
    render.insert("pageCount".into(), json!(pages));
    render.insert("pdfAvailable".into(), json!(!pdf.is_null()));
    render.insert("midiAvailable".into(), json!(!midi.is_null()));
    Ok(())
}

fn compact_output(result: &mut Value) -> Result<()> {
    remove(result, &["engineBuildId", "sha256", "mediaType"])?;
    if let Some(files) = result.get_mut("files") {
        for file in files
            .as_array_mut()
            .ok_or_else(|| storage("invalid output file list"))?
        {
            remove(file, &["sha256", "mediaType"])?;
        }
    }
    Ok(())
}

fn compact_job(job: &mut Value) -> Result<()> {
    let kind = job
        .get("kind")
        .and_then(Value::as_str)
        .ok_or_else(|| storage("job has no kind"))?
        .to_owned();
    let Some(result) = job.get_mut("result").filter(|result| !result.is_null()) else {
        return Ok(());
    };
    match kind.as_str() {
        "check" | "import" => {
            if let Some(render) = result.get_mut("render").filter(|render| !render.is_null()) {
                compact_render(render)?;
            }
        }
        "import-request" => {
            if let Some(report) = result.get_mut("report").filter(|report| !report.is_null()) {
                remove(report, &["engineBuildId"])?;
            }
            if let Some(document) = result
                .get_mut("document")
                .filter(|document| !document.is_null())
            {
                remove(document, &["sourcePath"])?;
            }
        }
        "output" => compact_output(result)?,
        _ => {}
    }
    Ok(())
}

pub(crate) fn compact(method: &str, args: &Value, mut result: Value) -> Result<Value> {
    match method {
        "status" => {
            remove(
                &mut result,
                &["instanceId", "seq", "engineBuildId", "documentCount"],
            )?;
            let runtime = result
                .get_mut("runtime")
                .ok_or_else(|| storage("status has no runtime"))?;
            let runtime = object(runtime)?;
            runtime.retain(|key, _| key == "webBaseUrl");
            let documentation = result
                .get_mut("documentation")
                .ok_or_else(|| storage("status has no documentation"))?;
            remove(documentation, &["root"])?;
        }
        "context" => {
            remove(&mut result, &["instanceId", "seq"])?;
            if let Some(render) = result.get_mut("render").filter(|render| !render.is_null()) {
                remove(render, &["renderKey"])?;
            }
        }
        "convert" => remove(&mut result, &["engineBuildId"])?,
        "docs" => remove(&mut result, &["root"])?,
        "check" | "import" | "output" => compact_job(&mut result)?,
        "job" if args.get("detail").and_then(Value::as_str) != Some("full") => {
            let job = result
                .get_mut("job")
                .ok_or_else(|| storage("job response has no job"))?;
            compact_job(job)?;
        }
        _ => {}
    }
    Ok(result)
}

#[cfg(test)]
#[path = "../tests/unit/agent_response.rs"]
mod tests;
