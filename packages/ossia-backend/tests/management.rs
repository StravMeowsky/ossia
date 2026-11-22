mod common;

use common::{until, Fixture};
use ossia_backend::*;
use serde_json::json;

#[tokio::test]
async fn management_ipc_lists_scores_previews_and_archive_records() {
    let f = Fixture::new(false).await;
    let page: ManagementScorePage = serde_json::from_value(
        ipc::dispatch(
            &f.runtime,
            "management-scores",
            json!({"limit": 10, "offset": 0}),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(page.total, 1);
    assert_eq!(page.items[0].document_id, f.doc.document_id);
    assert_eq!(page.items[0].title, "synthetic");
    assert!(page.items[0].archived_at.is_none());

    let preview: ManagementScorePreview = serde_json::from_value(
        ipc::dispatch(
            &f.runtime,
            "management-score-preview",
            json!({"documentId": f.doc.document_id, "retry": false}),
        )
        .unwrap(),
    )
    .unwrap();
    match preview {
        ManagementScorePreview::Ready { data_url } => {
            assert!(data_url.starts_with("data:image/svg+xml;base64,"));
        }
        other => panic!("expected ready preview, got {other:?}"),
    }

    let url: String = serde_json::from_value(
        ipc::dispatch(
            &f.runtime,
            "management-score-url",
            json!({"documentId": f.doc.document_id}),
        )
        .unwrap(),
    )
    .unwrap();
    assert!(url.ends_with(&format!("/scores?documentId={}", f.doc.document_id)));

    let archived: TrashedDocumentSummary = serde_json::from_value(
        ipc::dispatch(
            &f.runtime,
            "management-archive-score",
            json!({"documentId": f.doc.document_id}),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(archived.document_id, f.doc.document_id);
    let active: ManagementScorePage = serde_json::from_value(
        ipc::dispatch(
            &f.runtime,
            "management-scores",
            json!({"limit": 10, "offset": 0}),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(active.total, 0);
    let archived_page: ManagementScorePage = serde_json::from_value(
        ipc::dispatch(
            &f.runtime,
            "management-archived-scores",
            json!({"limit": 10, "offset": 0}),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(archived_page.total, 1);
    assert!(archived_page.items[0].archived_at.is_some());
    assert_eq!(
        ipc::dispatch(
            &f.runtime,
            "management-score-url",
            json!({"documentId": f.doc.document_id}),
        )
        .unwrap_err()
        .code,
        ErrorCode::NotFound
    );

    f.finish().await;
}

#[tokio::test]
async fn archiving_cancels_preview_work_before_moving_a_score() {
    let f = Fixture::new(true).await;
    f.engine
        .hold
        .store(true, std::sync::atomic::Ordering::Release);
    let job = f
        .runtime
        .check(
            CheckArgs {
                expect: None,
                reference: None,
                document_id: Some(f.doc.document_id),
                edit_id: None,
                expected_source_hash: f.doc.source_hash.clone(),
                wait_ms: None,
            },
            true,
        )
        .unwrap();
    until(|| {
        f.runtime
            .library
            .get_job(job.job_id)
            .is_ok_and(|job| job.state == JobState::Running)
    })
    .await;

    let archived = f
        .runtime
        .management_archive_score(DocumentIdArgs {
            document_id: f.doc.document_id,
        })
        .unwrap();
    assert_eq!(archived.document_id, f.doc.document_id);
    assert_ne!(
        f.runtime.library.get_job(job.job_id).unwrap().state,
        JobState::Running
    );
    f.finish().await;
}

#[tokio::test]
async fn archiving_does_not_cancel_a_real_check() {
    let f = Fixture::new(true).await;
    f.engine
        .hold
        .store(true, std::sync::atomic::Ordering::Release);
    let job = f
        .runtime
        .check(
            CheckArgs {
                expect: None,
                reference: None,
                document_id: Some(f.doc.document_id),
                edit_id: None,
                expected_source_hash: f.doc.source_hash.clone(),
                wait_ms: None,
            },
            false,
        )
        .unwrap();
    until(|| {
        f.runtime
            .library
            .get_job(job.job_id)
            .is_ok_and(|job| job.state == JobState::Running)
    })
    .await;

    let error = f
        .runtime
        .management_archive_score(DocumentIdArgs {
            document_id: f.doc.document_id,
        })
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::Conflict);
    assert_eq!(
        f.runtime.library.get_job(job.job_id).unwrap().state,
        JobState::Running
    );
    f.runtime.cancel(job.job_id).unwrap();
    f.engine
        .hold
        .store(false, std::sync::atomic::Ordering::Release);
    f.finish().await;
}
