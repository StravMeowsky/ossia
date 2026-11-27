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

#[tokio::test(flavor = "multi_thread")]
async fn management_create_score_imports_a_blank_score_with_catalog_metadata() {
    let f = Fixture::new(true).await;
    // Like a real blank score, the render has no MIDI notes, so the import asks for confirmation.
    f.engine
        .midi
        .store(false, std::sync::atomic::Ordering::SeqCst);
    // The call waits for the import, so run it off the scheduler's thread as IPC does.
    let runtime = f.runtime.clone();
    let created = tokio::task::spawn_blocking(move || {
        ipc::dispatch(
            &runtime,
            "management-create-score",
            json!({
                "title": "Morning Song",
                "composer": "A.Composer",
                "instrument": "violin",
                "fifths": -1,
                "mode": "minor",
                "timeNumerator": 3,
                "timeDenominator": 4,
                "tempoText": "Andante",
                "measures": 4
            }),
        )
    })
    .await
    .unwrap()
    .unwrap();
    let created: ManagementCreatedScore = serde_json::from_value(created).unwrap();
    assert_ne!(created.document_id, f.doc.document_id);

    let page: ManagementScorePage = serde_json::from_value(
        ipc::dispatch(
            &f.runtime,
            "management-scores",
            json!({"limit": 10, "offset": 0}),
        )
        .unwrap(),
    )
    .unwrap();
    let item = page
        .items
        .iter()
        .find(|item| item.document_id == created.document_id)
        .unwrap();
    assert_eq!(item.title, "Morning Song");
    assert_eq!(item.composer.as_deref(), Some("A.Composer"));
    assert_eq!(
        item.instruments.as_deref(),
        Some(&["Violin".to_string()][..])
    );

    let document = f.runtime.library.get_document(created.document_id).unwrap();
    let source = std::fs::read_to_string(f.root.path().join(&document.source_path)).unwrap();
    assert!(source.contains("violin = \\relative c'' {"));
    assert!(source.contains("\\key d \\minor \\time 3/4"));
    assert!(source.contains("\\tempo \"Andante\"\n"));
    assert!(source.contains("R2.*4"));
}

#[tokio::test]
async fn management_create_score_refuses_an_invalid_request_without_creating_a_score() {
    let f = Fixture::new(true).await;
    let error = ipc::dispatch(
        &f.runtime,
        "management-create-score",
        json!({
            "title": "",
            "instrument": "piano",
            "fifths": 0,
            "mode": "major",
            "timeNumerator": 4,
            "timeDenominator": 4,
            "measures": 4
        }),
    )
    .unwrap_err();
    assert!(error.message.contains("title"));
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
}
