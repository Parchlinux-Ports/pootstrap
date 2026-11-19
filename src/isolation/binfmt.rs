use crate::error::{PootstrapError, Result};
use crate::isolation::PrivilegedRunner;
use crate::profile::device::Architecture;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct BinfmtChecker;

impl BinfmtChecker {
    pub fn is_binfmt_misc_mounted() -> bool {
        Path::new("/proc/sys/fs/binfmt_misc/status").exists()
    }

    pub fn find_qemu_static(arch: Architecture) -> Option<PathBuf> {
        let binary_name = arch.qemu_binary();
        if let Ok(p) = which::which(binary_name) {
            return Some(p);
        }

        let candidates = [
            format!("/usr/bin/{}", binary_name),
            format!("/usr/local/bin/{}", binary_name),
            format!("/bin/{}", binary_name),
        ];

        for c in candidates {
            let p = PathBuf::from(c);
            if p.exists() {
                return Some(p);
            }
        }

        if let Ok(home) = std::env::var("HOME") {
            let user_bin = PathBuf::from(home)
                .join(".cache")
                .join("pootstrap")
                .join("bin")
                .join(binary_name);
            if user_bin.exists() {
                return Some(user_bin);
            }
        }

        None
    }

    pub fn ensure_qemu_static(arch: Architecture, cache_dir: &Path) -> Result<PathBuf> {
        if let Some(p) = Self::find_qemu_static(arch) {
            return Ok(p);
        }

        let binary_name = arch.qemu_binary();
        let target_dir = cache_dir.join("bin");
        if fs::create_dir_all(&target_dir).is_err() {
            PrivilegedRunner::create_dir_all(&target_dir)?;
        }
        let target_bin = target_dir.join(binary_name);

        if target_bin.exists() {
            return Ok(target_bin);
        }

        let url = match arch {
            Architecture::Aarch64 => {
                "https://github.com/multiarch/qemu-user-static/releases/download/v7.2.0-1/qemu-aarch64-static"
            }
            Architecture::Armv7h => {
                "https://github.com/multiarch/qemu-user-static/releases/download/v7.2.0-1/qemu-arm-static"
            }
            Architecture::X86_64 => return Ok(PathBuf::from("/bin/sh")),
        };

        tracing::info!("Downloading static user-mode QEMU for {} from {}...", arch, url);
        let status = Command::new("curl")
            .arg("-L")
            .arg("-f")
            .arg("--progress-bar")
            .arg("-o")
            .arg(&target_bin)
            .arg(url)
            .status()
            .map_err(PootstrapError::Io)?;

        if !status.success() {
            return Err(PootstrapError::MissingTool {
                tool: format!("Failed to download {} from {}", binary_name, url),
            });
        }

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(&target_bin, fs::Permissions::from_mode(0o755));
        }

        Ok(target_bin)
    }

    pub fn register_binfmt(arch: Architecture, qemu_path: &Path) -> Result<()> {
        if arch == Architecture::X86_64 {
            return Ok(());
        }

        let rule_name = match arch {
            Architecture::Aarch64 => "qemu-aarch64",
            Architecture::Armv7h => "qemu-arm",
            Architecture::X86_64 => return Ok(()),
        };

        let entry_path = PathBuf::from(format!("/proc/sys/fs/binfmt_misc/{}", rule_name));
        if entry_path.exists() {
            tracing::debug!("binfmt rule for {} already registered", rule_name);
            return Ok(());
        }

        let reg_file = Path::new("/proc/sys/fs/binfmt_misc/register");
        if !reg_file.exists() {
            let _ = PrivilegedRunner::command("mount")
                .arg("-t")
                .arg("binfmt_misc")
                .arg("binfmt_misc")
                .arg("/proc/sys/fs/binfmt_misc")
                .status();
        }

        let reg_str = match arch {
            Architecture::Aarch64 => format!(
                ":qemu-aarch64:M::\\x7fELF\\x02\\x01\\x01\\x00\\x00\\x00\\x00\\x00\\x00\\x00\\x00\\x00\\x02\\x00\\xb7\\x00:\\xff\\xff\\xff\\xff\\xff\\xff\\xff\\x00\\xff\\xff\\xff\\xff\\xff\\xff\\xff\\xff\\xfe\\xff\\xff\\xff:{}:POCF",
                qemu_path.display()
            ),
            Architecture::Armv7h => format!(
                ":qemu-arm:M::\\x7fELF\\x01\\x01\\x01\\x00\\x00\\x00\\x00\\x00\\x00\\x00\\x00\\x00\\x02\\x00\\x28\\x00:\\xff\\xff\\xff\\xff\\xff\\xff\\xff\\x00\\xff\\xff\\xff\\xff\\xff\\xff\\xff\\xff\\xfe\\xff\\xff\\xff:{}:POCF",
                qemu_path.display()
            ),
            Architecture::X86_64 => return Ok(()),
        };

        let _ = PrivilegedRunner::write_file(reg_file, &reg_str);
        Ok(())
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

    pub fn setup_qemu_in_rootfs(
        arch: Architecture,
        rootfs: &Path,
        cache_dir: Option<&Path>,
    ) -> Result<()> {
        if arch == Architecture::X86_64 {
            return Ok(());
        }

        let host_qemu = if let Some(p) = Self::find_qemu_static(arch) {
            p
        } else if let Some(cache) = cache_dir {
            Self::ensure_qemu_static(arch, cache)?
        } else {
            return Err(PootstrapError::MissingTool {
                tool: format!("{} (qemu-user-static)", arch.qemu_binary()),
            });
        };

        let target_dir = rootfs.join("usr").join("bin");
        if fs::create_dir_all(&target_dir).is_err() {
            PrivilegedRunner::create_dir_all(&target_dir)?;
        }

        let target_qemu = target_dir.join(arch.qemu_binary());
        if fs::copy(&host_qemu, &target_qemu).is_err() {
            PrivilegedRunner::copy(&host_qemu, &target_qemu)?;
        }

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if fs::set_permissions(&target_qemu, fs::Permissions::from_mode(0o755)).is_err() {
                let _ = PrivilegedRunner::command("chmod")
                    .arg("a+rx")
                    .arg(&target_qemu)
                    .status();
            }
        }

        let _ = Self::register_binfmt(arch, &host_qemu);

        tracing::debug!("Installed {:?} into rootfs {:?}", host_qemu, target_qemu);
        Ok(())
    }
}
