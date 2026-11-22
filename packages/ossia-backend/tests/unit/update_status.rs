use super::*;
use crate::{UpdateChannel, UpdateSnapshot, UpdateSource, UpdateState};

fn snapshot() -> UpdateSnapshot {
    UpdateSnapshot {
        state: UpdateState::Found,
        is_debug_build: false,
        current: "1.0.0".into(),
        latest: Some("1.1.0".into()),
        source: UpdateSource::Ossia,
        channel: UpdateChannel::Stable,
        error: None,
        done: None,
        total: None,
        notes: None,
        staged: false,
    }
}

#[test]
fn completed_checks_are_shared_and_expire() {
    let home = tempfile::tempdir().unwrap();
    assert!(read(home.path()).unwrap().is_none());
    write(home.path(), &snapshot()).unwrap();
    let mut saved = read(home.path()).unwrap().unwrap();
    assert_eq!(saved.state, UpdateState::Found);
    assert_eq!(saved.latest.as_deref(), Some("1.1.0"));
    assert!(!saved.stale);
    saved.checked_at = now().unwrap() - 24 * 60 * 60 - 1;
    std::fs::write(path(home.path()), serde_json::to_vec(&saved).unwrap()).unwrap();
    assert!(read(home.path()).unwrap().unwrap().stale);
    saved.channel = UpdateChannel::Beta;
    saved.checked_at = now().unwrap() - 6 * 60 * 60 - 1;
    std::fs::write(path(home.path()), serde_json::to_vec(&saved).unwrap()).unwrap();
    assert!(read(home.path()).unwrap().unwrap().stale);
    saved.checked_at = now().unwrap() + 60;
    std::fs::write(path(home.path()), serde_json::to_vec(&saved).unwrap()).unwrap();
    assert!(read(home.path()).unwrap().unwrap().stale);
    clear(home.path()).unwrap();
    assert!(read(home.path()).unwrap().is_none());
}

#[test]
fn failed_check_does_not_report_a_retained_download_as_a_new_result() {
    let home = tempfile::tempdir().unwrap();
    let mut result = snapshot();
    result.state = UpdateState::Ready;
    result.error = Some("private detail".into());
    write(home.path(), &result).unwrap();
    assert_eq!(
        read(home.path()).unwrap().unwrap().state,
        UpdateState::Error
    );
    assert!(!std::fs::read_to_string(path(home.path()))
        .unwrap()
        .contains("private detail"));
}
