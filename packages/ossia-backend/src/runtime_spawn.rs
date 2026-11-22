use crate::{storage, Result};
#[cfg(windows)]
use std::ffi::OsStr;
use std::{ffi::OsString, path::Path};

/// A spawned backend process that can be polled without inheriting CLI output handles.
pub(crate) struct RuntimeChild {
    #[cfg(unix)]
    child: tokio::process::Child,
    #[cfg(windows)]
    handle: usize,
}

impl RuntimeChild {
    pub(crate) fn try_wait(&mut self) -> Result<Option<i32>> {
        #[cfg(unix)]
        {
            Ok(self
                .child
                .try_wait()
                .map_err(storage)?
                .map(|status| status.code().unwrap_or(-1)))
        }
        #[cfg(windows)]
        {
            use windows_sys::Win32::{
                Foundation::STILL_ACTIVE, System::Threading::GetExitCodeProcess,
            };
            let mut code = 0u32;
            let success = unsafe { GetExitCodeProcess(self.handle as _, &mut code) } != 0;
            if !success {
                return Err(storage(std::io::Error::last_os_error()));
            }
            return Ok((code != STILL_ACTIVE as u32).then_some(code as i32));
        }
    }

    pub(crate) fn detach(self) {
        #[cfg(unix)]
        tokio::spawn(async move {
            let mut child = self.child;
            let _ = child.wait().await;
        });
        #[cfg(windows)]
        drop(self);
    }
}

#[cfg(windows)]
impl Drop for RuntimeChild {
    fn drop(&mut self) {
        use windows_sys::Win32::Foundation::CloseHandle;
        unsafe {
            CloseHandle(self.handle as _);
        }
    }
}

pub(crate) fn spawn(executable: &Path, args: &[OsString]) -> Result<RuntimeChild> {
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        let mut command = tokio::process::Command::new(executable);
        command
            .args(args)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
        command.as_std_mut().process_group(0);
        Ok(RuntimeChild {
            child: command.spawn().map_err(storage)?,
        })
    }
    #[cfg(windows)]
    {
        use windows_sys::Win32::{
            Foundation::CloseHandle,
            System::Threading::{
                CreateProcessW, CREATE_NEW_PROCESS_GROUP, CREATE_NO_WINDOW, DETACHED_PROCESS,
                PROCESS_INFORMATION, STARTUPINFOW,
            },
        };

        let application = wide(executable.as_os_str());
        let mut command_line = command_line(executable, args);
        let mut startup = STARTUPINFOW::default();
        startup.cb = std::mem::size_of::<STARTUPINFOW>() as u32;
        let mut process = PROCESS_INFORMATION::default();
        let created = unsafe {
            CreateProcessW(
                application.as_ptr(),
                command_line.as_mut_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                0,
                DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP | CREATE_NO_WINDOW,
                std::ptr::null(),
                std::ptr::null(),
                &startup,
                &mut process,
            )
        };
        if created == 0 {
            return Err(storage(std::io::Error::last_os_error()));
        }
        unsafe {
            CloseHandle(process.hThread);
        }
        return Ok(RuntimeChild {
            handle: process.hProcess as usize,
        });
    }
}

#[cfg(windows)]
fn wide(value: &OsStr) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    value.encode_wide().chain(Some(0)).collect()
}

#[cfg(windows)]
fn command_line(executable: &Path, args: &[OsString]) -> Vec<u16> {
    let mut line = String::new();
    quote(&mut line, executable.as_os_str());
    for arg in args {
        line.push(' ');
        quote(&mut line, arg);
    }
    wide(OsStr::new(&line))
}

#[cfg(windows)]
fn quote(line: &mut String, arg: &OsStr) {
    let arg = arg.to_string_lossy();
    let quoted = arg.is_empty()
        || arg
            .chars()
            .any(|character| matches!(character, ' ' | '\t' | '"'));
    if !quoted {
        line.push_str(&arg);
        return;
    }
    line.push('"');
    let mut slashes = 0usize;
    for character in arg.chars() {
        match character {
            '\\' => slashes += 1,
            '"' => {
                line.push_str(&"\\".repeat(slashes * 2 + 1));
                line.push('"');
                slashes = 0;
            }
            _ => {
                line.push_str(&"\\".repeat(slashes));
                slashes = 0;
                line.push(character);
            }
        }
    }
    line.push_str(&"\\".repeat(slashes * 2));
    line.push('"');
}
