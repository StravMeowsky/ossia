use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// CLI provider used by the desktop score assistant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
pub enum AssistantProvider {
    Codex,
    ClaudeCode,
}

/// One prompt for a score and its optional provider conversation.
#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct AssistantSendArgs {
    pub provider: AssistantProvider,
    pub run_id: String,
    pub document_id: String,
    pub title: String,
    pub message: String,
    pub session: Option<String>,
}

/// Progress reported by an assistant CLI and rendered in the score window.
#[derive(Debug, Clone, PartialEq, Serialize, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum AssistantEvent {
    Session {
        session: String,
    },
    Text {
        text: String,
    },
    Tool {
        name: String,
    },
    Done {
        session: Option<String>,
        error: Option<String>,
    },
}

/// An event correlated with the prompt that started its CLI process.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct AssistantUpdate {
    pub run_id: String,
    pub event: AssistantEvent,
}
