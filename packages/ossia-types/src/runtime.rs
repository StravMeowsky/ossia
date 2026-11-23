use crate::*;
use schemars::JsonSchema;
use ts_rs::TS;

mod output;
mod preferences;
mod update;

pub use output::*;
pub use preferences::*;
pub use update::*;

/// One of the bundled sample banks used for score playback.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "lowercase")]
pub enum SampleInstrument {
    #[default]
    Piano,
    Guitar,
    Violin,
    Ukulele,
}

impl SampleInstrument {
    /// The instrument's key in the bundled sample catalog.
    pub fn catalog_key(self) -> &'static str {
        match self {
            Self::Piano => "piano",
            Self::Guitar => "guitar",
            Self::Violin => "violin",
            Self::Ukulele => "ukulele",
        }
    }
}

/// Settings shared by all versions of one score.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScoreSettings {
    pub revision: u64,
    #[serde(default)]
    pub instruments: std::collections::BTreeMap<String, SampleInstrument>,
}

/// A freehand drawing attached to a document, independent of score versions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Drawing {
    pub revision: u64,
    pub strokes: Vec<DrawingStroke>,
}

/// A stroke in normalized page coordinates.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DrawingStroke {
    pub page: u32,
    pub width: f64,
    /// Ink color; defaults to black when omitted.
    #[serde(default)]
    pub color: DrawingColor,
    pub points: Vec<DrawingPoint>,
}

/// The fixed ink colors available in the score editor.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "lowercase")]
pub enum DrawingColor {
    #[default]
    Black,
    Blue,
    Green,
    Yellow,
    Red,
}

impl DrawingColor {
    /// Return the sRGB components of the preset.
    pub fn rgb(self) -> [u8; 3] {
        match self {
            Self::Black => [0, 0, 0],
            Self::Blue => [0, 122, 255],
            Self::Green => [52, 199, 89],
            Self::Yellow => [255, 204, 0],
            Self::Red => [255, 59, 48],
        }
    }
}

/// A point relative to the page width and height.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DrawingPoint {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeInfo {
    pub runtime_version: String,
    pub protocol_version: u32,
    pub schema_version: u32,
    pub instance_id: String,
    pub home: String,
    pub web_base_url: String,
    pub capabilities: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct Selection {
    pub document_id: DocumentId,
    pub source_hash: ContentHash,
    pub targets: Vec<SelectionTarget>,
    #[serde(default)]
    pub annotations: Vec<SelectionAnnotation>,
}

/// One user-authored change request attached to score elements.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct SelectionAnnotation {
    /// Client-generated identity used while editing or deleting the whole group.
    pub id: String,
    /// User-authored change request shown by the numbered pins.
    pub text: String,
    /// Primary score objects clicked for this annotation group.
    #[serde(default)]
    pub targets: Vec<SelectionTarget>,
    /// Persisted marquee rectangles; empty means the group uses per-target pins.
    #[serde(default)]
    pub boxes: Vec<AnnotationBox>,
}

/// One page fragment of a marquee selection.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct AnnotationBox {
    /// One-based rendered page containing this marquee fragment.
    pub page: u32,
    /// Marquee fragment in that page's SVG viewBox coordinates.
    pub bounds: GeometryBounds,
    /// Only one fragment per marquee gesture owns the group pin.
    #[serde(default = "default_annotation_pin")]
    pub pin: bool,
}

fn default_annotation_pin() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct RenderReference {
    pub render_key: ContentHash,
    pub manifest: RenderManifest,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct DocumentContext {
    pub instance_id: String,
    pub seq: u64,
    pub document: DocumentState,
    pub selection: Option<Selection>,
    pub regions: Vec<SelectionRegion>,
    pub annotation_regions: Vec<SelectionRegion>,
    pub render: Option<RenderReference>,
    pub edit_activity: Option<EditActivity>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeEvent {
    pub instance_id: String,
    pub seq: u64,
    pub r#type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub document_id: Option<DocumentId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub source_hash: Option<ContentHash>,
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ImportArgs {
    pub title: Option<String>,
    pub format: Option<InputFormat>,
    pub path: String,
    pub confirmation: Option<ContentHash>,
    pub author: Option<String>,
    pub expect: Option<MusicExpectation>,
    pub reference: Option<ReferenceExpectation>,
    pub wait_ms: Option<u64>,
}

/// Inspect one local input through the native converter without publishing it.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ConvertArgs {
    pub path: String,
    pub format: Option<InputFormat>,
    #[serde(default = "default_verify_render")]
    pub verify_render: bool,
    #[serde(default)]
    pub include_source: bool,
}

fn default_verify_render() -> bool {
    true
}

/// Identify one document for a context read.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ContextArgs {
    pub document_id: DocumentId,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "kebab-case")]
