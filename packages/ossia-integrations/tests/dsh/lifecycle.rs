use super::*;
use lifecycle::install;
#[cfg(windows)]
use lifecycle::sync;
use status::status;

fn linked_profile(target: &Path) -> Value {
    json!({
        "name": "dsh-profile-desktop",
        "private": true,
        "dsh": {
            "profile": {
                "bundles": [PLUGIN]
            }
        },
        "dependencies": {
            PLUGIN: format!("link:{}", target.display())
        }
    })
}

fn write_profile(integrator: &Integrator, profile: &Value) -> PathBuf {
    let path = integrator
        .paths()
        .dsh_home
        .join("profiles/desktop/package.json");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, serde_json::to_vec_pretty(profile).unwrap()).unwrap();
    path
}

#[test]
fn deleted_bundle_target_is_broken_and_reinstallable() {
    let (_root, integrator) = fixture();
    let detected = DetectedAgent {
        executable: std::env::current_exe().unwrap(),
        command: std::env::current_exe().unwrap(),
        command_args: Vec::new(),
        version: "test".into(),
        config_root: integrator.paths().dsh_home.clone(),
        profile: "desktop",
        env: BTreeMap::new(),
    };
    let ctx = context(&integrator, &detected);
    let expected = bundle_dir(&ctx);
    write_profile(&integrator, &linked_profile(&expected));

    let inspection = status(&ctx, None).unwrap();
    assert_eq!(inspection.state, AgentState::Broken);
    assert_eq!(inspection.message, Some(AgentNotice::DshBundleMissing));
    assert!(!inspection.installed);
    assert!(inspection.conflicts.is_empty());
    let expected_text = path_text(&expected);
    assert_eq!(inspection.path.as_deref(), Some(expected_text.as_str()));
    assert!(!inspection.path.unwrap().contains(r"\\?\"));

    let entries = install(&ctx, None, false).unwrap();
    assert!(expected.join("package.json").is_file());
    assert!(!entries.is_empty());
    let record = ManagedRecord {
        management_id: "ossia:dsh:desktop".into(),
        agent_id: "dsh".into(),
        method: InstallMethod::Plugin,
        profile: "desktop".into(),
        template_version: TEMPLATE_VERSION,
        home: integrator.home().to_string_lossy().into_owned(),
        executable: integrator.executable().to_string_lossy().into_owned(),
        shared_root: integrator.shared_root().to_string_lossy().into_owned(),
        updated_at: "unix:0".into(),
        entries,
    };
    let inspection = status(&ctx, Some(&record)).unwrap();
    assert_eq!(inspection.state, AgentState::Installed);
}

#[test]
fn existing_unmanaged_bundle_remains_a_conflict() {
    let (_root, integrator) = fixture();
    let detected = DetectedAgent {
        executable: std::env::current_exe().unwrap(),
        command: std::env::current_exe().unwrap(),
        command_args: Vec::new(),
        version: "test".into(),
        config_root: integrator.paths().dsh_home.clone(),
        profile: "desktop",
        env: BTreeMap::new(),
    };
    let ctx = context(&integrator, &detected);
    let expected = bundle_dir(&ctx);
    write_profile(&integrator, &linked_profile(&expected));
    fs::create_dir_all(&expected).unwrap();

    let inspection = status(&ctx, None).unwrap();
    assert_eq!(inspection.state, AgentState::Conflict);
    assert_eq!(inspection.message, Some(AgentNotice::DshUnmanagedBundle));
    assert_eq!(inspection.conflicts.len(), 1);
    assert!(!inspection.path.unwrap().contains(r"\\?\"));
    assert!(install(&ctx, None, false).is_err());
    assert!(install(&ctx, None, true).is_ok());
}

#[cfg(unix)]
#[test]
fn explicit_override_replaces_a_foreign_dsh_link() {
    use std::os::unix::fs::PermissionsExt;

    let (root, integrator) = fixture();
    let host = root.path().join("bin/dsh");
    fs::create_dir_all(host.parent().unwrap()).unwrap();
    fs::write(&host, "#!/bin/sh\nexit 0\n").unwrap();
    fs::set_permissions(&host, fs::Permissions::from_mode(0o755)).unwrap();
    let detected = DetectedAgent {
        command: host.clone(),
        command_args: Vec::new(),
        executable: host,
        version: "test".into(),
        config_root: integrator.paths().dsh_home.clone(),
        profile: "desktop",
        env: BTreeMap::new(),
    };
    let ctx = context(&integrator, &detected);
    let expected = bundle_dir(&ctx);
    write_profile(
        &integrator,
        &serde_json::json!({
            "private": true,
            "dsh": {"profile": {"bundles": [PLUGIN]}},
            "dependencies": {PLUGIN: "link:/tmp/other-ossia-dsh"}
        }),
    );

    let inspection = status(&ctx, None).unwrap();
    assert_eq!(inspection.state, AgentState::Conflict);
    assert!(install(&ctx, None, false).is_err());
    assert!(install(&ctx, None, true).is_ok());
    assert!(expected.join("package.json").is_file());
}

#[cfg(windows)]
#[test]
fn sync_repairs_a_verbatim_bundle_source_path() {
    let (root, integrator) = fixture();
    let log = root.path().join("dsh-args.txt");
    let host = root.path().join("DeepSeek Harness/dsh.cmd");
    fs::create_dir_all(host.parent().unwrap()).unwrap();
    fs::write(
        &host,
        "@echo off\r\n>>\"%DSH_TEST_LOG%\" echo %*\r\nexit /b 0\r\n",
    )
    .unwrap();
    let detected = DetectedAgent {
        command: host.clone(),
        command_args: Vec::new(),
        executable: host,
        version: "test".into(),
        config_root: integrator.paths().dsh_home.clone(),
        profile: "desktop",
        env: BTreeMap::from([("DSH_TEST_LOG".into(), log.to_string_lossy().into_owned())]),
    };
    let ctx = context(&integrator, &detected);
    let expected = bundle_dir(&ctx);
    fs::create_dir_all(&expected).unwrap();
    let raw = format!("//?/{}", path_text(&expected).replace('\\', "/"));
    write_profile(
        &integrator,
        &serde_json::json!({
            "private": true,
            "dsh": {"profile": {"bundles": [PLUGIN]}},
            "dependencies": {PLUGIN: format!("link:{raw}")}
        }),
    );
    let record = ManagedRecord {
        management_id: "ossia:dsh:desktop".into(),
        agent_id: "dsh".into(),
        method: InstallMethod::Plugin,
        profile: "desktop".into(),
        template_version: TEMPLATE_VERSION,
        home: path_text(integrator.home()),
        executable: path_text(integrator.executable()),
        shared_root: path_text(integrator.shared_root()),
        updated_at: "unix:0".into(),
        entries: Vec::new(),
    };

    assert!(sync(&ctx, &record).unwrap().is_some());
    let arguments = fs::read_to_string(log).unwrap();
    assert!(arguments.contains(&path_text(&expected)), "{arguments:?}");
    assert!(!arguments.contains(r"\\?\"), "{arguments:?}");
    assert!(!arguments.contains("//?/"), "{arguments:?}");
}
