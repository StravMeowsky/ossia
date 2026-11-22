use super::*;
use std::collections::BTreeMap;

#[test]
fn shared_integration_root_follows_the_runtime_home() {
    let root = tempfile::tempdir().unwrap();
    let home = root.path().join(".ossia");
    let paths = HostPaths {
        codex_home: root.path().join("codex"),
        dsh_home: root.path().join("dsh"),
        workbuddy_home: root.path().join("home"),
        workbuddy_apps: Vec::new(),
        codex_bin: None,
        dsh_bin: None,
        launch_env: BTreeMap::new(),
    };
    let integrator = Integrator::with_paths(
        home.clone(),
        std::env::current_exe().unwrap(),
        paths,
        DEFAULT_BACKEND_PORT,
    )
    .unwrap();
    assert_eq!(integrator.shared_root(), home.join("integrations/hosts"));
}

#[test]
fn launch_environment_values_are_normalized() {
    let root = tempfile::tempdir().unwrap();
    let paths = HostPaths {
        codex_home: root.path().join("codex"),
        dsh_home: root.path().join("dsh"),
        workbuddy_home: root.path().join("home"),
        workbuddy_apps: Vec::new(),
        codex_bin: None,
        dsh_bin: None,
        launch_env: BTreeMap::from([(
            "OSSIA_ENGINE_ROOT".into(),
            r"\\?\E:\Projects\ossia\engine".into(),
        )]),
    };
    let integrator = Integrator::with_paths(
        root.path().join(".ossia"),
        std::env::current_exe().unwrap(),
        paths,
        DEFAULT_BACKEND_PORT,
    )
    .unwrap();

    assert_eq!(
        integrator
            .launch_env()
            .get("OSSIA_ENGINE_ROOT")
            .map(String::as_str),
        Some(r"E:\Projects\ossia\engine")
    );
}

#[test]
fn mcp_command_uses_the_integrator_port() {
    let root = tempfile::tempdir().unwrap();
    let integrator = Integrator::with_paths(
        root.path().join(".ossia"),
        std::env::current_exe().unwrap(),
        HostPaths {
            codex_home: root.path().join("codex"),
            dsh_home: root.path().join("dsh"),
            workbuddy_home: root.path().join("home"),
            workbuddy_apps: Vec::new(),
            codex_bin: None,
            dsh_bin: None,
            launch_env: BTreeMap::new(),
        },
        3211,
    )
    .unwrap();

    assert_eq!(
        integrator.mcp_command(),
        vec![
            "mcp".to_string(),
            "--home".to_string(),
            path_text(integrator.home()),
            "--port".to_string(),
            "3211".to_string()
        ]
    );
}
