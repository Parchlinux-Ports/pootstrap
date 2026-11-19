use crate::config::Config;
use crate::distro::LocalRepoManager;
use crate::error::{PootstrapError, Result};
use crate::isolation::PrivilegedRunner;
use crate::profile::device::Architecture;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

pub struct PackageBuilder<'a> {
    config: &'a Config,
    pkgbuild_dir: PathBuf,
    arch: Architecture,
}

impl<'a> PackageBuilder<'a> {
    pub fn new(config: &'a Config, pkgbuild_dir: PathBuf, arch: Architecture) -> Self {
        Self {
            config,
            pkgbuild_dir,
            arch,
        }
    }

    pub fn build(&self) -> Result<Vec<PathBuf>> {
        let pkgbuild = self.pkgbuild_dir.join("PKGBUILD");
        if !pkgbuild.exists() {
            return Err(PootstrapError::RootfsBuild(format!(
                "No PKGBUILD found in directory {:?}",
                self.pkgbuild_dir
            )));
        }

        tracing::info!(
            "Building package from {:?} targeting architecture {}",
            self.pkgbuild_dir,
            self.arch
        );

        let is_native = (self.arch == Architecture::X86_64 && cfg!(target_arch = "x86_64"))
            || (self.arch == Architecture::Aarch64 && cfg!(target_arch = "aarch64"));

        if is_native && which::which("makepkg").is_ok() {
            tracing::info!("Executing native makepkg in {:?}", self.pkgbuild_dir);
            let status = Command::new("makepkg")
                .arg("-s")
                .arg("--noconfirm")
                .current_dir(&self.pkgbuild_dir)
                .status()
                .map_err(PootstrapError::Io)?;

            if !status.success() {
                return Err(PootstrapError::CommandFailed {
                    command: "makepkg -s --noconfirm".into(),
                    exit_code: status.code(),
                    stderr: "makepkg execution failed".into(),
                });
            }
        } else {
            let buildroot = self
                .config
                .work_dir
                .join("buildroot")
                .join(self.arch.as_str());

            if !buildroot.exists() {
                tracing::info!(
                    "Preparing isolated buildroot environment at {:?}...",
                    buildroot
                );
                fs::create_dir_all(&buildroot).map_err(PootstrapError::Io)?;

                let _ = crate::isolation::BinfmtChecker::setup_qemu_in_rootfs(
                    self.arch,
                    &buildroot,
                    Some(&self.config.cache_dir),
                );

                let pacman_conf = self.config.work_dir.join("pacman-buildroot.conf");
                crate::distro::PacmanManager::generate_pacman_conf(
                    self.config,
                    self.arch,
                    &pacman_conf,
                )?;

                let base_packages = vec![
                    "base".to_string(),
                    "base-devel".to_string(),
                    "git".to_string(),
                    "sudo".to_string(),
                ];

                if which::which("pacstrap").is_ok() {
                    let _ = crate::distro::PacmanManager::run_pacstrap(
                        &pacman_conf,
                        &self.config.cache_dir,
                        &buildroot,
                        &base_packages,
                    );
                }
            }

            let _guard = crate::isolation::MountGuard::mount_pseudo_filesystems(&buildroot)?;
            crate::isolation::ChrootRunner::setup_resolv_conf(&buildroot)?;

            let target_build_dir = buildroot.join("build");
            if fs::create_dir_all(&target_build_dir).is_err() {
                PrivilegedRunner::create_dir_all(&target_build_dir)?;
            }

            let mut bind_cmd = PrivilegedRunner::command("mount");
            bind_cmd
                .arg("--bind")
                .arg(&self.pkgbuild_dir)
                .arg(&target_build_dir);
            let _ = bind_cmd.status();

            let makepkg_cmd = "useradd -m -g users -s /bin/bash builder 2>/dev/null || true; \
                               echo 'builder ALL=(ALL) NOPASSWD: ALL' > /etc/sudoers.d/builder; \
                               chmod 440 /etc/sudoers.d/builder; \
                               chown -R builder:users /build; \
                               su - builder -c 'cd /build && makepkg -s --noconfirm'";

            tracing::info!("Executing makepkg inside isolated buildroot chroot...");
            let status = PrivilegedRunner::command("chroot")
                .arg(&buildroot)
                .arg("/bin/bash")
                .arg("-c")
                .arg(makepkg_cmd)
                .status()
                .map_err(PootstrapError::Io)?;

            let _ = PrivilegedRunner::command("umount").arg(&target_build_dir).status();

            if !status.success() {
                return Err(PootstrapError::CommandFailed {
                    command: "chroot makepkg".into(),
                    exit_code: status.code(),
                    stderr: "makepkg in isolated buildroot exited with error".into(),
                });
            }
        }

        let repo_dir = self.config.work_dir.join("repo").join(self.arch.as_str());
        if !repo_dir.exists() {
            fs::create_dir_all(&repo_dir).map_err(PootstrapError::Io)?;
        }

        let mut collected_packages = Vec::new();
        if let Ok(entries) = fs::read_dir(&self.pkgbuild_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                let pkg_name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .filter(|name| name.ends_with(".pkg.tar.zst") || name.ends_with(".pkg.tar.xz"));
                if let Some(name) = pkg_name {
                    let dest = repo_dir.join(name);
                    let _ = fs::copy(&path, &dest);
                    collected_packages.push(dest);
                }
            }
        }

        if collected_packages.is_empty() {
            tracing::warn!("No generated packages found in {:?}", self.pkgbuild_dir);
        } else {
            tracing::info!(
                "Successfully built {} package(s). Updating local repository...",
                collected_packages.len()
            );
            let _ = LocalRepoManager::update_repo(&repo_dir, None);
        }

        Ok(collected_packages)
    }
}
