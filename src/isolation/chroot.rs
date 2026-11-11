use crate::error::{PootstrapError, Result};
use crate::isolation::PrivilegedRunner;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Stdio;

pub struct MountGuard {
    mounted_paths: Vec<PathBuf>,
}

impl MountGuard {
    pub fn new() -> Self {
        Self {
            mounted_paths: Vec::new(),
        }
    }

    pub fn mount_pseudo_filesystems(rootfs: &Path) -> Result<Self> {
        let mut guard = Self::new();

        let mounts: Vec<(&str, &str, &str, Vec<&str>)> = vec![
            ("/dev", "dev", "none", vec!["--bind"]),
            ("/dev/pts", "dev/pts", "none", vec!["--bind"]),
            ("/dev/shm", "dev/shm", "none", vec!["--bind"]),
            ("proc", "proc", "proc", vec![]),
            ("sysfs", "sys", "sysfs", vec![]),
        ];

        for (source, relative_target, fstype, extra_flags) in mounts {
            let target_path = rootfs.join(relative_target);
            if fs::create_dir_all(&target_path).is_err() {
                PrivilegedRunner::create_dir_all(&target_path)?;
            }

            let mut cmd = PrivilegedRunner::command("mount");
            for flag in extra_flags {
                cmd.arg(flag);
            }
            if fstype != "none" {
                cmd.arg("-t").arg(fstype);
            }
            cmd.arg(source).arg(&target_path);

            let status = cmd.status().map_err(PootstrapError::Io)?;
            if !status.success() {
                return Err(PootstrapError::MountError {
                    path: target_path,
                    reason: format!("mount exited with code {:?}", status.code()),
                });
            }

            guard.mounted_paths.push(target_path);
        }

        Ok(guard)
    }

    pub fn unmount_all(&mut self) -> Result<()> {
        let mut last_error = None;
        while let Some(path) = self.mounted_paths.pop() {
            let status = PrivilegedRunner::command("umount")
                .arg("-l")
                .arg(&path)
                .status();

            match status {
                Ok(s) if s.success() => {
                    tracing::debug!("Successfully unmounted {:?}", path);
                }
                Ok(s) => {
                    tracing::warn!("Failed to unmount {:?}, exit code: {:?}", path, s.code());
                    last_error = Some(PootstrapError::UnmountError {
                        path: path.clone(),
                        reason: format!("umount exited with code {:?}", s.code()),
                    });
                }
                Err(err) => {
                    tracing::warn!("Error executing umount on {:?}: {}", path, err);
                    last_error = Some(PootstrapError::Io(err));
                }
            }
        }

        if let Some(err) = last_error {
            Err(err)
        } else {
            Ok(())
        }
    }
}

impl Drop for MountGuard {
    fn drop(&mut self) {
        if !self.mounted_paths.is_empty() {
            tracing::debug!("MountGuard cleaning up remaining mounts on drop...");
            let _ = self.unmount_all();
        }
    }
}

pub struct ChrootRunner;

impl ChrootRunner {
    pub fn setup_resolv_conf(rootfs: &Path) -> Result<()> {
        let etc_dir = rootfs.join("etc");
        if fs::create_dir_all(&etc_dir).is_err() {
            PrivilegedRunner::create_dir_all(&etc_dir)?;
        }
        let target = etc_dir.join("resolv.conf");

        // Copy host resolv.conf if available, or write standard nameservers
        if Path::new("/etc/resolv.conf").exists() {
            if fs::copy("/etc/resolv.conf", &target).is_err() {
                PrivilegedRunner::copy(Path::new("/etc/resolv.conf"), &target)?;
            }
        } else {
            PrivilegedRunner::write_file(&target, "nameserver 1.1.1.1\nnameserver 8.8.8.8\n")?;
        }
        Ok(())
    }

    pub fn run_in_chroot(rootfs: &Path, command: &[&str]) -> Result<()> {
        if command.is_empty() {
            return Ok(());
        }

        let mut cmd = PrivilegedRunner::command("chroot");
        cmd.arg(rootfs);
        for arg in command {
            cmd.arg(arg);
        }

        tracing::info!("Running chroot command: {:?}", command);
        let status = cmd.status().map_err(PootstrapError::Io)?;

        if !status.success() {
            return Err(PootstrapError::CommandFailed {
                command: command.join(" "),
                exit_code: status.code(),
                stderr: "Chroot command exited with error status".into(),
            });
        }

        Ok(())
    }

    pub fn run_interactive_shell(rootfs: &Path) -> Result<()> {
        Self::setup_resolv_conf(rootfs)?;
        let _guard = MountGuard::mount_pseudo_filesystems(rootfs)?;

        let mut cmd = PrivilegedRunner::command("chroot");
        cmd.arg(rootfs).arg("/bin/bash");
        cmd.stdin(Stdio::inherit());
        cmd.stdout(Stdio::inherit());
        cmd.stderr(Stdio::inherit());

        tracing::info!("Entering chroot interactive shell...");
        let status = cmd.status().map_err(PootstrapError::Io)?;

        if !status.success() {
            tracing::warn!("Chroot shell exited with code {:?}", status.code());
        }

        Ok(())
    }
}
