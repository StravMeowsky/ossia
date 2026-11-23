use super::*;
use crate::{HostPaths, Integrator};
use ossia_types::DEFAULT_BACKEND_PORT;
use std::{collections::BTreeMap, fs};

fn fixture() -> (tempfile::TempDir, Integrator, DetectedAgent) {
    let root = tempfile::tempdir().unwrap();
    let home = root.path().join("library");
    let shared = root.path().join("shared");
    let config_root = root.path().join("home/.workbuddy");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(&shared).unwrap();
    fs::create_dir_all(&config_root).unwrap();
    let executable = root.path().join("ossia");
    fs::write(&executable, b"test executable").unwrap();
    let paths = HostPaths {
        codex_home: root.path().join("codex"),
        dsh_home: root.path().join("dsh"),
        workbuddy_home: root.path().join("home"),
        workbuddy_apps: Vec::new(),
        codex_bin: None,
        dsh_bin: None,
        launch_env: BTreeMap::from([(
            "OSSIA_ENGINE_ROOT".into(),
            format!(r"\\?\{}", root.path().join("engine").display()),
        )]),
    };
    let resources = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources");
    let integrator =
        Integrator::with_resource_dir(home, executable, resources, paths, DEFAULT_BACKEND_PORT)
            .unwrap()
            .with_shared_root(shared)
            .unwrap();
    let detected = DetectedAgent {
        executable: root.path().join("codebuddy"),
        command: root.path().join("node"),
        command_args: vec![path_text(root.path().join("codebuddy"))],
        version: "5.11.3 (CLI 2.176.0)".into(),
        config_root,
        profile: "desktop",
        env: BTreeMap::new(),
    };
    (root, integrator, detected)
}

