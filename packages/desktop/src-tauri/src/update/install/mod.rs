//! Installing a verified update.

#[cfg(target_os = "linux")]
use super::target;
#[cfg(not(target_os = "macos"))]
use super::Error;
use super::Result;
use std::path::Path;

pub mod linux;
#[cfg(target_os = "macos")]
pub mod macos;
pub mod windows;

#[allow(dead_code)]
#[derive(Debug)]
pub enum Installation {
    Mac {
        bundle: std::path::PathBuf,
        executable: std::path::PathBuf,
    },
    Linux {
        kind: super::github::PackageKind,
        executable: std::path::PathBuf,
    },
    Windows,
}

pub fn prepare() -> Result<Installation> {
    let exe = std::env::current_exe()?;
    #[cfg(target_os = "macos")]
    {
        macos::prepare(&exe)
    }
    #[cfg(target_os = "linux")]
    {
        let target =
            target::current().ok_or_else(|| Error::other("this install cannot update itself"))?;
        linux::prepare(&exe, target.kind)
    }
    #[cfg(target_os = "windows")]
    {
        let _ = exe;
        Ok(Installation::Windows)
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        let _ = exe;
        Err(Error::other(
            "in-place update is not implemented on this platform",
        ))
    }
}

impl Installation {
    pub fn apply(&self, staged: &Path, restart: bool) -> Result<()> {
        match self {
            #[cfg(target_os = "macos")]
            Self::Mac { bundle, .. } => macos::install(staged, bundle),
            Self::Linux { kind, .. } => linux::install(staged, *kind),
            Self::Windows => windows::install(staged, restart),
            #[cfg(not(target_os = "macos"))]
            Self::Mac { .. } => Err(Error::other("macOS installation requires macOS")),
        }
    }

    pub fn relaunch(&self, home: &Path) -> Result<()> {
        match self {
            Self::Mac { executable, .. } | Self::Linux { executable, .. } => {
                linux::relaunch(executable, home)
            }
            Self::Windows => Ok(()),
        }
    }
}

pub fn relaunch_script(exe: &Path, pid: u32, log: &Path, args: &[std::ffi::OsString]) -> String {
    let arguments = args
        .iter()
        .map(|arg| sh_quote(&arg.to_string_lossy()))
        .collect::<Vec<_>>()
        .join(" ");
    let arguments = if arguments.is_empty() {
        String::new()
    } else {
        format!(" {arguments}")
    };
    format!(
        "while kill -0 {pid} 2>/dev/null; do case \"$(ps -p {pid} -o stat=)\" in *Z*) break;; esac; sleep 0.2; done; exec {}{arguments} >>{} 2>&1",
        sh_quote(&exe.display().to_string()),
        sh_quote(&log.display().to_string())
    )
}

pub fn sh_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', r"'\''"))
}

pub fn windows_quote(value: &str) -> String {
    if !value.is_empty()
        && !value
            .bytes()
            .any(|byte| matches!(byte, b' ' | b'\t' | b'"'))
    {
        return value.to_string();
    }
    let mut quoted = String::from("\"");
    let mut slashes = 0;
    for character in value.chars() {
        if character == '\\' {
            slashes += 1;
        } else if character == '"' {
            quoted.push_str(&"\\".repeat(slashes * 2 + 1));
            quoted.push('"');
            slashes = 0;
        } else {
            quoted.push_str(&"\\".repeat(slashes));
            quoted.push(character);
            slashes = 0;
        }
    }
    quoted.push_str(&"\\".repeat(slashes * 2));
    quoted.push('"');
    quoted
}
