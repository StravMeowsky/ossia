//! Static Ossia update feed.

use super::{feed, fetch, github, target::Target, Error, Result};
use ossia_types::UpdateChannel;
use reqwest::Url;
use serde::Deserialize;
use std::{io::Read, time::Duration};

const SCHEMA_VERSION: u32 = 1;
const FEED_LIMIT: u64 = 8 * 1024 * 1024;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Manifest {
    schema_version: u32,
    target: String,
    #[serde(default)]
    releases: Vec<feed::Release>,
}

pub fn releases(
    base: &str,
    target: Target,
    _current: Option<&str>,
    channel: UpdateChannel,
) -> Result<Vec<feed::Release>> {
    let slug = target.feed_slug();
    let url = format!("{base}/{slug}/index.json");
    fetch::ensure_secure(&url)?;
    let client = fetch::client()?;
    let mut response = client
        .get(&url)
        .header(reqwest::header::ACCEPT, "application/json")
        .timeout(Duration::from_secs(30))
        .send()
        .map_err(|error| Error::other(format!("Ossia update feed: {error}")))?;
    let status = response.status().as_u16();
    let body = read_body(&mut response)?;
    if !(200..300).contains(&status) {
        return Err(Error::other(format!(
            "Ossia update feed: HTTP {status}: {}",
            body.trim()
        )));
    }
    let manifest: Manifest = serde_json::from_str(&body)
        .map_err(|error| Error::other(format!("Ossia update feed: {error}")))?;
    if manifest.schema_version != SCHEMA_VERSION {
        return Err(Error::other(format!(
            "Ossia update feed: unsupported schema version {}",
            manifest.schema_version
        )));
    }
    if manifest.target != slug {
        return Err(Error::other(format!(
            "Ossia update feed: expected target {slug}, got {}",
            manifest.target
        )));
    }

    let mut releases = Vec::new();
    for mut release in manifest.releases {
        let version = github::parse_version(&release.version)?;
        if channel == UpdateChannel::Stable && (release.prerelease || !version.pre.is_empty()) {
            continue;
        }
        release.version = version.to_string();
        for asset in &mut release.assets {
            let resolved = Url::parse(&url)
                .and_then(|base| base.join(&asset.url))
                .map_err(|error| Error::other(format!("Ossia update asset: {error}")))?;
            fetch::ensure_secure(resolved.as_str())?;
            asset.url = resolved.to_string();
        }
        releases.push(release);
    }
    releases.sort_by(|left, right| {
        github::parse_version(&right.version)
            .unwrap()
            .cmp_precedence(&github::parse_version(&left.version).unwrap())
    });
    Ok(releases)
}

fn read_body(response: &mut reqwest::blocking::Response) -> Result<String> {
    if response
        .content_length()
        .is_some_and(|length| length > FEED_LIMIT)
    {
        return Err(Error::other(format!(
            "Ossia update feed: body exceeds {FEED_LIMIT} bytes"
        )));
    }
    let mut bytes = Vec::new();
    response
        .take(FEED_LIMIT + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| Error::other(format!("Ossia update feed: {error}")))?;
    if bytes.len() as u64 > FEED_LIMIT {
        return Err(Error::other(format!(
            "Ossia update feed: body exceeds {FEED_LIMIT} bytes"
        )));
    }
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}
