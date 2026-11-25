use crate::*;
use schemars::JsonSchema;
use ts_rs::TS;

/// Current stage of the desktop application update flow.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum UpdateState {
    Idle,
    Checking,
    AuthRequired,
    Latest,
    Source,
    NoRelease,
    Found,
    Downloading,
    Ready,
    Installing,
    Error,
}

impl UpdateState {
    pub fn in_flight(self) -> bool {
        matches!(self, Self::Checking | Self::Downloading | Self::Installing)
    }
}

/// Remote service the desktop updater reads releases from.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum UpdateSource {
    Ossia,
    #[default]
    Github,
}

/// Release channel the updater may install from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum UpdateChannel {
    Stable,
    Beta,
}

/// Saved channel preference. `Auto` follows the running build's pre-release state.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "snake_case")]
pub enum UpdateChannelPreference {
    #[default]
    Auto,
    Stable,
    Beta,
}

/// One release body retained for the release-notes dialog.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct ReleaseNote {
    pub version: String,
    pub body: String,
}

/// Bilingual release notes selected for the current update range.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct UpdateNotes {
    pub from: String,
    pub to: String,
    pub maintenance_only: bool,
    pub releases: Vec<ReleaseNote>,
}

/// Snapshot rendered by the management window.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSnapshot {
    pub state: UpdateState,
    pub is_debug_build: bool,
    pub current: String,
    pub source: UpdateSource,
    pub channel: UpdateChannel,
    pub latest: Option<String>,
    pub error: Option<String>,
    pub done: Option<u64>,
    pub total: Option<u64>,
    pub notes: Option<Box<UpdateNotes>>,
    pub staged: bool,
}
