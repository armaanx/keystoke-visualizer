use anyhow::{Context, Result};
use std::path::Path;
use std::process::{Command, Stdio};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;
#[cfg(windows)]
const DETACHED_PROCESS: u32 = 0x00000008;
#[cfg(windows)]
const CREATE_NEW_PROCESS_GROUP: u32 = 0x00000200;

pub fn spawn_daemon(executable: &Path, session_id: &str, control_token: &str) -> Result<u32> {
    let mut command = Command::new(executable);
    command
        .arg("daemon")
        .arg("--session-id")
        .arg(session_id)
        .arg("--control-token")
        .arg(control_token)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    #[cfg(windows)]
    command.creation_flags(CREATE_NO_WINDOW | DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP);

    let child = command.spawn().context("spawning detached collector")?;
    Ok(child.id())
}

pub fn process_exists(pid: u32) -> bool {
    #[cfg(windows)]
    {
        Command::new("powershell")
            .args(["-NoProfile", "-Command", &format!("Get-Process -Id {pid}")])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|status| status.success())
            .unwrap_or(false)
    }

    #[cfg(not(windows))]
    {
        Command::new("kill")
            .args(["-0", &pid.to_string()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|status| status.success())
            .unwrap_or(false)
    }
}

pub fn open_path(path: &Path) -> Result<()> {
    #[cfg(windows)]
    {
        Command::new("cmd")
            .args(["/C", "start", "", &path.to_string_lossy()])
            .spawn()
            .context("opening report in browser")?;
    }

    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg(path)
            .spawn()
            .context("opening report in browser")?;
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        Command::new("xdg-open")
            .arg(path)
            .spawn()
            .context("opening report in browser")?;
    }

    Ok(())
}
