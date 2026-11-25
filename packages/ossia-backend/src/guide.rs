use crate::{path_text, storage, Result, GUIDE};
use serde::Serialize;
use std::{
    io::Write,
    path::{Path, PathBuf},
};

/// Select an embedded guide document; omission selects the index.
#[derive(Debug, Clone, Default, serde::Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GuideArgs {
    pub document: Option<String>,
}

const DOCUMENTS: &[(&str, &str)] = &[
    (
        "guide/editing.md",
        include_str!("../resources/guide/editing.md"),
    ),
    (
        "guide/import.md",
        include_str!("../resources/guide/import.md"),
    ),
    (
        "guide/evidence.md",
        include_str!("../resources/guide/evidence.md"),
    ),
    (
        "guide/library.md",
        include_str!("../resources/guide/library.md"),
    ),
    (
        "guide/output.md",
        include_str!("../resources/guide/output.md"),
    ),
    (
        "guide/operations.md",
        include_str!("../resources/guide/operations.md"),
    ),
];

/// Text and materialized location of an Agent guide document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GuideOutcome {
    pub path: String,
    pub bytes: usize,
    pub lines: usize,
    pub text: String,
}

pub(crate) fn path(home: &Path) -> PathBuf {
    home.join("docs").join("guide.md")
}

/// Materialize the embedded guide documents and return the index.
pub(crate) fn outcome(home: &Path) -> Result<GuideOutcome> {
    document(home, None)
}

/// Text of an embedded guide document, by the path `ossia_guide` accepts.
pub fn guide_document(name: &str) -> Option<&'static str> {
    match name {
        "guide.md" => Some(GUIDE),
        name => DOCUMENTS
            .iter()
            .find(|(path, _)| *path == name)
            .map(|(_, content)| *content),
    }
}

pub(crate) fn document(home: &Path, document: Option<&str>) -> Result<GuideOutcome> {
    let (name, content) = match document {
        None | Some("guide.md") => ("guide.md", GUIDE),
        Some(name) => DOCUMENTS
            .iter()
            .copied()
            .find(|(path, _)| *path == name)
            .ok_or_else(|| {
                crate::AppError::new(crate::ErrorCode::InvalidInput, "unknown guide document")
            })?,
    };
    let path = path(home);
    for (name, content) in DOCUMENTS {
        materialize(&home.join("docs").join(name), content.as_bytes())?;
    }
    let bytes = GUIDE.as_bytes();
    materialize(&path, bytes)?;
    let selected_path = home.join("docs").join(name);
    Ok(GuideOutcome {
        path: path_text(&selected_path),
        bytes: content.len(),
        lines: content.lines().count(),
        text: content.to_string(),
    })
}

fn materialize(path: &Path, bytes: &[u8]) -> Result<()> {
    let current = std::fs::read(path).ok();
    if current.as_deref() != Some(bytes) {
        write(path, bytes).map_err(|error| storage(format!("{}: {error}", path.display())))?;
    }
    Ok(())
}

fn write(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| storage("guide path has no parent"))?;
    std::fs::create_dir_all(parent).map_err(storage)?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent).map_err(storage)?;
    temporary.write_all(bytes).map_err(storage)?;
    temporary.as_file().sync_all().map_err(storage)?;
    #[cfg(windows)]
    if path.exists() {
        std::fs::remove_file(path).map_err(storage)?;
    }
    temporary
        .persist(path)
        .map(|_| ())
        .map_err(|error| storage(error.error))
}
