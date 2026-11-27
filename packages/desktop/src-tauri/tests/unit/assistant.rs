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
        parse_line(
            r#"{"type":"result","subtype":"success","is_error":false,"result":"Done","session_id":"abc"}"#
        ),
        vec![AssistantEvent::Done {
            session: Some("abc".into()),
            error: None
        }]
    );
    assert_eq!(
        parse_line(
            r#"{"type":"result","subtype":"error_max_turns","is_error":true,"session_id":"abc"}"#
        ),
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

#[test]
fn codex_json_lines_report_the_thread_reply_and_ossia_tool() {
    let mut stream_error = None;
    let lines = [
        r#"{"type":"thread.started","thread_id":"thread-1"}"#,
        r#"{"type":"item.completed","item":{"type":"reasoning","text":"not a reply"}}"#,
        r#"{"type":"item.started","item":{"type":"mcp_tool_call","server":"ossia","tool":"ossia_context"}}"#,
        r#"{"type":"item.completed","item":{"type":"mcp_tool_call","server":"ossia","tool":"ossia_context"}}"#,
        r#"{"type":"item.completed","item":{"type":"agent_message","text":"The score is ready."}}"#,
        r#"{"type":"error","message":"Reconnecting"}"#,
        r#"{"type":"turn.completed","usage":{}}"#,
    ];
    assert_eq!(
        lines
            .into_iter()
            .flat_map(|line| parse_codex_line(line, &mut stream_error))
            .collect::<Vec<_>>(),
        vec![
            AssistantEvent::Session {
                session: "thread-1".into()
            },
            AssistantEvent::Tool {
                name: "ossia_context".into()
            },
            AssistantEvent::Text {
                text: "The score is ready.".into()
            },
            AssistantEvent::Done {
                session: None,
                error: None
            },
        ],
    );
    assert_eq!(
        parse_codex_line(
            r#"{"type":"turn.failed","error":{"message":"Not signed in"}}"#,
            &mut stream_error
        ),
        vec![AssistantEvent::Done {
            session: None,
            error: Some("Not signed in".into())
        }],
    );
}

#[test]
fn codex_resume_keeps_the_ossia_mcp_override_and_passes_the_prompt_positionally() {
    let launch = AssistantLaunch {
        executable: "/tools/codex".into(),
        codex_home: "/codex-profile".into(),
        mcp_command: "/Ossia App/ossia".into(),
        mcp_args: vec!["mcp".into(), "--home".into(), "/score library".into()],
        mcp_env: [("OSSIA_ENGINE_ROOT".into(), r"C:\Ossia Engine".into())].into(),
        instructions: "# Ossia".into(),
    };
    let args = AssistantSendArgs {
        provider: AssistantProvider::Codex,
        document_id: "score-1".into(),
        title: "Allemanda".into(),
        run_id: "run-1".into(),
        message: "--not-an-option".into(),
        session: Some("thread-1".into()),
    };
    let command = assistant_command(&launch, &args);
    assert_eq!(command.get_program(), "/tools/codex");
    let argv = command
        .get_args()
        .map(|arg| arg.to_str().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        &argv[..6],
        [
            "exec",
            "--json",
            "--skip-git-repo-check",
            "--ignore-user-config",
            "--sandbox",
            "read-only"
        ]
    );
    assert!(argv.iter().any(|arg| arg
        .contains(r#"command = "/Ossia App/ossia", args = ["mcp","--home","/score library"]"#)));
    assert!(argv
        .iter()
        .any(|arg| arg.contains(r#""OSSIA_ENGINE_ROOT" = "C:\\Ossia Engine""#)));
    assert_eq!(
        &argv[argv.len() - 4..],
        ["resume", "--", "thread-1", "--not-an-option"]
    );
}
