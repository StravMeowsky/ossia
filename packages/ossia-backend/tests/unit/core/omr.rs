use super::{config::OmrConfig, retain_musicxml};
use crate::{OmrService, UiPreferences, OSSIA_OMR_URL};

#[test]
fn retains_omr_musicxml_by_input_hash() {
    let home = tempfile::tempdir().unwrap();
    let path = retain_musicxml(home.path(), b"pdf", b"<score-partwise/>").unwrap();
    assert!(path.starts_with(home.path().join("imports/omr")));
    assert_eq!(std::fs::read(path).unwrap(), b"<score-partwise/>");
}

#[test]
fn ossia_service_uses_the_fixed_endpoint_and_ossia_key() {
    let preferences = UiPreferences {
        omr_service: OmrService::Ossia,
        omr_token: Some("ossia-key".into()),
        ..UiPreferences::default()
    };
    let config = OmrConfig::from_preferences(&preferences).unwrap();
    assert_eq!(config.url, OSSIA_OMR_URL);
    assert_eq!(config.token, "ossia-key");
}

#[test]
fn custom_service_uses_its_own_url_and_key() {
    let preferences = UiPreferences {
        omr_service: OmrService::Custom,
        omr_url: Some("https://omr.example.com/omr".into()),
        omr_custom_token: Some("custom-key".into()),
        ..UiPreferences::default()
    };
    let config = OmrConfig::from_preferences(&preferences).unwrap();
    assert_eq!(config.url, "https://omr.example.com/omr");
    assert_eq!(config.token, "custom-key");
}

#[test]
fn custom_service_requires_a_url_and_key() {
    let preferences = UiPreferences {
        omr_service: OmrService::Custom,
        ..UiPreferences::default()
    };
    assert!(OmrConfig::from_preferences(&preferences).is_err());
}
