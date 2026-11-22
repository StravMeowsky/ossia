//! Retained bilingual release notes for the update dialog.

use super::{feed::Release, version, Error, Result};
use ossia_types::{ReleaseNote, UpdateChannel, UpdateNotes, UpdateSource};
use serde::{Deserialize, Serialize};
use std::{
    io::Write,
    path::{Path, PathBuf},
};

pub const INTERNAL_ONLY_MARKER: &str = "<!-- ossia:internal-only -->";
const SCHEMA_VERSION: u32 = 2;
const MAX_CACHE_BYTES: u64 = 8 * 1024 * 1024;
const MAX_RETAINED: usize = 8;
const MAX_VISIBLE: usize = 5;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotesCache {
    pub schema_version: u32,
    pub source: Option<UpdateSource>,
    pub channel: Option<UpdateChannel>,
    pub releases: Vec<ReleaseNote>,
}

impl Default for NotesCache {
    fn default() -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            source: None,
            channel: None,
            releases: Vec::new(),
        }
    }
}

pub fn load(path: &Path) -> Result<NotesCache> {
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(NotesCache::default())
        }
        Err(error) => return Err(error.into()),
    };
    if bytes.len() as u64 > MAX_CACHE_BYTES {
        return Err(Error::other("update notes cache is too large"));
    }
    let cache: NotesCache = serde_json::from_slice(&bytes)
        .map_err(|error| Error::other(format!("cannot parse update notes cache: {error}")))?;
    if cache.schema_version != SCHEMA_VERSION {
        return Ok(NotesCache::default());
    }
    Ok(cache)
}

pub fn save(path: &Path, cache: &NotesCache) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut temporary = tempfile_named(path)?;
    serde_json::to_writer_pretty(&mut temporary, cache)?;
    temporary.write_all(b"\n")?;
    temporary.as_file().sync_all()?;
    temporary
        .persist(path)
        .map_err(|error| Error::other(error.error.to_string()))?;
    Ok(())
}

fn tempfile_named(path: &Path) -> Result<tempfile::NamedTempFile> {
    let parent = path
        .parent()
        .ok_or_else(|| Error::other("update notes cache has no parent"))?;
    tempfile::NamedTempFile::new_in(parent).map_err(|error| Error::other(error.to_string()))
}

pub fn remember(
    path: &Path,
    source: UpdateSource,
    channel: UpdateChannel,
    releases: &[Release],
) -> Result<NotesCache> {
    let mut cache = NotesCache {
        schema_version: SCHEMA_VERSION,
        source: Some(source),
        channel: Some(channel),
        releases: Vec::new(),
    };
    for release in releases.iter().take(MAX_RETAINED) {
        if release.body.trim().is_empty() {
            continue;
        }
        cache.releases.push(ReleaseNote {
            version: version::normalised(&release.version),
            body: release.body.clone(),
        });
    }
    if let Err(error) = save(path, &cache) {
        eprintln!("ossia: cannot save update notes cache: {error}");
    }
    Ok(cache)
}

pub fn select(
    cache: &NotesCache,
    current: Option<&str>,
    target: Option<&str>,
) -> Option<UpdateNotes> {
    if cache.releases.is_empty() {
        return None;
    }
    let visible = cache
        .releases
        .iter()
        .filter(|release| !internal_only(&release.body))
        .collect::<Vec<_>>();
    let target = target
        .map(version::normalised)
        .or_else(|| visible.first().map(|release| release.version.clone()))
        .or_else(|| {
            cache
                .releases
                .first()
                .map(|release| release.version.clone())
        })?;

    let newer = cache
        .releases
        .iter()
        .take_while(|release| match current {
            Some(current) => version::is_newer(&release.version, current),
            None => true,
        })
        .collect::<Vec<_>>();
    let mut selected = newer
        .iter()
        .filter(|release| !internal_only(&release.body))
        .take(MAX_VISIBLE)
        .map(|release| (*release).clone())
        .collect::<Vec<_>>();
    if selected.is_empty() && current.is_some_and(|current| version::normalised(current) == target)
    {
        if let Some(release) = cache
            .releases
            .iter()
            .find(|release| release.version == target && !internal_only(&release.body))
        {
            selected.push(release.clone());
        }
    }
    let maintenance_only =
        selected.is_empty() && newer.iter().any(|release| internal_only(&release.body));
    let from = current
        .map(version::normalised)
        .or_else(|| selected.last().map(|release| release.version.clone()))
        .unwrap_or_else(|| target.clone());
    Some(UpdateNotes {
        from,
        to: target,
        maintenance_only,
        releases: selected,
    })
}

pub fn internal_only(body: &str) -> bool {
    body.trim_start().starts_with(INTERNAL_ONLY_MARKER)
}

pub fn notes_path(home: &Path) -> PathBuf {
    home.join("update/releases.json")
}
