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

/// Staff layout of a new blank score.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema, ts_rs::TS,
)]
#[serde(rename_all = "kebab-case")]
pub enum NewScoreInstrument {
    /// Treble and bass staves in a piano staff.
    Piano,
    /// One treble-clef staff.
    Violin,
    /// One alto-clef staff.
    Viola,
    /// One bass-clef staff.
    Cello,
}

/// Mode of a new score's key.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema, ts_rs::TS,
)]
#[serde(rename_all = "kebab-case")]
pub enum NewScoreMode {
    Major,
    Minor,
}

/// Create a blank score whose source follows the engraving conventions: one music variable per
/// staff with clef, key and time at its start, empty measures and a final bar line.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ManagementCreateScoreArgs {
    /// Score title; also the library title.
    pub title: String,
    /// Composer as it should appear in the header and the library; omitted when blank.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub composer: Option<String>,
    pub instrument: NewScoreInstrument,
    /// Key signature from -7 (seven flats) to 7 (seven sharps).
    pub fifths: i8,
    pub mode: NewScoreMode,
    /// Beats per measure, 1..=32.
    pub time_numerator: u8,
    /// Beat unit: 1, 2, 4, 8, 16 or 32.
    pub time_denominator: u8,
    /// Tempo marking such as "Andante"; omitted when blank.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub tempo_text: Option<String>,
    /// Metronome mark in beats per minute, 20..=400.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub tempo_bpm: Option<u16>,
    /// Number of empty measures, 1..=999.
    pub measures: u16,
}

/// The library document created for a new score.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub struct ManagementCreatedScore {
    pub document_id: DocumentId,
}

/// Desktop request: create a score, open its window and optionally hand the assistant a
/// composition brief to write into it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ManagementNewScoreArgs {
    pub score: ManagementCreateScoreArgs,
    /// What the assistant should compose in the new score; omitted when blank.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub compose: Option<String>,
}
