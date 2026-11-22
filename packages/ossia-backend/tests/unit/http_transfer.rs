use super::*;
use serde_json::json;

#[test]
fn runtime_events_normalize_paths_without_rewriting_content() {
    let event = RuntimeEvent {
        instance_id: "test".into(),
        seq: 7,
        r#type: "job".into(),
        document_id: None,
        source_hash: None,
        payload: json!({
            "path": r"\\?\E:\Projects\ossia\exports\score.pdf",
            "content": r"literal \\?\E:\Projects\ossia\score.ly"
        }),
    };

    let data = event_data(&event);
    let value: Value = serde_json::from_str(&data).unwrap();

    assert_eq!(
        value["payload"]["path"],
        r"E:\Projects\ossia\exports\score.pdf"
    );
    assert_eq!(
        value["payload"]["content"],
        r"literal \\?\E:\Projects\ossia\score.ly"
    );
}
