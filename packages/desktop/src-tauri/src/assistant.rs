//! Runs the selected assistant CLI with this instance’s Ossia MCP server.

use crate::{config, integrator, storage, Management};
use ossia_backend::{AppError, ErrorCode, Result};
use ossia_integrations::AssistantLaunch;
use ossia_types::{AssistantEvent, AssistantProvider, AssistantSendArgs, AssistantUpdate};
use serde_json::{json, Value};
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
        let run = self
            .runs
            .lock()
            .ok()
            .and_then(|mut runs| runs.remove(label));
        if let Some(child) = run {
            if let Ok(mut child) = child.lock() {
                let _ = child.kill();
            }
        }
    }
}

/// Translate one line of Claude Code's `stream-json` output into assistant events.
pub fn parse_line(line: &str) -> Vec<AssistantEvent> {
    let Ok(value) = serde_json::from_str::<Value>(line) else {
        return Vec::new();
    };
    let text =
        |value: &Value, key: &str| value.get(key).and_then(Value::as_str).map(str::to_string);
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

/// Translate a Codex JSONL event. Reasoning and tools from other servers stay out of the chat.
fn parse_codex_line(line: &str, stream_error: &mut Option<String>) -> Vec<AssistantEvent> {
    let Ok(value) = serde_json::from_str::<Value>(line) else {
        return Vec::new();
    };
    let event = match value["type"].as_str() {
        Some("thread.started") => {
            value["thread_id"]
                .as_str()
                .map(|session| AssistantEvent::Session {
                    session: session.into(),
                })
        }
        Some("item.started")
            if value["item"]["type"] == "mcp_tool_call" && value["item"]["server"] == "ossia" =>
        {
            value["item"]["tool"]
                .as_str()
                .map(|name| AssistantEvent::Tool { name: name.into() })
        }
        Some("item.completed") if value["item"]["type"] == "agent_message" => value["item"]["text"]
            .as_str()
            .filter(|text| !text.trim().is_empty())
            .map(|text| AssistantEvent::Text { text: text.into() }),
        Some("error") => {
            // Codex can emit errors while reconnecting. Only surface one if the run cannot finish.
            *stream_error = value["message"].as_str().map(str::to_string);
            None
        }
        Some("turn.completed") => Some(AssistantEvent::Done {
            session: None,
            error: None,
        }),
        Some("turn.failed") => Some(AssistantEvent::Done {
            session: None,
            error: Some(
                value["error"]["message"]
                    .as_str()
                    .unwrap_or("Codex failed")
                    .into(),
            ),
        }),
        _ => None,
    };
    event.into_iter().collect()
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

/// GUI apps on macOS start with a minimal PATH; npm-installed CLIs need `node`.
fn search_path(executable: &Path) -> std::ffi::OsString {
    let mut dirs: Vec<std::path::PathBuf> = executable
        .parent()
        .map(Path::to_path_buf)
        .into_iter()
        .collect();
    dirs.extend(["/opt/homebrew/bin", "/usr/local/bin"].map(Into::into));
    if let Some(path) = std::env::var_os("PATH") {
        dirs.extend(std::env::split_paths(&path));
    }
    std::env::join_paths(dirs).unwrap_or_default()
}

fn emit(window: &WebviewWindow, run_id: &str, event: AssistantEvent) {
    let _ = window.emit_to(
        window.label(),
        EVENT,
        AssistantUpdate {
            run_id: run_id.into(),
            event,
        },
    );
}

fn assistant_command(launch: &AssistantLaunch, args: &AssistantSendArgs) -> Command {
    let prompt = system_prompt(&launch.instructions, &args.document_id, &args.title);
    let session = args
        .session
        .as_deref()
        .filter(|session| !session.is_empty());
    let mut command = Command::new(&launch.executable);
    match args.provider {
        AssistantProvider::ClaudeCode => {
            let mcp_config = json!({
                "mcpServers": {
                    "ossia": {
                        "command": launch.mcp_command,
                        "args": launch.mcp_args,
                        "env": launch.mcp_env,
                    }
                }
            });
            command
                .args([
                    "-p",
                    "--output-format",
                    "stream-json",
                    "--verbose",
                    "--mcp-config",
                ])
                .arg(mcp_config.to_string())
                .args(["--strict-mcp-config", "--allowedTools", "mcp__ossia"])
                .arg("--append-system-prompt")
                .arg(prompt)
                .env("ENABLE_TOOL_SEARCH", "false");
            if let Some(session) = session {
                command.args(["--resume", session]);
            }
            command.arg("--");
        }
        AssistantProvider::Codex => {
            let env = launch
                .mcp_env
                .iter()
                .map(|(key, value)| format!("{} = {}", json!(key), json!(value)))
                .collect::<Vec<_>>()
                .join(", ");
            // Keep the user's sign-in and use the MCP server for this running Ossia instance.
            command
                .args(["exec", "--json", "--skip-git-repo-check", "--ignore-user-config",
                       "--sandbox", "read-only"])
                .arg("-c")
                .arg(format!("developer_instructions={}", json!(prompt)))
                .arg("-c")
                .arg(format!(
                    "mcp_servers.ossia={{ command = {}, args = {}, env = {{ {env} }}, required = true }}",
                    json!(launch.mcp_command),
                    json!(launch.mcp_args),
                ))
                .env("CODEX_HOME", &launch.codex_home);
            if let Some(session) = session {
                command.args(["resume", "--", session]);
            } else {
                command.arg("--");
            }
        }
    }
    command
        .arg(&args.message)
        .env("PATH", search_path(&launch.executable))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    command
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
    let launch = integrator(&config)?.assistant_launch(args.provider)?;
    let name = match args.provider {
        AssistantProvider::ClaudeCode => "Claude Code",
        AssistantProvider::Codex => "Codex",
    };
    let mut command = assistant_command(&launch, &args);
    command.current_dir(&config.home);
    assistant.stop(&label);
    let mut child = command.spawn().map_err(|error| {
        AppError::new(
            ErrorCode::StorageError,
            format!("cannot start {name}: {error}"),
        )
    })?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| storage(format!("missing {name} output")))?;
    let mut stderr = child
        .stderr
        .take()
        .ok_or_else(|| storage(format!("missing {name} errors")))?;
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
        let mut done = None;
        let mut stream_error = None;
        for line in BufReader::new(stdout)
            .lines()
            .map_while(std::result::Result::ok)
        {
            let events = match args.provider {
                AssistantProvider::ClaudeCode => parse_line(&line),
                AssistantProvider::Codex => parse_codex_line(&line, &mut stream_error),
            };
            for event in events {
                if matches!(event, AssistantEvent::Done { .. }) {
                    done = Some(event);
                } else {
                    emit(&window, &args.run_id, event);
                }
            }
        }
        if let Ok(mut child) = child.lock() {
            let _ = child.wait();
        }
        let errors = errors.join().unwrap_or_default();
        let mut stopped = false;
        if let Some(assistant) = window.try_state::<Assistant>() {
            if let Ok(mut runs) = assistant.runs.lock() {
                if runs.get(&label).is_some_and(|run| Arc::ptr_eq(run, &child)) {
                    runs.remove(&label);
                } else {
                    stopped = true;
                }
            }
        }
        let event = if stopped {
            AssistantEvent::Done {
                session: None,
                error: Some("stopped".into()),
            }
        } else {
            done.unwrap_or_else(|| AssistantEvent::Done {
                session: None,
                error: Some(
                    stream_error
                        .or_else(|| {
                            errors
                                .lines()
                                .rev()
                                .find(|line| !line.trim().is_empty())
                                .map(str::to_string)
                        })
                        .unwrap_or_else(|| format!("{name} exited without a reply")),
                ),
            })
        };
        emit(&window, &args.run_id, event);
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
