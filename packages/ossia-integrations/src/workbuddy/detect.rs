use super::*;
use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub(super) struct AppInstance {
    pub(super) id: String,
    pub(super) app_executable: PathBuf,
    pub(super) cli: PathBuf,
    pub(super) config_root: PathBuf,
    pub(super) app_version: String,
}

pub(super) fn app_candidates(home: &Path) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Some(value) = std::env::var_os("OSSIA_WORKBUDDY_APP") {
        paths.extend(
            std::env::split_paths(&value)
                .filter(|path| path.is_absolute())
                .collect::<Vec<_>>(),
        );
    }
    #[cfg(target_os = "macos")]
    {
        for root in [PathBuf::from("/Applications"), home.join("Applications")] {
            let Ok(entries) = fs::read_dir(root) else {
                continue;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.extension().is_some_and(|extension| extension == "app") {
                    continue;
                }
                if path
                    .file_stem()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.starts_with("WorkBuddy"))
                {
                    paths.push(path);
                }
            }
        }
    }
    #[cfg(windows)]
    {
        let mut roots = vec![
            home.join("AppData/Local/Programs"),
            home.join("AppData/Local"),
        ];
        for key in [
            "LOCALAPPDATA",
            "PROGRAMFILES",
            "PROGRAMW6432",
            "PROGRAMFILES(X86)",
        ] {
            if let Some(path) = std::env::var_os(key) {
                roots.push(PathBuf::from(path));
            }
        }
        roots.push(home.join("AppData/Local/Programs"));
        for root in roots {
            let Ok(entries) = fs::read_dir(&root) else {
                continue;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir()
                    && path
                        .file_name()
                        .and_then(|name| name.to_str())
                        .is_some_and(|name| name.starts_with("WorkBuddy"))
                {
                    paths.push(path);
                }
            }
        }
    }
    let mut unique = BTreeMap::new();
    for path in paths {
        let key = normalize_path(&path_text(&path));
        unique.entry(key).or_insert(path);
    }
    unique.into_values().collect()
}

pub(super) fn instances(paths: &HostPaths) -> Vec<AppInstance> {
    let mut instances: BTreeMap<String, AppInstance> = BTreeMap::new();
    for app in &paths.workbuddy_apps {
        let Some(instance) = discover(app, &paths.workbuddy_home) else {
            continue;
        };
        let key = normalize_path(&path_text(&instance.config_root));
        match instances.get(&key) {
            Some(existing)
                if version_key(&existing.app_version) >= version_key(&instance.app_version) => {}
            _ => {
                instances.insert(key, instance);
            }
        }
    }
    instances.into_values().collect()
}

fn discover(app: &Path, home: &Path) -> Option<AppInstance> {
    let cli = cli_path(app)?;
    let product = read_product_json(app)?;
    let data_folder = product.get("dataFolderName")?.as_str()?;
    if !valid_data_folder(data_folder) {
        return None;
    }
    let app_version = product
        .get("genieVersion")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("unknown")
        .to_string();
    let config_root = home.join(data_folder);
    let mut id_hash =
        ContentHash::of(normalize_path(&path_text(&config_root)).as_bytes()).to_string();
    id_hash.truncate(12);
    Some(AppInstance {
        id: format!("workbuddy:{id_hash}"),
        app_executable: app_executable(app)?,
        cli,
        config_root,
        app_version,
    })
}

fn valid_data_folder(value: &str) -> bool {
    !value.is_empty()
        && Path::new(value).components().count() == 1
        && !value.contains('/')
        && !value.contains('\\')
}

fn read_product_json(app: &Path) -> Option<serde_json::Value> {
    let path = product_path(app)?;
    serde_json::from_slice(&fs::read(path).ok()?).ok()
}

fn product_path(app: &Path) -> Option<PathBuf> {
    let relative = Path::new("app.asar.unpacked/cli/product.json");
    #[cfg(target_os = "macos")]
    {
        let path = app.join("Contents/Resources").join(relative);
        if path.exists() {
            return Some(path);
        }
    }
    let path = app.join("resources").join(relative);
    path.exists().then_some(path)
}

fn cli_path(app: &Path) -> Option<PathBuf> {
    product_path(app)?
        .parent()
        .map(|path| path.join("bin/codebuddy"))
        .filter(|path| path.is_file())
}

