mod common;

use common::Fixture;
use ossia_backend::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn convert_render_preflight_keeps_the_candidate_source() {
    let f = Fixture::new(false).await;
    let args = ConvertArgs {
        path: "input.ly".into(),
        format: Some(InputFormat::Ly),
        verify_render: true,
        include_source: true,
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
            },
            b"BROKEN".to_vec(),
        )
        .unwrap();
    assert_eq!(report.status, ImportStatus::Clean);
    assert!(report.source.is_none());
    assert_eq!(report.source_hash, Some(ContentHash::of(b"BROKEN")));
    assert_eq!(report.omr_source.as_deref(), Some("/tmp/omr.musicxml"));
}
