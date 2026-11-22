use ossia_integrations::{HostPaths, Integrator};
use ossia_types::{AgentState, AgentsChange, DEFAULT_BACKEND_PORT};
use std::{collections::BTreeMap, path::PathBuf};

fn resources() -> PathBuf {
    PathBuf::from(
        std::env::var_os("OSSIA_TEST_INTEGRATION_ROOT").expect("set OSSIA_TEST_INTEGRATION_ROOT"),
    )
}

fn executable() -> PathBuf {
    PathBuf::from(std::env::var_os("OSSIA_TEST_OSSIA_BIN").expect("set OSSIA_TEST_OSSIA_BIN"))
}

fn paths(work: &tempfile::TempDir) -> HostPaths {
    HostPaths {
        codex_home: work.path().join("codex home"),
        dsh_home: work.path().join("dsh"),
        workbuddy_home: work.path().join("home"),
        workbuddy_apps: Vec::new(),
        codex_bin: std::env::var_os("OSSIA_TEST_CODEX_BIN").map(PathBuf::from),
        dsh_bin: std::env::var_os("OSSIA_TEST_DSH_BIN").map(PathBuf::from),
        launch_env: BTreeMap::new(),
    }
}

fn integrator(work: &tempfile::TempDir) -> Integrator {
    Integrator::with_resource_dir(
        work.path().join("中文 library"),
        executable(),
        resources(),
        paths(work),
        DEFAULT_BACKEND_PORT,
    )
    .unwrap()
    .with_shared_root(work.path().join("shared"))
    .unwrap()
}

#[test]
#[ignore = "requires OSSIA_TEST_INTEGRATION_ROOT, OSSIA_TEST_OSSIA_BIN, and the Codex CLI"]
fn codex_plugin_install_and_remove_round_trip() {
    let work = tempfile::tempdir().unwrap();
    let integrator = integrator(&work);
    let before = integrator.agents().unwrap();
    assert!(before.agents.iter().any(|agent| agent.id == "codex"));

    let installed = integrator
        .set_agents(AgentsChange {
            agent_ids: vec!["codex".into()],
            enabled: true,
            overwrite: false,
        })
        .unwrap();
    assert!(installed.problems.is_empty(), "{:?}", installed.problems);
    let codex = installed
        .snapshot
        .agents
        .iter()
        .find(|agent| agent.id == "codex")
        .unwrap();
    assert!(matches!(
        codex.state,
        AgentState::Installed | AgentState::Disabled
    ));

    let removed = integrator
        .set_agents(AgentsChange {
            agent_ids: vec!["codex".into()],
            enabled: false,
            overwrite: false,
        })
        .unwrap();
    assert!(removed.problems.is_empty(), "{:?}", removed.problems);
    let codex = removed
        .snapshot
        .agents
        .iter()
        .find(|agent| agent.id == "codex")
        .unwrap();
    assert!(!codex.installed);
}

#[test]
#[ignore = "requires OSSIA_TEST_INTEGRATION_ROOT, OSSIA_TEST_OSSIA_BIN, and the DSH CLI"]
fn dsh_plugin_install_and_remove_round_trip() {
    let work = tempfile::tempdir().unwrap();
    let integrator = integrator(&work);
    let before = integrator.agents().unwrap();
    assert!(before.agents.iter().any(|agent| agent.id == "dsh"));

    let installed = integrator
        .set_agents(AgentsChange {
            agent_ids: vec!["dsh".into()],
            enabled: true,
            overwrite: false,
        })
        .unwrap();
    assert!(installed.problems.is_empty(), "{:?}", installed.problems);
    let dsh = installed
        .snapshot
        .agents
        .iter()
        .find(|agent| agent.id == "dsh")
        .unwrap();
    assert_eq!(dsh.state, AgentState::Installed);

    let removed = integrator
        .set_agents(AgentsChange {
            agent_ids: vec!["dsh".into()],
            enabled: false,
            overwrite: false,
        })
        .unwrap();
    assert!(removed.problems.is_empty(), "{:?}", removed.problems);
    let dsh = removed
        .snapshot
        .agents
        .iter()
        .find(|agent| agent.id == "dsh")
        .unwrap();
    assert!(!dsh.installed);
}

#[test]
#[ignore = "requires a WorkBuddy app and an Ossia executable"]
fn workbuddy_plugin_install_and_remove_round_trip() {
    let work = tempfile::tempdir().unwrap();
    let app = std::env::var_os("OSSIA_TEST_WORKBUDDY_APP")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/Applications/WorkBuddy.app"));
    let paths = HostPaths {
        codex_home: work.path().join("codex"),
        dsh_home: work.path().join("dsh"),
        workbuddy_home: work.path().join("workbuddy-home"),
        workbuddy_apps: vec![app],
        codex_bin: Some(work.path().join("missing-codex")),
        dsh_bin: Some(work.path().join("missing-dsh")),
        launch_env: BTreeMap::new(),
    };
    let integrator = Integrator::with_paths(
        work.path().join("library"),
        executable(),
        paths,
        DEFAULT_BACKEND_PORT,
    )
    .unwrap()
    .with_shared_root(work.path().join("shared"))
    .unwrap();
    let before = integrator.agents().unwrap();
    let before = before
        .agents
        .iter()
        .find(|agent| agent.name == "WorkBuddy")
        .expect("WorkBuddy should be detected");
    assert!(before.installed == false);

    let installed = integrator
        .set_agents(AgentsChange {
            agent_ids: vec![before.id.clone()],
            enabled: true,
            overwrite: false,
        })
        .unwrap();
    assert!(installed.problems.is_empty(), "{:?}", installed.problems);
    let workbuddy = installed
        .snapshot
        .agents
        .iter()
        .find(|agent| agent.id == before.id)
        .unwrap();
    assert_eq!(workbuddy.state, AgentState::Installed);

    let removed = integrator
        .set_agents(AgentsChange {
            agent_ids: vec![before.id.clone()],
            enabled: false,
            overwrite: false,
        })
        .unwrap();
    assert!(removed.problems.is_empty(), "{:?}", removed.problems);
    let workbuddy = removed
        .snapshot
        .agents
        .iter()
        .find(|agent| agent.id == before.id)
        .unwrap();
    assert!(!workbuddy.installed);
}