fn app_executable(app: &Path) -> Option<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        let electron = app.join("Contents/MacOS/Electron");
        if electron.is_file() {
            return Some(electron);
        }
        let directory = app.join("Contents/MacOS");
        fs::read_dir(directory)
            .ok()?
            .flatten()
            .map(|entry| entry.path())
            .find(|path| path.is_file())
    }
    #[cfg(windows)]
    {
        for name in ["WorkBuddyAI.exe", "WorkBuddy AI.exe", "WorkBuddy.exe"] {
            let path = app.join(name);
            if path.is_file() {
                return Some(path);
            }
        }
        fs::read_dir(app)
            .ok()?
            .flatten()
            .map(|entry| entry.path())
            .find(|path| {
                path.is_file()
                    && path
                        .extension()
                        .is_some_and(|extension| extension.eq_ignore_ascii_case("exe"))
                    && path
                        .file_stem()
                        .and_then(|name| name.to_str())
                        .is_some_and(|name| name.starts_with("WorkBuddy"))
            })
    }
    #[cfg(not(any(target_os = "macos", windows)))]
    {
        let _ = app;
        None
    }
}

pub(super) fn detect(instance: &AppInstance) -> Option<DetectedAgent> {
    let (command, command_args, mut env) = launcher(instance)?;
    env.insert("CODEBUDDY_FORCE_LITE_WB_BUNDLE".into(), "1".into());
    env.insert(
        "WORKBUDDY_CONFIG_DIR".into(),
        path_text(&instance.config_root),
    );
    env.insert(
        "CODEBUDDY_CONFIG_DIR".into(),
        path_text(&instance.config_root),
    );
    let cli_version = command::version_with_prefix(&command, &command_args, &env).ok()?;
    command::run_with_prefix(
        &command,
        &command_args,
        &command::arguments(&["plugin", "--help"]),
        &env,
    )
    .ok()?;
    let version = if instance.app_version == "unknown" {
        format!("CLI {cli_version}")
    } else {
        format!("{} (CLI {cli_version})", instance.app_version)
    };
    Some(DetectedAgent {
        executable: instance.cli.clone(),
        command,
        command_args,
        version,
        config_root: instance.config_root.clone(),
        profile: "desktop",
        env,
    })
}

fn launcher(instance: &AppInstance) -> Option<(PathBuf, Vec<String>, BTreeMap<String, String>)> {
    if let Some(node) = node_executable(&instance.config_root) {
        return Some((node, vec![path_text(&instance.cli)], BTreeMap::new()));
    }
    if instance.app_executable.is_file() {
        return Some((
            instance.app_executable.clone(),
            vec![path_text(&instance.cli)],
            BTreeMap::from([("ELECTRON_RUN_AS_NODE".into(), "1".into())]),
        ));
    }
    None
}

fn node_executable(config_root: &Path) -> Option<PathBuf> {
    let root = config_root.join("binaries/node/versions");
    let mut candidates = fs::read_dir(root)
        .ok()?
        .flatten()
        .filter_map(|entry| {
            let directory = entry.path();
            let version = version_key(&entry.file_name().to_string_lossy());
            for path in [directory.join("bin/node"), directory.join("node")] {
                if path.is_file() {
                    return Some((version, path));
                }
            }
            #[cfg(windows)]
            {
                for path in [directory.join("bin/node.exe"), directory.join("node.exe")] {
                    if path.is_file() {
                        return Some((version, path));
                    }
                }
            }
            None
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| right.0.cmp(&left.0));
    candidates.into_iter().map(|(_, path)| path).next()
}

fn version_key(value: &str) -> Vec<u64> {
    value
        .split(|character: char| !character.is_ascii_digit())
        .filter(|part| !part.is_empty())
        .filter_map(|part| part.parse().ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_folder_must_be_one_component() {
        assert!(valid_data_folder(".workbuddy"));
        assert!(valid_data_folder(".workbuddy-ai"));
        assert!(!valid_data_folder("../escape"));
        assert!(!valid_data_folder("a/b"));
        assert!(!valid_data_folder(""));
    }

    #[test]
    fn version_key_orders_numeric_segments() {
        assert!(version_key("22.22.2") > version_key("22.9.9"));
        assert!(version_key("22.22.2-6") > version_key("22.22.1"));
    }

    #[test]
    fn node_executable_uses_the_newest_staged_runtime() {
        let root = tempfile::tempdir().unwrap();
        let older = root.path().join("binaries/node/versions/22.9.0/bin");
        let newer = root.path().join("binaries/node/versions/22.10.0/bin");
        fs::create_dir_all(&older).unwrap();
        fs::create_dir_all(&newer).unwrap();
        fs::write(older.join("node"), b"older").unwrap();
        fs::write(newer.join("node"), b"newer").unwrap();
        assert_eq!(node_executable(root.path()).unwrap(), newer.join("node"));
    }

    #[cfg(windows)]
    #[test]
    fn node_executable_accepts_the_windows_flat_layout() {
        let root = tempfile::tempdir().unwrap();
        let version = root.path().join("binaries/node/versions/22.22.2-6");
        fs::create_dir_all(&version).unwrap();
        fs::write(version.join("node.exe"), b"node").unwrap();
        assert_eq!(
            node_executable(root.path()).unwrap(),
            version.join("node.exe")
        );
    }
}
