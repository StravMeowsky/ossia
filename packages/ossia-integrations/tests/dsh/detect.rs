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
