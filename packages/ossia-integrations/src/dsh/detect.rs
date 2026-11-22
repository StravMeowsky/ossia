use super::*;

pub(super) fn detect(paths: &HostPaths) -> Option<DetectedAgent> {
    let executable = scan::find("dsh", paths.dsh_bin.as_deref(), candidates())?;
    let env = command::host_env("DSH_HOME", &paths.dsh_home);
    let version = command::version(&executable, &env).ok()?;
    command::run(
        &executable,
        &command::arguments(&["plugin", "--help"]),
        &env,
    )
    .ok()?;
    Some(DetectedAgent {
        command: executable.clone(),
        command_args: Vec::new(),
        executable,
        version,
        config_root: paths.dsh_home.clone(),
        profile: "desktop",
        env,
    })
}

fn candidates() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    #[cfg(target_os = "macos")]
    {
        paths.extend(scan::mac_app(
            "DeepSeek Harness.app/Contents/Resources/runtime/cli/bin/dsh",
        ));
    }
    paths.extend(scan::common_bin("dsh"));
    #[cfg(windows)]
    {
        for root in scan::program_files("DeepSeek Harness") {
            paths.extend(installation_candidates(root));
        }
    }
    paths
}

#[cfg(windows)]
fn installation_candidates(root: PathBuf) -> Vec<PathBuf> {
    vec![
        root.join("resources/runtime/cli/bin/dsh.exe"),
        root.join("resources/runtime/cli/bin/dsh.cmd"),
        root.join("runtime/cli/bin/dsh.exe"),
        root.join("runtime/cli/bin/dsh.cmd"),
        root.join("dsh.exe"),
    ]
}

#[cfg(test)]
mod tests {
    #[cfg(windows)]
    use super::*;
    #[cfg(windows)]
    use std::collections::BTreeMap;

    #[cfg(windows)]
    #[test]
    fn windows_installation_candidates_include_the_packaged_cli_shim() {
        let root = PathBuf::from(r"C:\Users\person\AppData\Local\Programs\DeepSeek Harness");

        assert!(installation_candidates(root.clone())
            .contains(&root.join("resources/runtime/cli/bin/dsh.cmd")));
    }

    #[cfg(windows)]
    #[test]
    fn detection_runs_a_cmd_launcher_from_a_path_with_spaces() {
        let root = tempfile::tempdir().unwrap();
        let directory = root
            .path()
            .join("DeepSeek Harness/resources/runtime/cli/bin");
        std::fs::create_dir_all(&directory).unwrap();
        let script = directory.join("dsh.cmd");
        std::fs::write(
            &script,
            [
                "@echo off",
                "if \"%~1\"==\"--version\" (",
                "  echo dsh 0.2.2-test",
                "  exit /b 0",
                ")",
                "exit /b 0",
                "",
            ]
            .join("\r\n"),
        )
        .unwrap();
        let paths = HostPaths {
            codex_home: root.path().join("codex"),
            dsh_home: root.path().join("dsh"),
            workbuddy_home: root.path().join("home"),
            workbuddy_apps: Vec::new(),
            codex_bin: None,
            dsh_bin: Some(script.clone()),
            launch_env: BTreeMap::new(),
        };

        let detected = detect(&paths).expect("DSH should be detected");

        assert_eq!(detected.executable, script);
        assert_eq!(detected.command, script);
        assert!(detected.command_args.is_empty());
        assert_eq!(detected.version, "0.2.2-test");
    }
}
