//! Checksum handling for release assets.

use super::{feed::Asset, Error, Result};
use std::collections::HashMap;

pub fn expected(asset: &Asset, sums: Option<&str>) -> Result<String> {
    if let Some(digest) = asset.digest.as_deref() {
        return digest_hex(digest)
            .ok_or_else(|| Error::other(format!("{}: malformed digest {digest}", asset.name)));
    }
    let sums = sums.ok_or_else(|| {
        Error::other(format!(
            "{}: the release publishes no digest and no SHA256SUMS",
            asset.name
        ))
    })?;
    parse_sums(sums)
        .remove(&asset.name)
        .ok_or_else(|| Error::other(format!("SHA256SUMS has no entry for {}", asset.name)))
}

fn digest_hex(digest: &str) -> Option<String> {
    let hex = digest.strip_prefix("sha256:")?;
    let valid = hex.len() == 64 && hex.bytes().all(|byte| byte.is_ascii_hexdigit());
    valid.then(|| hex.to_ascii_lowercase())
}

pub fn parse_sums(text: &str) -> HashMap<String, String> {
    text.lines()
        .filter_map(|line| {
            let mut parts = line.split_whitespace();
            let hex = parts.next()?;
            let name = parts.next()?;
            Some((
                name.trim_start_matches('*').to_string(),
                hex.to_ascii_lowercase(),
            ))
        })
        .collect()
}
