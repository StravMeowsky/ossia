//! Timing policy for automatic update checks.

use ossia_types::{UpdateChannel, UpdateState};
use std::time::Duration;

pub(crate) const STARTUP_DELAY: Duration = Duration::from_secs(5);

const STABLE_INTERVAL: Duration = Duration::from_secs(24 * 60 * 60);
const BETA_INTERVAL: Duration = Duration::from_secs(6 * 60 * 60);
const RETRY_BASE: Duration = Duration::from_secs(60 * 60);
const RETRY_MAX: Duration = Duration::from_secs(24 * 60 * 60);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NextCheck {
    After(Duration),
    Stop,
}

pub(crate) fn next_check(state: UpdateState, channel: UpdateChannel, failures: u32) -> NextCheck {
    match state {
        UpdateState::Source => NextCheck::Stop,
        UpdateState::Error => NextCheck::After(retry_delay(failures)),
        state if state.in_flight() => NextCheck::After(RETRY_BASE),
        _ => NextCheck::After(interval(channel)),
    }
}

pub(crate) fn retry_delay(failures: u32) -> Duration {
    let doublings = failures.saturating_sub(1).min(5);
    let seconds = RETRY_BASE.as_secs().saturating_mul(1 << doublings);
    Duration::from_secs(seconds.min(RETRY_MAX.as_secs()))
}

fn interval(channel: UpdateChannel) -> Duration {
    match channel {
        UpdateChannel::Stable => STABLE_INTERVAL,
        UpdateChannel::Beta => BETA_INTERVAL,
    }
}

#[cfg(test)]
#[path = "../../tests/unit/update_schedule.rs"]
mod tests;
