//! The score window's assistant: runs Claude Code in print mode with this instance's Ossia MCP
//! server and streams its replies to the window that asked.

use crate::{config, integrator, storage, Management};
use ossia_backend::{AppError, ErrorCode, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::HashMap,
    io::{BufRead, BufReader, Read},
    path::Path,
    process::{Child, Command, Stdio},
    sync::{Arc, Mutex},
};
use tauri::{Emitter, Manager, WebviewWindow};

/// Event the score page listens to for assistant progress.
pub const EVENT: &str = "assistant-event";
/// Only score windows may run the assistant.
const WINDOW_PREFIX: &str = "score-";

#[derive(Default)]
pub struct Assistant {
    runs: Mutex<HashMap<String, Arc<Mutex<Child>>>>,
}

impl Assistant {
    /// Stop the run belonging to a window, if any.
    pub fn stop(&self, label: &str) {
        let run = self.runs.lock().ok().and_then(|mut runs| runs.remove(label));
        if let Some(child) = run {
            if let Ok(mut child) = child.lock() {
                let _ = child.kill();
            }
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssistantSendArgs {
    document_id: String,
    title: String,
    message: String,
    session: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum AssistantEvent {
    /// Claude Code started a conversation; pass it back to continue it.
    Session { session: String },
    /// A piece of the assistant's reply.
    Text { text: String },
    /// The assistant is using an Ossia tool.
    Tool { name: String },
    /// The run finished, successfully or with an error message.
    Done {
        session: Option<String>,
        error: Option<String>,
    },
}

/// Translate one line of Claude Code's `stream-json` output into assistant events.
pub fn parse_line(line: &str) -> Vec<AssistantEvent> {
    let Ok(value) = serde_json::from_str::<Value>(line) else {
        return Vec::new();
    };
    let text = |value: &Value, key: &str| value.get(key).and_then(Value::as_str).map(str::to_string);
    match value.get("type").and_then(Value::as_str) {
        Some("system") if value.get("subtype").and_then(Value::as_str) == Some("init") => {
            text(&value, "session_id")
                .map(|session| vec![AssistantEvent::Session { session }])
                .unwrap_or_default()
        }
        Some("assistant") => value
            .pointer("/message/content")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|item| match item.get("type").and_then(Value::as_str) {
                Some("text") => text(item, "text")
                    .filter(|text| !text.trim().is_empty())
                    .map(|text| AssistantEvent::Text { text }),
                Some("tool_use") => text(item, "name").map(|name| AssistantEvent::Tool {
                    name: name
                        .strip_prefix("mcp__ossia__")
                        .unwrap_or(&name)
                        .to_string(),
                }),
                _ => None,
            })
            .collect(),
        Some("result") => {
            let failed = value.get("is_error").and_then(Value::as_bool) == Some(true)
                || value.get("subtype").and_then(Value::as_str) != Some("success");
            vec![AssistantEvent::Done {
                session: text(&value, "session_id"),
                error: failed.then(|| {
                    text(&value, "result")
                        .filter(|text| !text.trim().is_empty())
                        .or_else(|| text(&value, "subtype"))
                        .unwrap_or_else(|| "Claude Code failed".into())
                }),
            }]
        }
        _ => Vec::new(),
    }
}

/// Guide documents nearly every score task reads; giving them up front saves `ossia_guide` rounds.
const PRELOADED_GUIDES: [&str; 2] = ["guide.md", "guide/editing.md"];

fn system_prompt(instructions: &str, document_id: &str, title: &str) -> String {
    let guides = PRELOADED_GUIDES
        .iter()
        .filter_map(|name| {
            ossia_backend::guide_document(name)
                .map(|text| format!("<ossia-guide document=\"{name}\">\n{text}\n</ossia-guide>"))
        })
        .collect::<Vec<_>>()
        .join("\n\n");
    format!(
        "{instructions}\n\n## Preloaded guide\n\n\
         These are the `ossia_guide` results for {preloaded}. They are already read: do not \
         call `ossia_guide` for them. Read other guide documents only when the task needs \
         them.\n\n{guides}\n\n## This conversation\n\n\
         The user is chatting from the Ossia score window that shows the score \
         \"{title}\" (documentId `{document_id}`). Work on this score unless the user names \
         another one. Use `ossia_context` to see what the user selected or annotated. The \
         window refreshes by itself after you save changes, so do not include score links, \
         whatever the guide says about them. Keep replies short.\n\n## Working efficiently\n\n\
         Use as few tool calls as the task allows. Read only the source you need. Set \
         `\"preview\": false` on `ossia_write_source` when you check right after, and pass \
         `waitMs` to `ossia_check` instead of polling `ossia_job`. Once a check passes, \
         commit; inspect jobs, layout or evidence again only when a check fails or the \
         user asked for it.",
        preloaded = PRELOADED_GUIDES
            .iter()
            .map(|name| format!("`{name}`"))
            .collect::<Vec<_>>()
            .join(" and "),
    )
}

/// GUI apps on macOS start with a minimal PATH; Claude Code installed through npm needs `node`.
fn search_path(claude: &Path) -> std::ffi::OsString {
    let mut dirs: Vec<std::path::PathBuf> = claude.parent().map(Path::to_path_buf).into_iter().collect();
    dirs.extend(["/opt/homebrew/bin", "/usr/local/bin"].map(Into::into));
    if let Some(path) = std::env::var_os("PATH") {
        dirs.extend(std::env::split_paths(&path));
    }
    std::env::join_paths(dirs).unwrap_or_default()
}

fn emit(window: &WebviewWindow, event: AssistantEvent) {
    let _ = window.emit_to(window.label(), EVENT, event);
}

#[tauri::command]
pub(crate) async fn assistant_send(
    window: WebviewWindow,
    management: tauri::State<'_, Management>,
    assistant: tauri::State<'_, Assistant>,
    args: AssistantSendArgs,
) -> Result<()> {
    let label = window.label().to_string();
    if !label.starts_with(WINDOW_PREFIX) {
        return Err(AppError::new(
            ErrorCode::Forbidden,
            "the assistant is only available in score windows",
        ));
    }
    if args.message.trim().is_empty() {
        return Err(AppError::new(ErrorCode::InvalidInput, "message is empty"));
    }
    let config = config(&management)?;
    let launch = tauri::async_runtime::spawn_blocking({
        let config = config.clone();
        move || integrator(&config)?.assistant_launch()
    })
    .await
    .map_err(storage)??;
    let mut command = Command::new(&launch.claude);
    command
        .arg("-p")
        .arg(&args.message)
        .args(["--output-format", "stream-json", "--verbose"])
        .arg("--mcp-config")
        .arg(launch.mcp_config.to_string())
        .arg("--strict-mcp-config")
        .args(["--allowedTools", "mcp__ossia"])
        .arg("--append-system-prompt")
        .arg(system_prompt(&launch.instructions, &args.document_id, &args.title));
    if let Some(session) = args.session.as_deref().filter(|s| !s.is_empty()) {
        command.args(["--resume", session]);
    }
    command
        .current_dir(&config.home)
        .env("PATH", search_path(&launch.claude))
        // Load the Ossia tool definitions up front instead of searching for them each run.
        .env("ENABLE_TOOL_SEARCH", "false")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    assistant.stop(&label);
    let mut child = command.spawn().map_err(|error| {
        AppError::new(
            ErrorCode::StorageError,
            format!("cannot start Claude Code: {error}"),
        )
    })?;
    let stdout = child.stdout.take().ok_or_else(|| storage("missing Claude Code output"))?;
    let mut stderr = child.stderr.take().ok_or_else(|| storage("missing Claude Code errors"))?;
    let child = Arc::new(Mutex::new(child));
    assistant
        .runs
        .lock()
        .map_err(storage)?
        .insert(label.clone(), child.clone());
    std::thread::spawn(move || {
        let errors = std::thread::spawn(move || {
            let mut text = String::new();
            let _ = stderr.read_to_string(&mut text);
            text
        });
        let mut finished = false;
        for line in BufReader::new(stdout).lines().map_while(std::result::Result::ok) {
            for event in parse_line(&line) {
                finished |= matches!(event, AssistantEvent::Done { .. });
                emit(&window, event);
            }
        }
        let status = child.lock().ok().and_then(|mut child| child.wait().ok());
        let errors = errors.join().unwrap_or_default();
        if !finished {
            let stopped = status.is_some_and(|status| !status.success()) && errors.trim().is_empty();
            emit(
                &window,
                AssistantEvent::Done {
                    session: None,
                    error: Some(if stopped {
                        "stopped".into()
                    } else {
                        errors
                            .lines()
                            .rev()
                            .find(|line| !line.trim().is_empty())
                            .unwrap_or("Claude Code exited without a reply")
                            .to_string()
                    }),
                },
            );
        }
        if let Some(assistant) = window.try_state::<Assistant>() {
            if let Ok(mut runs) = assistant.runs.lock() {
                if runs.get(window.label()).is_some_and(|run| Arc::ptr_eq(run, &child)) {
                    runs.remove(window.label());
                }
            }
        }
    });
    Ok(())
}

#[tauri::command]
pub(crate) fn assistant_stop(window: WebviewWindow, assistant: tauri::State<'_, Assistant>) {
    assistant.stop(window.label());
}

#[cfg(test)]
#[path = "../tests/unit/assistant.rs"]
mod tests;