#[test]
fn plugin_files_include_marketplace_skill_and_mcp() {
    let (_root, integrator, detected) = fixture();
    let ctx = AgentContext {
        integrator: &integrator,
        detected: &detected,
    };
    let (version, files) = plugin_files(&ctx).unwrap();
    assert!(version.starts_with(concat!(env!("CARGO_PKG_VERSION"), "+ossia.")));
    assert_eq!(files.len(), 8);
    let marketplace = files
        .iter()
        .find(|file| file.kind == "marketplace")
        .unwrap();
    assert!(marketplace
        .path
        .ends_with(".codebuddy-plugin/marketplace.json"));
    let marketplace: Value = serde_json::from_slice(&marketplace.bytes).unwrap();
    assert_eq!(marketplace["name"], MARKETPLACE);
    assert_eq!(marketplace["plugins"][0]["name"], PLUGIN);
    assert_eq!(marketplace["plugins"][0]["version"], version);

    let manifest = files
        .iter()
        .find(|file| file.kind == "plugin-manifest")
        .unwrap();
    assert_eq!(manifest.path, plugin_manifest_path(&ctx));
    let manifest: Value = serde_json::from_slice(&manifest.bytes).unwrap();
    assert_eq!(manifest["name"], PLUGIN);
    assert_eq!(manifest["version"], version);
    assert_eq!(manifest["mcpServers"], "./.mcp.json");

    let mcp = files.iter().find(|file| file.kind == "mcp").unwrap();
    assert_eq!(mcp.path, plugin_dir(&ctx).join(".mcp.json"));
    let mcp: Value = serde_json::from_slice(&mcp.bytes).unwrap();
    assert_eq!(
        mcp["mcpServers"]["ossia"]["command"],
        json!(path_text(integrator.executable()))
    );
    assert_eq!(
        mcp["mcpServers"]["ossia"]["args"],
        json!([
            "mcp",
            "--home",
            path_text(integrator.home()),
            "--port",
            DEFAULT_BACKEND_PORT.to_string()
        ])
    );
    assert!(!mcp.to_string().contains(r"\\?\"));
    assert_eq!(
        files
            .iter()
            .filter(|file| file.kind == "skill-reference")
            .count(),
        3
    );
    assert!(files
        .iter()
        .all(|file| file.path.starts_with(integrator.shared_root())));
}

#[test]
fn plugin_version_is_stable_and_binding_sensitive() {
    let (_root, integrator, detected) = fixture();
    let first = {
        let ctx = AgentContext {
            integrator: &integrator,
            detected: &detected,
        };
        plugin_files(&ctx).unwrap().0
    };
    let second = {
        let ctx = AgentContext {
            integrator: &integrator,
            detected: &detected,
        };
        plugin_files(&ctx).unwrap().0
    };
    assert_eq!(first, second);
    let changed = plugin_version(
        b"skill",
        b"piano",
        b"single",
        b"guitar",
        b"icon",
        &json!({"command": "/different/ossia"}),
    );
    let unchanged = plugin_version(
        b"skill",
        b"piano",
        b"single",
        b"guitar",
        b"icon",
        &json!({"command": "/original/ossia"}),
    );
    assert_ne!(unchanged, changed);
    assert_ne!(first, changed);
}

#[cfg(target_os = "macos")]
#[test]
fn duplicate_config_roots_select_the_newer_app() {
    let root = tempfile::tempdir().unwrap();
    let home = root.path().join("home");
    let older = fake_app(root.path(), "WorkBuddy.app", "5.10.0", ".workbuddy");
    let newer = fake_app(root.path(), "WorkBuddyAI.app", "5.11.3", ".workbuddy");
    let paths = HostPaths {
        codex_home: root.path().join("codex"),
        dsh_home: root.path().join("dsh"),
        workbuddy_home: home.clone(),
        workbuddy_apps: vec![older, newer],
        codex_bin: None,
        dsh_bin: None,
        launch_env: BTreeMap::new(),
    };
    let instances = instances(&paths);
    assert_eq!(instances.len(), 1);
    assert_eq!(instances[0].instance.app_version, "5.11.3");
    assert_eq!(instances[0].instance.config_root, home.join(".workbuddy"));
}

#[cfg(target_os = "macos")]
fn fake_app(root: &Path, name: &str, version: &str, data_folder: &str) -> PathBuf {
    let app = root.join(name);
    let cli = app.join("Contents/Resources/app.asar.unpacked/cli");
    fs::create_dir_all(cli.join("bin")).unwrap();
    fs::create_dir_all(app.join("Contents/MacOS")).unwrap();
    fs::write(app.join("Contents/MacOS/Electron"), b"app").unwrap();
    fs::write(cli.join("bin/codebuddy"), b"cli").unwrap();
    fs::write(
        cli.join("product.json"),
        serde_json::to_vec(&json!({
            "applicationName": "WorkBuddy",
            "dataFolderName": data_folder,
            "genieVersion": version
        }))
        .unwrap(),
    )
    .unwrap();
    app
}

#[cfg(unix)]
#[test]
fn install_status_sync_and_remove_round_trip() {
    use std::os::unix::fs::PermissionsExt;

    let (root, integrator, mut detected) = fixture();
    let log = root.path().join("workbuddy.log");
    let script = root.path().join("fake-workbuddy");
    fs::write(
        &script,
        r#"#!/bin/sh
printf '%s\n' "$*" >> "$OSSIA_TEST_WORKBUDDY_LOG"
root="$WORKBUDDY_CONFIG_DIR"
case "$1 $2" in
  "plugin list")
    if [ -f "$root/plugin-list.json" ]; then cat "$root/plugin-list.json"; else printf '[]'; fi
    ;;
  "plugin marketplace")
    case "$3" in
      add)
        mkdir -p "$root/plugins"
        printf '{"ossia-workbuddy":{"installLocation":"%s","source":{"source":"directory","path":"%s"}}}' "$4" "$4" > "$root/plugins/known_marketplaces.json"
        ;;
      remove)
        rm -f "$root/plugins/known_marketplaces.json" "$root/plugin-list.json" "$root/plugins/installed_plugins.json"
        ;;
    esac
    ;;
  "plugin install")
    version="$OSSIA_TEST_WORKBUDDY_VERSION"
    cache="$root/plugins/cache/ossia-workbuddy/ossia/$version"
    mkdir -p "$cache" "$root/plugins"
    printf '{"version":2,"plugins":{"ossia@ossia-workbuddy":[{"scope":"user","installPath":"%s","version":"%s"}]}}' "$cache" "$version" > "$root/plugins/installed_plugins.json"
    printf '[{"id":"ossia@ossia-workbuddy","version":"%s","scope":"user","enabled":true,"installPath":"%s"}]' "$version" "$cache" > "$root/plugin-list.json"
    ;;
  "plugin uninstall")
    rm -f "$root/plugin-list.json" "$root/plugins/installed_plugins.json"
    ;;
