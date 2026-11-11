use crate::error::{PootstrapError, Result};
use crate::isolation::PrivilegedRunner;
use crate::profile::device::Architecture;
use std::fs;
use std::path::{Path, PathBuf};

pub struct BinfmtChecker;

impl BinfmtChecker {
    pub fn is_binfmt_misc_mounted() -> bool {
        Path::new("/proc/sys/fs/binfmt_misc/status").exists()
    }

    pub fn find_qemu_static(arch: Architecture) -> Option<PathBuf> {
        let binary_name = arch.qemu_binary();
        which::which(binary_name).ok()
    }

    pub fn check_support(arch: Architecture) -> Result<()> {
        if arch == Architecture::X86_64 {
            return Ok(());
        }

        if !Self::is_binfmt_misc_mounted() {
            tracing::warn!("/proc/sys/fs/binfmt_misc is not mounted. Host might not support transparent binfmt emulation.");
        }

        let qemu_bin = Self::find_qemu_static(arch).ok_or_else(|| {
            PootstrapError::MissingTool {
                tool: format!("{} (qemu-user-static)", arch.qemu_binary()),
            }
        })?;

        tracing::info!("Found emulation binary for {}: {:?}", arch, qemu_bin);
        Ok(())
    }

    pub fn setup_qemu_in_rootfs(arch: Architecture, rootfs: &Path) -> Result<()> {
        if arch == Architecture::X86_64 {
            return Ok(());
        }

        let host_qemu = Self::find_qemu_static(arch).ok_or_else(|| {
            PootstrapError::MissingTool {
                tool: format!("{} (qemu-user-static)", arch.qemu_binary()),
            }
        })?;

        let target_dir = rootfs.join("usr").join("bin");
        if fs::create_dir_all(&target_dir).is_err() {
            PrivilegedRunner::create_dir_all(&target_dir)?;
        }

        let target_qemu = target_dir.join(arch.qemu_binary());
        if fs::copy(&host_qemu, &target_qemu).is_err() {
            PrivilegedRunner::copy(&host_qemu, &target_qemu)?;
        }
        tracing::debug!("Installed {:?} into rootfs {:?}", host_qemu, target_qemu);
        Ok(())
    }
}
