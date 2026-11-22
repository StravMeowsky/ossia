use crate::{path_text, storage, Result, GUIDE};
use serde::Serialize;
use std::{
    io::Write,
    path::{Path, PathBuf},
};

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

/// Absolute location and size of the materialized Agent guide.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GuideOutcome {
    pub path: String,
    pub bytes: usize,
    pub lines: usize,
}

pub(crate) fn path(home: &Path) -> PathBuf {
    home.join("docs").join("guide.md")
}

/// Materialize the embedded guide documents and return the index path and size.
pub(crate) fn outcome(home: &Path) -> Result<GuideOutcome> {
    let path = path(home);
    for (name, content) in DOCUMENTS {
        materialize(&home.join("docs").join(name), content.as_bytes())?;
    }
    let bytes = GUIDE.as_bytes();
    materialize(&path, bytes)?;
    Ok(GuideOutcome {
        path: path_text(&path),
        bytes: bytes.len(),
        lines: GUIDE.lines().count(),
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
