use crate::error::{PootstrapError, Result};
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

pub struct PrivilegedRunner;

impl PrivilegedRunner {
    pub fn is_root() -> bool {
        nix::unistd::getuid().is_root()
    }

    pub fn command(program: &str) -> Command {
        if Self::is_root() {
            Command::new(program)
        } else {
            let mut cmd = Command::new("sudo");
            cmd.arg(program);
            cmd
        }
    }

    pub fn write_file(path: &Path, content: &str) -> Result<()> {
        if let Some(parent) = path.parent() {
            let _ = Self::create_dir_all(parent);
        }

        // Try unprivileged write first
        if std::fs::write(path, content).is_ok() {
            return Ok(());
        }

        // Elevate with sudo only if permission denied
        let mut child = Command::new("sudo")
            .arg("tee")
            .arg(path)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .spawn()
            .map_err(PootstrapError::Io)?;

        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(content.as_bytes())?;
        }

        let status = child.wait().map_err(PootstrapError::Io)?;
        if !status.success() {
            return Err(PootstrapError::RootfsBuild(format!(
                "Failed to write to privileged file {:?}",
                path
            )));
        }

        Ok(())
    }

    pub fn create_dir_all(path: &Path) -> Result<()> {
        // Try unprivileged mkdir first
        if std::fs::create_dir_all(path).is_ok() {
            return Ok(());
        }

        // Elevate with sudo only if permission denied
        let status = Command::new("sudo")
            .arg("mkdir")
            .arg("-p")
            .arg(path)
            .status()
            .map_err(PootstrapError::Io)?;

        if !status.success() {
            return Err(PootstrapError::Io(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                format!("Failed to create directory {:?}", path),
            )));
        }

        Ok(())
    }

    pub fn copy(from: &Path, to: &Path) -> Result<()> {
        if let Some(parent) = to.parent() {
            let _ = Self::create_dir_all(parent);
        }

        // Try unprivileged copy first
        if std::fs::copy(from, to).is_ok() {
            return Ok(());
        }

        // Elevate with sudo only if permission denied
        let status = Command::new("sudo")
            .arg("cp")
            .arg("-a")
            .arg(from)
            .arg(to)
            .status()
            .map_err(PootstrapError::Io)?;

        if !status.success() {
            return Err(PootstrapError::Io(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                format!("Failed to copy from {:?} to {:?}", from, to),
            )));
        }

        Ok(())
    }
}
