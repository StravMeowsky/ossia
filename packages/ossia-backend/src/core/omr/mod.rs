mod client;
mod config;

use crate::Result;
use ossia_engine::RunContext;
use ossia_types::{ContentHash, Diagnostic, OmrSourceFormat};
use std::{
    fs,
    path::{Path, PathBuf},
};

pub(crate) fn transcribe(
    home: &Path,
    format: OmrSourceFormat,
    input: &[u8],
    context: &RunContext,
) -> Result<(Vec<u8>, Vec<Diagnostic>)> {
    let preferences = crate::preferences::load(home)?;
    let config = config::OmrConfig::from_preferences(&preferences)?;
    client::OmrClient::new(config)?.transcribe(format, input, context)
}

pub(crate) fn retain_musicxml(
    home: &Path,
    input: &[u8],
    musicxml: &[u8],
) -> std::io::Result<PathBuf> {
    let path = home
        .join("imports")
        .join("omr")
        .join(format!("{}.musicxml", ContentHash::of(input)));
    fs::create_dir_all(path.parent().expect("OMR import directory"))?;
    fs::write(&path, musicxml)?;
    Ok(path)
}

#[cfg(test)]
#[path = "../../../tests/unit/core/omr.rs"]
mod tests;
