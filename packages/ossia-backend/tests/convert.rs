mod common;

use common::Fixture;
use ossia_backend::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn migration_uses_the_engine_and_keeps_the_original_unpublished() {
    let f = Fixture::new(false).await;
    let args = ConvertArgs {
        path: "old.ly".into(),
        format: Some(InputFormat::Ly),
        verify_render: false,
        include_source: true,
        migrate_lilypond: true,
    };
    let report = f.runtime.convert(args, b"old source".to_vec()).unwrap();
    assert_eq!(report.input_hash, ContentHash::of(b"old source"));
    assert!(report.source.unwrap().contains("2.26.0"));
    assert_eq!(f.runtime.library.document_count().unwrap(), 1);
    let args = ConvertArgs {
        path: "input.mid".into(),
        format: Some(InputFormat::Midi),
        verify_render: false,
        include_source: true,
        migrate_lilypond: true,
    };
    assert_eq!(
        f.runtime.convert(args, vec![]).unwrap_err().code,
        ErrorCode::InvalidRequest
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn convert_render_preflight_keeps_the_candidate_source() {
    let f = Fixture::new(false).await;
    let args = ConvertArgs {
        path: "input.ly".into(),
        format: Some(InputFormat::Ly),
        verify_render: true,
        include_source: true,
        migrate_lilypond: false,
    };
    let report = f.runtime.convert(args, b"BROKEN".to_vec()).unwrap();
    assert_eq!(report.status, ImportStatus::Refused);
    assert_eq!(report.source.as_deref(), Some("BROKEN"));
    assert!(report
        .blocking
        .iter()
        .any(|reason| reason.starts_with("render: ")));

    let report = f
        .runtime
        .convert(
            ConvertArgs {
                path: "input.ly".into(),
                format: Some(InputFormat::Ly),
                verify_render: false,
                include_source: true,
                migrate_lilypond: false,
            },
            b"BROKEN".to_vec(),
        )
        .unwrap();
    assert_eq!(report.status, ImportStatus::Clean);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn convert_omits_source_unless_requested() {
    let f = Fixture::new(false).await;
    *f.engine.omr_source.lock().unwrap() = Some("/tmp/omr.musicxml".into());
    let report = f
        .runtime
        .convert(
            ConvertArgs {
                path: "input.ly".into(),
                format: Some(InputFormat::Ly),
                verify_render: false,
                include_source: false,
                migrate_lilypond: false,
            },
            b"BROKEN".to_vec(),
        )
        .unwrap();
    assert_eq!(report.status, ImportStatus::Clean);
    assert!(report.source.is_none());
    assert_eq!(report.source_hash, Some(ContentHash::of(b"BROKEN")));
    assert_eq!(report.omr_source.as_deref(), Some("/tmp/omr.musicxml"));
}
