//! Management-window projections for browsing and maintaining the local score library.
use crate::*;

/// One active or archived score shown in the management window.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub struct ManagementScoreSummary {
    pub document_id: DocumentId,
    pub title: String,
    pub created_at: String,
    pub updated_at: String,
    pub archived_at: Option<String>,
    /// Composer used to group and filter the library.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub composer: Option<String>,
    /// Instruments the score is written for; absent when none are recorded.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub instruments: Option<Vec<String>>,
    /// Musical period or style, such as Baroque, Classical or Romantic.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub period: Option<String>,
}

/// One bounded page of management score metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub struct ManagementScorePage {
    pub items: Vec<ManagementScoreSummary>,
    pub total: usize,
    pub offset: usize,
    pub next_offset: Option<usize>,
}

/// Request a cached first-page preview, optionally retrying the latest failed generation.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema, ts_rs::TS,
)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ManagementScorePreviewArgs {
    pub document_id: DocumentId,
    #[serde(default)]
    pub retry: bool,
}

/// Availability of one management score's first-page preview.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(
    tag = "state",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
pub enum ManagementScorePreview {
    Ready { data_url: String },
    Pending { job_id: JobId },
    Failed { error: AppError },
    Missing,
}
