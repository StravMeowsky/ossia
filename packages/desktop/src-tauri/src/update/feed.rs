//! Source-neutral release feed model.

use super::{github, ossia, target::Target, Error, Result};
use ossia_types::{UpdateChannel, UpdateSource};
use serde::Deserialize;
use std::sync::Arc;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Release {
    pub version: String,
    #[serde(default)]
    pub prerelease: bool,
    #[serde(default)]
    pub body: String,
    #[serde(default)]
    pub assets: Vec<Asset>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Asset {
    pub name: String,
    pub url: String,
    pub size: u64,
    #[serde(default)]
    pub digest: Option<String>,
}

impl Release {
    pub fn sha256sums(&self) -> Option<&Asset> {
        self.assets.iter().find(|asset| asset.name == "SHA256SUMS")
    }

    pub fn asset_for(&self, target: Target) -> Option<&Asset> {
        let wanted = target.asset_name();
        self.assets.iter().find(|asset| asset.name == wanted)
    }
}

#[derive(Clone)]
pub struct Endpoints {
    pub github_base: Arc<str>,
    pub github_repository: Arc<str>,
    pub ossia_base: Arc<str>,
}

pub fn releases(
    endpoints: &Endpoints,
    source: UpdateSource,
    token: Option<&str>,
    current: Option<&str>,
    channel: UpdateChannel,
    target: Option<Target>,
) -> Result<Vec<Release>> {
    match source {
        UpdateSource::Github => github::releases(
            &endpoints.github_base,
            &endpoints.github_repository,
            token,
            current,
            channel,
        ),
        UpdateSource::Ossia => {
            let target =
                target.ok_or_else(|| Error::other("this install cannot update itself in place"))?;
            ossia::releases(&endpoints.ossia_base, target, current, channel)
        }
    }
}
