use super::*;
use crate::test_support::Fixture;

#[test]
fn settings_persist_and_reject_stale_writes() {
    let fixture = Fixture::new();
    let id = fixture.doc.document_id;
    let mut settings = fixture.library.score_settings(id).unwrap();
    settings
        .instruments
        .insert("part-a".into(), SampleInstrument::Guitar);
    let saved = fixture
        .library
        .save_score_settings(id, settings.clone())
        .unwrap();
    assert_eq!(saved.revision, 1);
    assert_eq!(fixture.library.score_settings(id).unwrap(), saved);
    assert_eq!(
        fixture
            .library
            .save_score_settings(id, settings)
            .unwrap_err()
            .code,
        ErrorCode::Conflict
    );
}
