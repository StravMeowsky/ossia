use super::*;

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
