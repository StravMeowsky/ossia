use super::{next_check, retry_delay, NextCheck};
use ossia_types::{UpdateChannel, UpdateState};
use std::time::Duration;

const HOUR: Duration = Duration::from_secs(60 * 60);

#[test]
fn released_channels_use_their_check_intervals() {
    assert_eq!(
        next_check(UpdateState::Latest, UpdateChannel::Stable, 0),
        NextCheck::After(Duration::from_secs(24 * 60 * 60))
    );
    assert_eq!(
        next_check(UpdateState::Found, UpdateChannel::Beta, 0),
        NextCheck::After(Duration::from_secs(6 * 60 * 60))
    );
}

#[test]
fn failures_back_off_to_one_day() {
    let delays: Vec<_> = (1..=7).map(retry_delay).collect();
    assert_eq!(
        delays,
        vec![
            HOUR,
            2 * HOUR,
            4 * HOUR,
            8 * HOUR,
            16 * HOUR,
            24 * HOUR,
            24 * HOUR
        ]
    );
}

#[test]
fn zero_failures_use_the_first_retry_delay() {
    assert_eq!(retry_delay(0), HOUR);
}

#[test]
fn in_flight_checks_retry_without_backoff() {
    assert_eq!(
        next_check(UpdateState::Downloading, UpdateChannel::Stable, 3),
        NextCheck::After(HOUR)
    );
}

#[test]
fn source_builds_stop_the_scheduler() {
    assert_eq!(
        next_check(UpdateState::Source, UpdateChannel::Stable, 0),
        NextCheck::Stop
    );
}
