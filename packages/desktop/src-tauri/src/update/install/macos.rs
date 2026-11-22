//! Replace the running macOS application bundle.

use super::super::{Error, Result};
use std::{
    path::{Path, PathBuf},
    process::Command,
};

pub fn bundle_of(exe: &Path) -> Option<PathBuf> {
    let exe = std::fs::canonicalize(exe).ok()?;
    let macos = exe.parent()?;
    let contents = macos.parent()?;
    let bundle = contents.parent()?;
    let inside = macos.file_name()? == "MacOS"
        && contents.file_name()? == "Contents"
        && bundle.extension()? == "app";
    inside.then(|| bundle.to_path_buf())
}

pub fn install(staged: &Path, bundle: &Path) -> Result<()> {
    let work = staging_dir(bundle)?;
    let _ = std::fs::remove_dir_all(&work);
    std::fs::create_dir_all(&work)?;
    let unpacked = work.join("unpacked");
    let output = Command::new("ditto")
        .args(["-x", "-k"])
        .arg(staged)
        .arg(&unpacked)
        .output()?;
    if !output.status.success() {
        return Err(Error::other(format!(
            "cannot unpack the update: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    let new_app = find_app(&unpacked)
        .ok_or_else(|| Error::other("the downloaded archive contains no .app"))?;
    let previous = work.join("previous.app");
    std::fs::rename(bundle, &previous)?;
    if let Err(error) = std::fs::rename(&new_app, bundle) {
        if std::fs::rename(&previous, bundle).is_err() {
            return Err(Error::other(format!(
                "cannot put the update in place ({error}), and the previous app could not be moved back: it is at {}",
                previous.display()
            )));
        }
        return Err(error.into());
    }
    let _ = std::fs::remove_dir_all(&work);
    Ok(())
}

pub fn prepare(exe: &Path) -> Result<super::Installation> {
    let executable = std::fs::canonicalize(exe)?;
    let bundle = bundle_of(&executable)
        .ok_or_else(|| Error::other("this copy of Ossia is not inside an .app bundle"))?;
    Ok(super::Installation::Mac { bundle, executable })
}

fn staging_dir(bundle: &Path) -> Result<PathBuf> {
    let parent = bundle
        .parent()
        .ok_or_else(|| Error::other("the app has no parent directory"))?;
    ensure_writable(parent)?;
    Ok(parent.join(".ossia-update"))
}

fn ensure_writable(dir: &Path) -> Result<()> {
    let probe = dir.join(".ossia-update-probe");
    std::fs::write(&probe, b"").map_err(|error| {
        Error::other(format!(
            "{} is not writable ({error}); move Ossia to a folder you can write to",
            dir.display()
        ))
    })?;
    let _ = std::fs::remove_file(&probe);
    Ok(())
}

fn find_app(dir: &Path) -> Option<PathBuf> {
    std::fs::read_dir(dir)
        .ok()?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .find(|path| path.extension().is_some_and(|extension| extension == "app"))
}
