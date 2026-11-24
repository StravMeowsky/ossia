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
#[path = "../../tests/dsh/detect.rs"]
mod tests;
