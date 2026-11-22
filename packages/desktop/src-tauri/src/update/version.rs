//! Version and build-provenance rules for the updater.

use ossia_types::{UpdateChannel, UpdateChannelPreference};

fn parse(text: &str) -> Option<semver::Version> {
    let text = text.trim();
    semver::Version::parse(text.strip_prefix('v').unwrap_or(text)).ok()
}

pub fn is_newer(candidate: &str, current: &str) -> bool {
    match (parse(candidate), parse(current)) {
        (Some(candidate), Some(current)) => {
            candidate.cmp_precedence(&current) == std::cmp::Ordering::Greater
        }
        _ => false,
    }
}

pub fn is_released(version: &str) -> bool {
    let Some(parsed) = parse(version) else {
        return false;
    };
    let pre = parsed.pre.as_str();
    !pre.contains("dirty") && pre != "untagged" && !looks_like_describe(pre)
}

fn looks_like_describe(pre: &str) -> bool {
    let Some((commits, rest)) = pre.split_once('-') else {
        return false;
    };
    let Some(hash) = rest.strip_prefix('g') else {
        return false;
    };
    !commits.is_empty()
        && commits.bytes().all(|byte| byte.is_ascii_digit())
        && !hash.is_empty()
        && hash.bytes().all(|byte| byte.is_ascii_hexdigit())
}

pub fn build_stamp() -> &'static str {
    env!("OSSIA_BUILD_STAMP")
}

pub fn normalised(tag: &str) -> String {
    let tag = tag.trim();
    tag.strip_prefix('v').unwrap_or(tag).to_string()
}

pub fn effective_channel(preference: UpdateChannelPreference, build: &str) -> UpdateChannel {
    match preference {
        UpdateChannelPreference::Stable => UpdateChannel::Stable,
        UpdateChannelPreference::Beta => UpdateChannel::Beta,
        UpdateChannelPreference::Auto => {
            let beta =
                parse(build).is_some_and(|version| !version.pre.is_empty() && is_released(build));
            if beta {
                UpdateChannel::Beta
            } else {
                UpdateChannel::Stable
            }
        }
    }
}

pub fn is_prerelease(version: &str) -> bool {
    parse(version).is_some_and(|version| !version.pre.is_empty())
}
