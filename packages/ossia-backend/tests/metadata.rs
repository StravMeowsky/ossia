mod common;

use common::Fixture;
use ossia_backend::*;
use serde_json::json;
use std::fs;

#[tokio::test]
async fn update_metadata_changes_only_the_library_title() {
    let f = Fixture::new(false).await;
    let edit = f.begin();
    let changed = f.save(b"\\score { { g'1 } }");
    let before = f.runtime.library.get_document(f.doc.document_id).unwrap();
    assert_eq!(before.source_hash, changed);
    assert_eq!(before.edit_state, DocumentEditState::Active);
    let history = f.runtime.library.history(f.doc.document_id).unwrap();
    let cursor = f.runtime.agent_status().unwrap().seq;

    let result = ipc::dispatch(
        &f.runtime,
        "update_metadata",
        json!({"documentId":f.doc.document_id,"title":"  Renamed  "}),
    )
    .unwrap();
    let summary: DocumentSummary = serde_json::from_value(result).unwrap();
    assert_eq!(summary.title, "Renamed");
    assert_eq!(
        summary.url.as_deref(),
        Some(
            format!(
                "{}/scores?documentId={}",
                f.runtime.info.web_base_url, f.doc.document_id
            )
            .as_str()
        )
    );
    assert_eq!(summary.source_hash, changed);
    assert_eq!(summary.head_version_id, f.doc.head_version_id);
    assert_eq!(summary.edit_state, DocumentEditState::Active);
    assert_eq!(
        fs::read(&f.doc.source_path).unwrap(),
        b"\\score { { g'1 } }"
    );
    assert_eq!(
        f.runtime.library.history(f.doc.document_id).unwrap(),
        history
    );
    assert_eq!(
        f.runtime.library.get_edit(edit.edit_id).unwrap().status,
        EditStatus::Active
    );

    let events = f
        .runtime
        .events(Some(f.doc.document_id), &f.runtime.info.instance_id, cursor)
        .unwrap();
    assert_eq!(
        events
            .iter()
            .map(|event| event.r#type.as_str())
            .collect::<Vec<_>>(),
        vec!["document-changed"]
    );

    let listed: DocumentPage =
        serde_json::from_value(ipc::dispatch(&f.runtime, "list", json!({"limit":10})).unwrap())
            .unwrap();
    assert_eq!(listed.items[0].title, "Renamed");
    let context = ipc::dispatch(
        &f.runtime,
        "context",
        json!({"documentId":f.doc.document_id}),
    )
    .unwrap();
    assert_eq!(context["document"]["title"], json!("Renamed"));
    assert_eq!(
        context["document"]["sourceHash"],
        json!(changed.to_string())
    );

    let error = ipc::dispatch(
        &f.runtime,
        "update_metadata",
        json!({"documentId":f.doc.document_id,"title":"\n"}),
    )
    .unwrap_err();
    assert_eq!(error.code, ErrorCode::InvalidInput);
    let error = ipc::dispatch(
        &f.runtime,
        "update_metadata",
        json!({"documentId":DocumentId::new(),"title":"missing"}),
    )
    .unwrap_err();
    assert_eq!(error.code, ErrorCode::NotFound);

    f.finish().await;
}
