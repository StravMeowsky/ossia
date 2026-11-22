//! Sharing desktop update checks with local Agent transports.

use crate::{storage, AgentUpdate, Result, UpdateChannel, UpdateSnapshot, UpdateState};
use std::{
    io::Write,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

fn path(home: &Path) -> std::path::PathBuf {
    home.join("update").join("status.json")
}

fn now() -> Result<u64> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(storage)?
        .as_secs())
}

/// Read the last completed desktop check; absence means no check is known.
pub fn read(home: &Path) -> Result<Option<AgentUpdate>> {
    let bytes = match std::fs::read(path(home)) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(storage(error)),
    };
    let mut result: AgentUpdate = serde_json::from_slice(&bytes).map_err(storage)?;
    let max_age = match result.channel {
        UpdateChannel::Stable => 24 * 60 * 60,
        UpdateChannel::Beta => 6 * 60 * 60,
    };
    result.stale = now()?
        .checked_sub(result.checked_at)
        .is_none_or(|age| age > max_age);
    Ok(Some(result))
}

/// Atomically publish a completed check without exposing credentials or release notes.
pub fn write(home: &Path, snapshot: &UpdateSnapshot) -> Result<()> {
    let status = AgentUpdate {
        state: if snapshot.error.is_some() {
            UpdateState::Error
        } else {
            snapshot.state
        },
        current: snapshot.current.clone(),
        latest: snapshot.latest.clone(),
        source: snapshot.source,
        channel: snapshot.channel,
        checked_at: now()?,
        stale: false,
    };
    let path = path(home);
    let parent = path.parent().expect("update status has a parent");
    std::fs::create_dir_all(parent).map_err(storage)?;
    let mut file = tempfile::NamedTempFile::new_in(parent).map_err(storage)?;
    serde_json::to_writer(&mut file, &status).map_err(storage)?;
    file.write_all(b"\n").map_err(storage)?;
    file.persist(path).map_err(storage)?;
    Ok(())
}

/// Invalidate a result when its updater configuration is no longer applicable.
pub fn clear(home: &Path) -> Result<()> {
    match std::fs::remove_file(path(home)) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(storage(error)),
    }
}

#[cfg(test)]
#[path = "../tests/unit/update_status.rs"]
mod tests;
