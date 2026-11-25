use super::*;

#[test]
fn init_line_reports_the_session() {
    assert_eq!(
        parse_line(r#"{"type":"system","subtype":"init","session_id":"abc","tools":[]}"#),
        vec![AssistantEvent::Session {
            session: "abc".into()
        }]
    );
}

#[test]
fn assistant_messages_report_text_and_ossia_tools() {
    let line = r#"{"type":"assistant","message":{"content":[
        {"type":"text","text":"Transposing now."},
        {"type":"tool_use","name":"mcp__ossia__ossia_edit","input":{}},
        {"type":"text","text":"  "},
        {"type":"tool_use","name":"Read","input":{}}
    ]}}"#
        .replace('\n', "");
    assert_eq!(
        parse_line(&line),
        vec![
            AssistantEvent::Text {
                text: "Transposing now.".into()
            },
            AssistantEvent::Tool {
                name: "ossia_edit".into()
            },
            AssistantEvent::Tool {
                name: "Read".into()
            },
        ]
    );
}

#[test]
fn results_finish_with_or_without_an_error() {
    assert_eq!(
        parse_line(r#"{"type":"result","subtype":"success","is_error":false,"result":"Done","session_id":"abc"}"#),
        vec![AssistantEvent::Done {
            session: Some("abc".into()),
            error: None
        }]
    );
    assert_eq!(
        parse_line(r#"{"type":"result","subtype":"error_max_turns","is_error":true,"session_id":"abc"}"#),
        vec![AssistantEvent::Done {
            session: Some("abc".into()),
            error: Some("error_max_turns".into())
        }]
    );
}

#[test]
fn other_lines_are_ignored() {
    assert!(parse_line("not json").is_empty());
    assert!(parse_line(r#"{"type":"user","message":{"content":[]}}"#).is_empty());
    assert!(parse_line(r#"{"type":"system","subtype":"hook"}"#).is_empty());
}

#[test]
fn system_prompt_names_the_open_score() {
    let prompt = system_prompt("# Ossia", "doc-1", "Allemanda");
    assert!(prompt.starts_with("# Ossia"));
    assert!(prompt.contains("\"Allemanda\" (documentId `doc-1`)"));
    for name in PRELOADED_GUIDES {
        let text = ossia_backend::guide_document(name).unwrap();
        assert!(prompt.contains(&format!("<ossia-guide document=\"{name}\">\n{text}")));
    }
}

#[test]
fn search_path_starts_with_the_claude_directory() {
    let path = search_path(Path::new("/Users/me/.local/bin/claude"));
    let first = std::env::split_paths(&path).next().unwrap();
    assert_eq!(first, Path::new("/Users/me/.local/bin"));
}
