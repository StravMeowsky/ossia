use super::*;

#[cfg(unix)]
#[test]
fn status_requires_the_ossia_mcp_entry() {
    use std::os::unix::fs::PermissionsExt;

    let (root, integrator) = fixture();
    let host = root.path().join("codex");
    std::fs::write(
        &host,
        "#!/bin/sh\ncase \"$1 $2\" in\n  \"plugin list\") printf '%s' \"$OSSIA_TEST_PLUGIN_JSON\" ;;\n  \"plugin marketplace\") printf '%s' \"$OSSIA_TEST_MARKETPLACE_JSON\" ;;\n  \"mcp list\") printf '%s' \"$OSSIA_TEST_MCP_JSON\" ;;\nesac\n",
    )
    .unwrap();
    std::fs::set_permissions(&host, std::fs::Permissions::from_mode(0o755)).unwrap();

    let plugin = json!({
        "installed": [{
            "pluginId": PLUGIN_ID,
            "enabled": true,
            "source": {"path": plugin_dir(integrator.shared_root())}
        }]
    });
    let marketplace = json!({
        "marketplaces": [{
            "name": MARKETPLACE,
            "root": marketplace_root(integrator.shared_root())
        }]
    });
    let record = ManagedRecord {
        management_id: "ossia:codex:default".into(),
        agent_id: "codex".into(),
        method: InstallMethod::Plugin,
        profile: "default".into(),
        template_version: TEMPLATE_VERSION,
        home: root.path().to_string_lossy().into_owned(),
        executable: host.to_string_lossy().into_owned(),
        shared_root: integrator.shared_root().to_string_lossy().into_owned(),
        updated_at: String::new(),
        entries: Vec::new(),
    };
    let detected = DetectedAgent {
        command: host.clone(),
        command_args: Vec::new(),
        executable: host,
        version: "test".into(),
        config_root: integrator.paths().codex_home.clone(),
        profile: "default",
        env: BTreeMap::from([
            ("OSSIA_TEST_PLUGIN_JSON".into(), plugin.to_string()),
            (
                "OSSIA_TEST_MARKETPLACE_JSON".into(),
                marketplace.to_string(),
            ),
            (
                "OSSIA_TEST_MCP_JSON".into(),
                json!({"servers": [{"name": "ossia", "enabled": true}]}).to_string(),
            ),
        ]),
    };
    let ctx = AgentContext {
        integrator: &integrator,
        detected: &detected,
    };

    let installed = status::status(&ctx, Some(&record)).unwrap();
    assert_eq!(installed.state, AgentState::Installed);
    let expected_text = path_text(plugin_dir(integrator.shared_root()));
    assert_eq!(installed.path.as_deref(), Some(expected_text.as_str()));
    assert!(!installed.path.unwrap().contains(r"\\?\"));

    let detected = DetectedAgent {
        env: BTreeMap::from([
            ("OSSIA_TEST_PLUGIN_JSON".into(), plugin.to_string()),
            (
                "OSSIA_TEST_MARKETPLACE_JSON".into(),
                marketplace.to_string(),
            ),
            (
                "OSSIA_TEST_MCP_JSON".into(),
                json!({"servers": []}).to_string(),
            ),
        ]),
        ..detected
    };
    let ctx = AgentContext {
        integrator: &integrator,
        detected: &detected,
    };
    let missing = status::status(&ctx, Some(&record)).unwrap();
    assert_eq!(missing.state, AgentState::Broken);
}
