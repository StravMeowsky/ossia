//! Replace a Linux installation through its package manager.

use super::super::{github::PackageKind, target, Error, Result};
use std::{path::Path, process::Command};

#[allow(dead_code)]
pub fn prepare(exe: &Path, kind: PackageKind) -> Result<super::Installation> {
    Ok(super::Installation::Linux {
        kind,
        executable: exe.to_path_buf(),
    })
}

pub fn install(staged: &Path, kind: PackageKind) -> Result<()> {
    match kind {
        PackageKind::Deb => run_package(PackageKind::Deb, "dpkg", staged),
        PackageKind::Rpm => run_package(PackageKind::Rpm, "rpm", staged),
        _ => Err(Error::other(format!(
            "a {kind:?} download cannot be installed on Linux"
        ))),
    }
}

pub fn relaunch(exe: &Path, home: &Path) -> Result<()> {
    let logs = home.join("logs");
    std::fs::create_dir_all(&logs)?;
    let log = logs.join("relaunch.log");
    let args = std::env::args_os().skip(1).collect::<Vec<_>>();
    let script = super::relaunch_script(exe, std::process::id(), &log, &args);
    Command::new("/bin/sh").arg("-c").arg(script).spawn()?;
    Ok(())
}

pub fn package_command(kind: PackageKind, program: &Path, staged: &Path) -> Result<Command> {
    let flag = match kind {
        PackageKind::Deb => "-i",
        PackageKind::Rpm => "-U",
        _ => {
            return Err(Error::other(format!(
                "a {kind:?} download is not a distribution package"
            )))
        }
    };
    let mut command = Command::new("pkexec");
    command.arg(program).arg(flag).arg(staged);
    Ok(command)
}

fn run_package(kind: PackageKind, tool: &str, staged: &Path) -> Result<()> {
    let program = target::on_path(tool)
        .ok_or_else(|| Error::other(format!("no {tool} on this machine to install this update")))?;
    let status = package_command(kind, &program, staged)?.status()?;
    if !status.success() {
        return Err(Error::other(format!(
            "{tool} exited with {status}; the previous version is still installed"
        )));
    }
    Ok(())
}
