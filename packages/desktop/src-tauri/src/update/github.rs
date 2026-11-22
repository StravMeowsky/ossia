//! GitHub Releases feed used by the desktop updater.

use super::{feed, fetch, Error, Result};
use ossia_types::UpdateChannel;
use serde::Deserialize;
use std::{io::Read, time::Duration};

pub const REPOSITORY: &str = "micooz/ossia";
pub const API_BASE: &str = "https://api.github.com";
const PAGE_SIZE: usize = 100;
const MAX_PAGES: usize = 20;
const FEED_LIMIT: u64 = 8 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackageKind {
    MacApp,
    WindowsSetup,
    Deb,
    Rpm,
}

impl PackageKind {
    pub fn suffix(self) -> &'static str {
        match self {
            Self::MacApp => ".zip",
            Self::WindowsSetup => ".exe",
            Self::Deb => ".deb",
            Self::Rpm => ".rpm",
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
struct GithubRelease {
    pub tag_name: String,
    #[serde(default)]
    pub draft: bool,
    #[serde(default)]
    pub prerelease: bool,
    #[serde(default)]
    pub body: String,
    #[serde(default)]
    pub assets: Vec<feed::Asset>,
}

/// Read releases newest-first, stopping once the running version is covered.
pub fn releases(
    base: &str,
    repository: &str,
    token: Option<&str>,
    current: Option<&str>,
    channel: UpdateChannel,
) -> Result<Vec<feed::Release>> {
    let current_version = current.map(parse_version).transpose()?;
    let mut releases = Vec::new();
    for page in 1..=MAX_PAGES {
        let page_releases = releases_page(base, repository, token, page)?;
        let page_len = page_releases.len();
        for release in page_releases {
            if release.draft {
                continue;
            }
            let version = parse_version(&release.tag_name)?;
            if channel == UpdateChannel::Stable && (release.prerelease || !version.pre.is_empty()) {
                continue;
            }
            releases.push(release);
        }
        releases.sort_by(|left, right| {
            parse_version(&right.tag_name)
                .unwrap()
                .cmp_precedence(&parse_version(&left.tag_name).unwrap())
        });
        if page_len < PAGE_SIZE
            || current_version.as_ref().is_some_and(|current| {
                releases.iter().any(|release| {
                    parse_version(&release.tag_name)
                        .unwrap()
                        .cmp_precedence(current)
                        != std::cmp::Ordering::Greater
                })
            })
        {
            return release_models(releases);
        }
    }
    Err(Error::other(format!(
        "update feed: more than {MAX_PAGES} release pages"
    )))
}

fn release_models(releases: Vec<GithubRelease>) -> Result<Vec<feed::Release>> {
    releases
        .into_iter()
        .map(|release| {
            let version = parse_version(&release.tag_name)?;
            Ok(feed::Release {
                version: version.to_string(),
                prerelease: release.prerelease || !version.pre.is_empty(),
                body: release.body,
                assets: release.assets,
            })
        })
        .collect()
}

pub fn parse_version(tag: &str) -> Result<semver::Version> {
    let text = tag.trim();
    semver::Version::parse(text.strip_prefix('v').unwrap_or(text))
        .map_err(|error| Error::other(format!("update feed: invalid release tag {tag:?}: {error}")))
}

fn releases_page(
    base: &str,
    repository: &str,
    token: Option<&str>,
    page: usize,
) -> Result<Vec<GithubRelease>> {
    let url = format!("{base}/repos/{repository}/releases?per_page={PAGE_SIZE}&page={page}");
    let client = fetch::client()?;
    let mut response = fetch::api_get(&client, &url, token)?
        .timeout(Duration::from_secs(30))
        .send()
        .map_err(|error| Error::other(format!("update feed: {error}")))?;
    let status = response.status().as_u16();
    let headers = response.headers().clone();
    let body = read_body(&mut response)?;
    if !(200..300).contains(&status) {
        return Err(Error::other(format!(
            "update feed: {}",
            describe_failure(status, &headers, &body)
        )));
    }
    serde_json::from_str::<Vec<GithubRelease>>(&body)
        .map_err(|error| Error::other(format!("update feed: {error}")))
}

fn read_body(response: &mut reqwest::blocking::Response) -> Result<String> {
    if response
        .content_length()
        .is_some_and(|length| length > FEED_LIMIT)
    {
        return Err(Error::other(format!(
            "update feed: body exceeds {FEED_LIMIT} bytes"
        )));
    }
    let mut bytes = Vec::new();
    response
        .take(FEED_LIMIT + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| Error::other(format!("update feed: {error}")))?;
    if bytes.len() as u64 > FEED_LIMIT {
        return Err(Error::other(format!(
            "update feed: body exceeds {FEED_LIMIT} bytes"
        )));
    }
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

pub fn describe_failure(status: u16, headers: &reqwest::header::HeaderMap, body: &str) -> String {
    let message = serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|value| {
            value
                .get("message")
                .and_then(|message| message.as_str())
                .map(str::to_string)
        });
    let remaining = headers
        .get("x-ratelimit-remaining")
        .and_then(|value| value.to_str().ok());
    let reset = headers
        .get("x-ratelimit-reset")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok());
    if matches!(status, 403 | 429) && remaining == Some("0") {
        let wait = reset.map_or_else(
            || "later".to_string(),
            |reset| {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|since| since.as_secs())
                    .unwrap_or(0);
                let minutes = reset.saturating_sub(now).div_ceil(60);
                format!("in about {minutes} minute(s)")
            },
        );
        return format!("GitHub rate limit reached; try again {wait}");
    }
    match (status, message) {
        (401 | 403 | 404, _) => format!(
            "GitHub release access failed (HTTP {status}) for {REPOSITORY}; check the repository visibility and any configured token"
        ),
        (_, Some(message)) => format!("HTTP {status}: {message}"),
        (_, None) => format!("HTTP {status}"),
    }
}