esac
"#,
    )
    .unwrap();
    fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
    detected.command = script;
    detected.command_args.clear();
    detected.env = BTreeMap::from([
        (
            "WORKBUDDY_CONFIG_DIR".into(),
            path_text(&detected.config_root),
        ),
        (
            "CODEBUDDY_CONFIG_DIR".into(),
            path_text(&detected.config_root),
        ),
        ("OSSIA_TEST_WORKBUDDY_LOG".into(), path_text(&log)),
    ]);
    let version = {
        let ctx = AgentContext {
            integrator: &integrator,
            detected: &detected,
        };
        plugin_files(&ctx).unwrap().0
    };
    detected
        .env
        .insert("OSSIA_TEST_WORKBUDDY_VERSION".into(), version.clone());
    let ctx = AgentContext {
        integrator: &integrator,
        detected: &detected,
    };
    let mut entries = lifecycle::install(&ctx, None, false).unwrap();
    assert!(entries.iter().any(|entry| entry
        .path
        .ends_with("marketplace/.codebuddy-plugin/marketplace.json")));
    let mut previous_record = managed_record(&ctx, entries.clone());
    let installed = status::status(&ctx, Some(&previous_record)).unwrap();
    assert_eq!(installed.state, AgentState::Installed);
    assert!(installed.enabled);

    let changed_integrator = Integrator::with_paths(
        integrator.home(),
        integrator.executable(),
        integrator.paths().clone(),
        DEFAULT_BACKEND_PORT + 1,
    )
    .unwrap()
    .with_shared_root(integrator.shared_root())
    .unwrap();
    let changed_version = {
        let changed_ctx = AgentContext {
            integrator: &changed_integrator,
            detected: &detected,
        };
        plugin_files(&changed_ctx).unwrap().0
    };
    assert_ne!(changed_version, version);
    detected.env.insert(
        "OSSIA_TEST_WORKBUDDY_VERSION".into(),
        changed_version.clone(),
    );
    let changed_ctx = AgentContext {
        integrator: &changed_integrator,
        detected: &detected,
    };
    let synced = lifecycle::sync(&changed_ctx, &previous_record)
        .unwrap()
        .expect("enabled WorkBuddy plugin should refresh");
    entries = synced;
    previous_record = managed_record(&changed_ctx, entries.clone());
    let installed = status::status(&changed_ctx, Some(&previous_record)).unwrap();
    assert_eq!(installed.state, AgentState::Installed);
    let cache_path = installed.path.clone().unwrap();

    fs::write(
        detected.config_root.join("plugin-list.json"),
        serde_json::to_vec(&json!([
            {
                "id": PLUGIN_ID,
                "version": changed_version,
                "scope": "user",
                "enabled": false,
                "installPath": cache_path.clone()
            }
        ]))
        .unwrap(),
    )
    .unwrap();
    let disabled = status::status(&changed_ctx, Some(&previous_record)).unwrap();
    assert_eq!(disabled.state, AgentState::Disabled);
    assert_eq!(disabled.message, Some(AgentNotice::WorkbuddyPluginDisabled));

    fs::remove_file(detected.config_root.join("plugin-list.json")).unwrap();
    let missing = status::status(&changed_ctx, Some(&previous_record)).unwrap();
    assert_eq!(missing.state, AgentState::Broken);
    fs::write(
        detected.config_root.join("plugin-list.json"),
        serde_json::to_vec(&json!([
            {
                "id": PLUGIN_ID,
                "version": changed_version,
                "scope": "user",
                "enabled": true,
                "installPath": cache_path
            }
        ]))
        .unwrap(),
    )
    .unwrap();

    let removed = entries
        .iter()
        .map(|entry| entry.path.clone())
        .collect::<Vec<_>>();
    lifecycle::remove(&changed_ctx, &previous_record).unwrap();
    assert!(removed.iter().all(|path| !Path::new(path).exists()));
    let log = fs::read_to_string(log).unwrap();
    assert!(log.contains("plugin install ossia@ossia-workbuddy --scope user"));
    assert!(log.contains("plugin uninstall ossia@ossia-workbuddy --scope user -y"));
}

#[cfg(unix)]
fn managed_record(ctx: &AgentContext, entries: Vec<ManagedEntry>) -> ManagedRecord {
    ManagedRecord {
        management_id: format!("ossia:{}:desktop", ctx.detected.config_root.display()),
        agent_id: format!("workbuddy:{}", storage_key(&ctx.detected.config_root)),
        method: InstallMethod::Plugin,
        profile: "desktop".into(),
        template_version: TEMPLATE_VERSION,
        home: path_text(ctx.integrator.home()),
        executable: path_text(ctx.integrator.executable()),
        shared_root: path_text(ctx.integrator.shared_root()),
        updated_at: String::new(),
        entries,
    }
}
