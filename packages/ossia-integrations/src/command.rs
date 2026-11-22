use ossia_types::{path_text, AppError, ErrorCode};
use std::{
    collections::BTreeMap,
    ffi::OsStr,
    path::{Path, PathBuf},
    process::Command,
};

pub fn run(
    program: &Path,
    args: &[String],
    env: &BTreeMap<String, String>,
) -> Result<String, AppError> {
    run_with_prefix(program, &[], args, env)
}

pub fn run_with_prefix(
    program: &Path,
    prefix: &[String],
    args: &[String],
    env: &BTreeMap<String, String>,
) -> Result<String, AppError> {
    let args = prefix.iter().chain(args).cloned().collect::<Vec<_>>();
    let output = host_command(program, &args)
        .envs(env)
        .output()
        .map_err(|error| {
            AppError::new(
                ErrorCode::RuntimeUnavailable,
                format!("cannot execute {}: {error}", program.display()),
            )
        })?;
    if output.status.success() {
        return Ok(String::from_utf8_lossy(&output.stdout).into_owned());
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let detail: String = stderr
        .lines()
        .chain(stdout.lines())
        .rev()
        .take(8)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>()
        .join("\n");
    Err(AppError::new(
        ErrorCode::RuntimeUnavailable,
        format!(
            "{} exited with {}: {}",
            program.display(),
            output.status,
            if detail.is_empty() {
                "no diagnostic output"
            } else {
                detail.trim()
            }
        ),
    ))
}

fn host_command(program: &Path, args: &[String]) -> Command {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;

        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        let mut command = if program
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| {
                extension.eq_ignore_ascii_case("cmd") || extension.eq_ignore_ascii_case("bat")
            }) {
            let mut command =
                Command::new(std::env::var_os("COMSPEC").unwrap_or_else(|| "cmd.exe".into()));
            command
                .args(["/D", "/S", "/C"])
                .raw_arg(script_command_line(program, args));
            command
        } else {
            let mut command = Command::new(program);
            command.args(args);
            command
        };
        command.creation_flags(CREATE_NO_WINDOW);
        command
    }
    #[cfg(not(windows))]
    {
        let mut command = Command::new(program);
        command.args(args);
        command
    }
}

#[cfg(windows)]
fn script_command_line(program: &Path, args: &[String]) -> String {
    let mut line = quote_script_argument(program);
    for argument in args {
        line.push(' ');
        line.push_str(&quote_script_argument(Path::new(argument)));
    }
    format!("\"{line}\"")
}

#[cfg(windows)]
fn quote_script_argument(value: &Path) -> String {
    format!("\"{}\"", value.to_string_lossy().replace('"', "\"\""))
}

pub fn version(program: &Path, env: &BTreeMap<String, String>) -> Result<String, AppError> {
    version_with_prefix(program, &[], env)
}

pub fn version_with_prefix(
    program: &Path,
    prefix: &[String],
    env: &BTreeMap<String, String>,
) -> Result<String, AppError> {
    let text = run_with_prefix(program, prefix, &["--version".into()], env)?;
    let line = text
        .lines()
        .find(|line| !line.trim().is_empty())
        .ok_or_else(|| AppError::new(ErrorCode::InvalidInput, "host version output is empty"))?;
    Ok(line
        .split_whitespace()
        .last()
        .unwrap_or(line)
        .trim()
        .to_string())
}

pub fn find_program(name: &str, explicit: Option<&Path>) -> Option<PathBuf> {
    if let Some(path) = explicit {
        return executable(path).then(|| path.to_path_buf());
    }
    find_program_on_path(name, &std::env::var_os("PATH")?)
}

fn find_program_on_path(name: &str, path: &OsStr) -> Option<PathBuf> {
    for directory in std::env::split_paths(path) {
        for candidate in candidates(&directory, name) {
            if executable(&candidate) {
                return Some(candidate);
            }
        }
    }
    None
}

/// Find an explicit override, then a `PATH` entry, then known installation paths.
pub fn find_program_with_candidates(
    name: &str,
    explicit: Option<&Path>,
    candidates: impl IntoIterator<Item = PathBuf>,
) -> Option<PathBuf> {
    if let Some(path) = explicit {
        return executable(path).then(|| path.to_path_buf());
    }
    if let Some(path) = find_program(name, None) {
        return Some(path);
    }
    candidates.into_iter().find(|path| executable(path))
}

fn candidates(directory: &Path, name: &str) -> Vec<PathBuf> {
    #[cfg(windows)]
    {
        [".exe", ".com", ".cmd", ".bat"]
            .into_iter()
            .map(|extension| directory.join(format!("{name}{extension}")))
            .collect()
    }
    #[cfg(not(windows))]
    {
        vec![directory.join(name)]
    }
}

fn executable(path: &Path) -> bool {
    let Ok(metadata) = std::fs::metadata(path) else {
        return false;
    };
    if !metadata.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        metadata.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        true
    }
}

pub fn host_env(root_var: &str, root: &Path) -> BTreeMap<String, String> {
    BTreeMap::from([(root_var.to_string(), path_text(root))])
}

pub fn arguments(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_string()).collect()
}

#[cfg(test)]
#[path = "../tests/unit/command.rs"]
mod tests;
