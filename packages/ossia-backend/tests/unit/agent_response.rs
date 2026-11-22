use super::*;

#[test]
fn status_and_context_keep_task_data_and_drop_transport_internals() {
    let input = json!({"instanceId":"i", "seq":3, "engineBuildId":"e", "documentCount":4,
        "runtime":{"instanceId":"i","webBaseUrl":"http://localhost:3212","capabilities":["mcp"]},
        "jobs":[{"jobId":"j"}], "language":"zh-CN", "update":{"state":"found"},
        "documentation":{"available":false,"root":"/private/docs","error":{"message":"missing"}}});
    let result = compact("status", &json!({}), input).unwrap();
    assert_eq!(
        result["runtime"],
        json!({"webBaseUrl":"http://localhost:3212"})
    );
    assert_eq!(
        result["documentation"],
        json!({"available":false,"error":{"message":"missing"}})
    );
    assert_eq!(result["jobs"][0]["jobId"], "j");
    assert_eq!(result["update"]["state"], "found");
    assert!(result.get("seq").is_none());
    let context = compact(
        "context",
        &json!({}),
        json!({"instanceId":"i","seq":1,
        "document":{"sourceHash":"current","lastCheck":{"sourceHash":"old"}},
        "render":{"renderKey":"key","sourceHash":"old","pages":2},"regions":[{"label":"note"}]}),
    )
    .unwrap();
    assert_eq!(context["render"], json!({"sourceHash":"old","pages":2}));
    assert_eq!(context["document"]["lastCheck"]["sourceHash"], "old");
    assert_eq!(context["regions"][0]["label"], "note");
}

#[test]
fn check_summary_preserves_diagnostics_and_full_job_is_untouched() {
    let job = json!({"kind":"check","jobId":"j","state":"done","result":{
        "render":{"sourceHash":"h","engineBuildId":"e","optionsHash":"o",
        "pages":[{"page":1,"path":"private","sha256":"s","bytes":10}],
        "pdf":null,"midi":{"path":"private"},
        "diagnostics":[{"severity":"warning","message":"warning"}],"formatErrors":[{"format":"pdf"}]},
        "verification":{"status":"failed","differences":[{"message":"wrong pitch"}]}}});
    let immediate = compact("check", &json!({}), job.clone()).unwrap();
    let summary = compact("job", &json!({}), json!({"job":job,"analysis":null})).unwrap();
    assert_eq!(immediate, summary["job"]);
    assert_eq!(
        immediate["result"]["render"],
        json!({"sourceHash":"h","pageCount":1,
        "pdfAvailable":false,"midiAvailable":true,
        "diagnostics":[{"severity":"warning","message":"warning"}],"formatErrors":[{"format":"pdf"}]})
    );
    assert_eq!(
        immediate["result"]["verification"]["differences"][0]["message"],
        "wrong pitch"
    );
    let full = json!({"job":{"kind":"check","result":{"render":{"pages":[{"path":"private"}]}}}});
    assert_eq!(
        compact("job", &json!({"detail":"full"}), full.clone()).unwrap(),
        full
    );
}

#[test]
fn import_and_output_summaries_preserve_deliverables_and_failure_details() {
    let import = json!({"kind":"import-request","result":{"report":{"engineBuildId":"e",
        "status":"needs-confirmation","confirmation":"h","diagnostics":[{"message":"warning"}]},
        "document":{"sourcePath":"private","documentId":"d"}}});
    let import = compact("import", &json!({}), import).unwrap();
    assert!(import["result"]["report"].get("engineBuildId").is_none());
    assert!(import["result"]["document"].get("sourcePath").is_none());
    assert_eq!(
        import["result"]["report"]["diagnostics"][0]["message"],
        "warning"
    );
    assert_eq!(import["result"]["report"]["confirmation"], "h");
    let output = json!({"kind":"output","result":{"source":{"kind":"version","versionId":"v"},
        "sourceHash":"h","engineBuildId":"e","format":"png","directory":"/tmp/out",
        "files":[{"page":1,"path":"/tmp/out/1.png","bytes":42,"sha256":"s","mediaType":"image/png","bounds":{"x0":1}}]}});
    let immediate = compact("output", &json!({}), output.clone()).unwrap();
    let later = compact("job", &json!({}), json!({"job":output})).unwrap();
    assert_eq!(immediate, later["job"]);
    assert_eq!(
        immediate["result"]["files"][0],
        json!({"page":1,"path":"/tmp/out/1.png","bytes":42,"bounds":{"x0":1}})
    );
    assert_eq!(immediate["result"]["source"]["versionId"], "v");
}
