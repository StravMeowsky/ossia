#![cfg(target_os = "macos")]
use ossia_integrations::{HostPaths, Integrator};
use ossia_types::{AgentState, AgentsChange};
use std::{collections::BTreeMap, fs, os::unix::fs::PermissionsExt};

#[test]
fn claude_code_is_independent_of_retired_desktop_integration() {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path().join("home");
    let app = home.join("Applications/Claude.app/Contents/MacOS/Claude");
    fs::create_dir_all(app.parent().unwrap()).unwrap();
    fs::write(&app, "app").unwrap();
    let cli = temp.path().join("claude-cli");
    let cli_home = temp.path().join("code config");
    std::env::set_var("OSSIA_CLAUDE_BIN", &cli);
    std::env::set_var("CLAUDE_CONFIG_DIR", &cli_home);
    let paths = HostPaths {
        codex_home: temp.path().join("codex"),
        dsh_home: temp.path().join("dsh"),
        workbuddy_home: home.clone(),
        workbuddy_apps: vec![],
        codex_bin: Some(temp.path().join("no-codex")),
        dsh_bin: Some(temp.path().join("no-dsh")),
        launch_env: BTreeMap::new(),
    };
    let integrator = Integrator::with_paths(
        temp.path().join("library"),
        temp.path().join("ossia"),
        paths,
        3212,
    )
    .unwrap();
    let desktop_config = home.join("Library/Application Support/Claude/claude_desktop_config.json");
    fs::create_dir_all(desktop_config.parent().unwrap()).unwrap();
    let original =
        r#"{"mcpServers":{"ossia":{"command":"/old/ossia"},"other":{"command":"/other"}}}"#;
    fs::write(&desktop_config, original).unwrap();
    let managed = integrator.home().join("integrations/managed.json");
    fs::create_dir_all(managed.parent().unwrap()).unwrap();
    fs::write(
        &managed,
        serde_json::to_vec(&serde_json::json!({
            "schemaVersion": 2,
            "connectionOrder": ["claude"],
            "enabledGroups": ["claude"],
            "groupProblems": {"ossia:claude:desktop": "old problem"},
            "records": {
                "ossia:claude:desktop": {
                    "managementId": "ossia:claude:desktop",
                    "agentId": "claude",
                    "method": "mcp",
                    "profile": "desktop",
                    "templateVersion": 2,
                    "home": integrator.home(),
                    "executable": "/old/ossia",
                    "sharedRoot": "/old/integrations",
                    "updatedAt": "unix:1",
                    "entries": []
                }
            }
        }))
        .unwrap(),
    )
    .unwrap();

    let desktop_only = integrator.agents().unwrap();
    assert!(desktop_only.agents.iter().all(|agent| agent.id != "claude"));
    assert!(!desktop_only
        .connection_order
        .iter()
        .any(|id| id == "claude"));

    fs::write(&cli, r##"#!/usr/bin/env python3
import json, os, pathlib, sys
root=pathlib.Path(os.environ['CLAUDE_CONFIG_DIR']); root.mkdir(parents=True, exist_ok=True)
a=sys.argv[1:]; plugin=root/'plugins'; plugin.mkdir(exist_ok=True); known=plugin/'known_marketplaces.json'; installed=root/'list.json'
if a == ['--version']: print('2.1.123 (Claude Code)')
elif a == ['plugin','--help']: print('plugins')
elif a[:2] == ['plugin','list']: print(installed.read_text() if installed.exists() else '[]')
elif a[:3] == ['plugin','marketplace','add']:
 p=a[3]; known.write_text(json.dumps({'ossia-claude':{'source':{'path':p}}}))
elif a[:3] == ['plugin','marketplace','remove']:
 known.unlink(missing_ok=True); installed.unlink(missing_ok=True)
elif a[:2] == ['plugin','install']:
 p=json.loads(known.read_text())['ossia-claude']['source']['path']; manifest=json.loads((pathlib.Path(p)/'.claude-plugin/marketplace.json').read_text()); version=manifest['plugins'][0]['version']; cache=plugin/'cache'; cache.mkdir(exist_ok=True); installed.write_text(json.dumps([{'id':'ossia@ossia-claude','scope':'user','version':version,'enabled':True,'installPath':str(cache)}]))
elif a[:2] == ['plugin','uninstall']: installed.unlink(missing_ok=True)
"##).unwrap();
    fs::set_permissions(&cli, fs::Permissions::from_mode(0o755)).unwrap();
    let available = integrator.sync_managed().unwrap();
    let code = available.agents.iter().find(|a| a.id == "claude").unwrap();
    assert_eq!(code.name, "Claude Code");
    assert_eq!(code.state, AgentState::Available);
    assert!(!cli_home.join("list.json").exists());
    let store: serde_json::Value = serde_json::from_slice(&fs::read(&managed).unwrap()).unwrap();
    assert!(store["records"].get("ossia:claude:desktop").is_none());

    let change = |enabled| AgentsChange {
        agent_ids: vec!["claude".into()],
        enabled,
        overwrite: false,
    };
    let connected = integrator.set_agents(change(true)).unwrap();
    assert!(connected.problems.is_empty(), "{:?}", connected.problems);
    let snapshot = integrator.sync_managed().unwrap();
    let code = snapshot.agents.iter().find(|a| a.id == "claude").unwrap();
    assert_eq!(code.name, "Claude Code");
    assert_eq!(code.state, AgentState::Installed);
    assert_eq!(code.version.as_deref(), Some("2.1.123"));
    assert!(code.hosts.is_empty());
    assert_eq!(snapshot.connection_order, ["claude"]);

    let list_path = cli_home.join("list.json");
    let mut installed: serde_json::Value =
        serde_json::from_slice(&fs::read(&list_path).unwrap()).unwrap();
    installed[0]["enabled"] = false.into();
    fs::write(&list_path, serde_json::to_vec(&installed).unwrap()).unwrap();
    let disabled = integrator.sync_managed().unwrap();
    let code = disabled.agents.iter().find(|a| a.id == "claude").unwrap();
    assert_eq!(code.state, AgentState::Disabled);
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&fs::read(&list_path).unwrap()).unwrap()[0]
            ["enabled"],
        false
    );

    let removed = integrator.set_agents(change(false)).unwrap();
    assert!(removed.problems.is_empty(), "{:?}", removed.problems);
    let snapshot = integrator.sync_managed().unwrap();
    let code = snapshot.agents.iter().find(|a| a.id == "claude").unwrap();
    assert_eq!(code.state, AgentState::Available);
    assert!(!snapshot.connection_order.iter().any(|id| id == "claude"));
    assert!(!list_path.exists());
    assert_eq!(fs::read_to_string(desktop_config).unwrap(), original);
}
