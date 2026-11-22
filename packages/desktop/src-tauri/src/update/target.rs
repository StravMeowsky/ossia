//! Release asset selected for this machine.

use super::github::PackageKind;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Target {
    pub kind: PackageKind,
    pub arch: &'static str,
}

impl Target {
    pub fn asset_name(self) -> String {
        let platform = match self.kind {
            PackageKind::MacApp => "darwin",
            PackageKind::WindowsSetup => "windows",
            PackageKind::Deb | PackageKind::Rpm => "linux",
        };
        format!("ossia-{platform}-{}{}", self.arch, self.kind.suffix())
    }

    pub fn feed_slug(self) -> &'static str {
        match (self.kind, self.arch) {
            (PackageKind::MacApp, "arm64") => "darwin-arm64",
            (PackageKind::MacApp, "amd64") => "darwin-amd64",
            (PackageKind::WindowsSetup, "amd64") => "windows-amd64",
            (PackageKind::Deb, "amd64") => "linux-amd64-deb",
            (PackageKind::Rpm, "amd64") => "linux-amd64-rpm",
            _ => unreachable!("unsupported update target"),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PackageManagers {
    pub dpkg: bool,
    pub rpm: bool,
}

impl PackageManagers {
    pub fn detect() -> Self {
        Self {
            dpkg: on_path("dpkg").is_some(),
            rpm: on_path("rpm").is_some(),
        }
    }
}

pub fn current() -> Option<Target> {
    target_for(
        std::env::consts::OS,
        std::env::consts::ARCH,
        PackageManagers::detect(),
    )
}

pub fn target_for(os: &str, arch: &str, managers: PackageManagers) -> Option<Target> {
    match os {
        "macos" => Some(Target {
            kind: PackageKind::MacApp,
            arch: release_arch(arch)?,
        }),
        "windows" if arch == "x86_64" => Some(Target {
            kind: PackageKind::WindowsSetup,
            arch: "amd64",
        }),
        "linux" if arch == "x86_64" => {
            let kind = match (managers.dpkg, managers.rpm) {
                (true, _) => PackageKind::Deb,
                (false, true) => PackageKind::Rpm,
                (false, false) => return None,
            };
            Some(Target {
                kind,
                arch: "amd64",
            })
        }
        _ => None,
    }
}

fn release_arch(arch: &str) -> Option<&'static str> {
    match arch {
        "aarch64" => Some("arm64"),
        "x86_64" => Some("amd64"),
        _ => None,
    }
}

pub fn on_path(name: &str) -> Option<PathBuf> {
    std::env::var_os("PATH")
        .map(|path| std::env::split_paths(&path).collect::<Vec<_>>())
        .unwrap_or_default()
        .into_iter()
        .map(|dir| dir.join(name))
        .find(|candidate| candidate.is_file())
}