pub enum EditAction {
    Begin,
    Resume,
    End,
    Discard,
    ResolveRecoveryConflict,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EditArgs {
    pub action: EditAction,
    pub document_id: DocumentId,
    pub base_version_id: Option<VersionId>,
    pub expected_source_hash: ContentHash,
    pub edit_id: Option<EditId>,
    #[serde(default)]
    pub adopt_workspace: bool,
    #[serde(default = "default_author")]
    pub author: String,
}
/// Attribution used when an operation omits its author.
pub const DEFAULT_AUTHOR: &str = "Ossia";
fn default_author() -> String {
    DEFAULT_AUTHOR.into()
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CheckArgs {
    #[serde(default)]
    pub expect: Option<MusicExpectation>,
    #[serde(default)]
    pub reference: Option<ReferenceExpectation>,
    pub document_id: Option<DocumentId>,
    pub edit_id: Option<EditId>,
    pub expected_source_hash: ContentHash,
    pub wait_ms: Option<u64>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CommitArgs {
    pub edit_id: EditId,
    pub expected_source_hash: ContentHash,
    pub summary: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HistoryArgs {
    pub document_id: DocumentId,
    pub limit: Option<usize>,
    #[serde(default)]
    pub offset: usize,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RestoreArgs {
    pub version_id: VersionId,
    /// Current document head.
    pub base_version_id: VersionId,
    pub expected_source_hash: ContentHash,
    /// Allow discarding the current workspace and ending its edit when restoring.
    #[serde(default)]
    pub discard_current_edit: bool,
    pub author: Option<String>,
    pub summary: String,
    pub wait_ms: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "kebab-case")]
pub enum JobAction {
    Get,
    Cancel,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct JobArgs {
    #[serde(default)]
    pub detail: DetailLevel,
    pub job_id: JobId,
    pub action: JobAction,
    pub wait_ms: Option<u64>,
    pub section: Option<AnalysisSection>,
    #[serde(default)]
    pub offset: usize,
    pub limit: Option<usize>,
    pub measure_range: Option<MeasureRange>,
    pub voice: Option<String>,
    pub track: Option<usize>,
    pub channel: Option<u8>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LocateArgs {
    pub source_hash: ContentHash,
    pub page: u32,
    pub x: f64,
    pub y: f64,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct LocateOutcome {
    pub capability: LocationCapability,
    pub reason: Option<String>,
    pub region: Option<SelectionRegion>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EmptyArgs {}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct RpcRequest {
    pub method: String,
    pub params: serde_json::Value,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct RpcResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub error: Option<AppError>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct EditActivity {
    pub author: String,
    pub status: EditStatus,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct GeometryBounds {
    pub x0: f64,
    pub y0: f64,
    pub x1: f64,
    pub y1: f64,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct GeometryRegion {
    pub locator: Locator,
    pub system_index: usize,
    pub bounds: GeometryBounds,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct GeometryPage {
    pub page: u32,
    pub view_box: Option<GeometryBounds>,
    pub capability: LocationCapability,
    pub reason: Option<String>,
    pub regions: Vec<GeometryRegion>,
    pub elements: Vec<GeometryElement>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct RenderGeometry {
    pub render_key: ContentHash,
    pub source_hash: ContentHash,
    pub pages: Vec<GeometryPage>,
}

/// One scheduled MIDI note in wall-clock seconds.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackNote {
    pub pitch: u8,
    pub start_seconds: f64,
    pub duration_seconds: f64,
    pub part_id: String,
    pub instrument: SampleInstrument,
}

/// One independently assignable sounding part in a score render.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackPart {
    pub id: String,
    pub name: String,
    pub instrument: SampleInstrument,
}

/// Source noteheads sharing one sounding pitch at a playback step, or rests.
///
/// Staff and voice identify the navigation scope; fallback-only groups may not
/// resolve either value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackNoteGroup {
    pub element_ids: Vec<String>,
    pub pitches: Vec<u8>,
    pub staff_index: Option<usize>,
    pub voice: Option<String>,
    pub start_seconds: Option<f64>,
    pub duration_seconds: Option<f64>,
    pub part_id: Option<String>,
}

/// One measure's playhead rectangle within a rendered system.
///
/// A measure span starts at its first notehead, not at the preceding
/// barline, clef or signature space.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackSystemSpan {
    pub page: u32,
    pub system_index: usize,
    pub measure_start: u32,
    pub measure_end: u32,
    pub start_seconds: f64,
    pub end_seconds: f64,
    pub x0: f64,
    pub x1: f64,
    pub y0: f64,
    pub y1: f64,
}

/// Browser playback data derived from one immutable render.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct ScorePlayback {
    pub render_key: ContentHash,
    pub source_hash: ContentHash,
    pub duration_seconds: f64,
    pub parts: Vec<PlaybackPart>,
    pub notes: Vec<PlaybackNote>,
    pub groups: Vec<PlaybackNoteGroup>,
    pub systems: Vec<PlaybackSystemSpan>,
}

/// One rendered staff within a system, in SVG user units.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct LayoutStaff {
    pub top: f64,
    pub bot: f64,
    pub x0: f64,
    pub x1: f64,
}

/// One rendered system and its staves.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct LayoutSystem {
    pub top: f64,
    pub bot: f64,
    pub bars: usize,
    pub measure_start: Option<u32>,
    pub measure_end: Option<u32>,
    pub staves: Vec<LayoutStaff>,
}

/// Reliability of source-measure mapping for rendered pages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "kebab-case")]
pub enum MeasurementCapability {
    None,
    Partial,
    Complete,
}

/// Rendered page structure independent of measure regions.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct LayoutPage {
    pub page: u32,
    pub view_box: Option<GeometryBounds>,
    pub staff_space: f64,
    pub systems: Vec<LayoutSystem>,
    pub capability: LocationCapability,
    pub measure_mapping: MeasurementCapability,
    pub reason: Option<String>,
}

/// Page, system and staff structure from one immutable score snapshot.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct LayoutOutcome {
    pub job_id: JobId,
    pub document_id: DocumentId,
    pub source_hash: ContentHash,
    pub page_count: usize,
    pub system_count: usize,
    pub staff_count: usize,
    pub measure_mapping: MeasurementCapability,
    pub pages: Vec<LayoutPage>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LayoutArgs {
    pub job_id: JobId,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct EngineHealth {
    pub ok: bool,
    pub build_id: Option<String>,
    pub error: Option<AppError>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeHealth {
    pub ok: bool,
    pub info: Option<RuntimeInfo>,
    pub error: Option<AppError>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub struct DoctorReport {
    #[serde(default)]
    pub documentation: Option<DocumentationStatus>,
    pub home: String,
    pub engine: EngineHealth,
    pub runtime: RuntimeHealth,
}
