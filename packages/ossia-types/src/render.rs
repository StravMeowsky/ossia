use crate::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactRef {
    /// Relative to the artifact root supplied by the caller.
    pub path: String,
    pub sha256: ContentHash,
    pub bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub struct RenderPage {
    pub page: u32,
    #[serde(flatten)]
    pub artifact: ArtifactRef,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub struct FormatError {
    pub format: RenderFormat,
    pub error: AppError,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema, ts_rs::TS)]
#[serde(rename_all = "camelCase")]
pub struct RenderManifest {
    pub source_hash: ContentHash,
    pub engine_build_id: String,
    pub options_hash: ContentHash,
    pub pages: Vec<RenderPage>,
    pub pdf: Option<ArtifactRef>,
    pub midi: Option<ArtifactRef>,
    pub diagnostics: Vec<Diagnostic>,
    pub format_errors: Vec<FormatError>,
}
