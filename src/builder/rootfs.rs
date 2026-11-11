use crate::config::Config;
use crate::distro::PacmanManager;
use crate::error::{PootstrapError, Result};
use crate::isolation::{BinfmtChecker, ChrootRunner, MountGuard, PrivilegedRunner};
use crate::profile::{DeviceProfile, FlavourProfile};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn is_valid_rootfs(path: &Path) -> bool {
    path.exists() && path.join("etc").exists()
}

pub struct RootfsBuilder<'a> {
    config: &'a Config,
    device: &'a DeviceProfile,
    flavour: &'a FlavourProfile,
    target_rootfs: PathBuf,
}

impl<'a> RootfsBuilder<'a> {
    pub fn new(
        config: &'a Config,
        device: &'a DeviceProfile,
        flavour: &'a FlavourProfile,
        target_rootfs: PathBuf,
    ) -> Self {
        Self {
            config,
            device,
            flavour,
            target_rootfs,
        }
    }

    pub fn build(&self) -> Result<()> {
        tracing::info!(
            "Building rootfs for device '{}' with flavour '{}' at {:?}",
            self.device.codename,
            self.flavour.name,
            self.target_rootfs
        );

        if fs::create_dir_all(&self.target_rootfs).is_err() {
            PrivilegedRunner::create_dir_all(&self.target_rootfs)?;
        }

        let has_binfmt = BinfmtChecker::check_support(self.device.arch).is_ok();
        let has_pacstrap = which::which("pacstrap").is_ok();

        if has_binfmt && has_pacstrap {
            tracing::info!("Host supports binfmt and has pacstrap. Running native package installation...");
            let pacman_conf_path = self.config.work_dir.join("pacman.conf");
            PacmanManager::generate_pacman_conf(
                self.config,
                self.device.arch,
                &pacman_conf_path,
            )?;

            let mut packages = vec![
                "base".to_string(),
                "linux-firmware".to_string(),
                "sudo".to_string(),
                "nano".to_string(),
                "networkmanager".to_string(),
                "pipewire".to_string(),
                "pipewire-pulse".to_string(),
                "wireplumber".to_string(),
            ];

            packages.push(self.device.kernel_package.clone());
            packages.extend(self.device.extra_packages.clone());
            packages.extend(self.flavour.packages.clone());

            PacmanManager::run_pacstrap(
                &pacman_conf_path,
                &self.config.cache_dir,
                &self.target_rootfs,
                &packages,
            )?;

            BinfmtChecker::setup_qemu_in_rootfs(self.device.arch, &self.target_rootfs)?;
        } else {
            tracing::info!(
                "Host lacks foreign binfmt or pacstrap. Bootstrapping via Arch Linux ARM rootfs archive..."
            );
            self.bootstrap_alarm_tarball(&self.target_rootfs)?;
        }

        ChrootRunner::setup_resolv_conf(&self.target_rootfs)?;
        self.configure_system(has_binfmt)?;

        let boot_dir = self.target_rootfs.join("boot");
        if boot_dir.exists() {
            let _ = PrivilegedRunner::command("chmod")
                .arg("-R")
                .arg("a+rX")
                .arg(&boot_dir)
                .status();
        }

        tracing::info!("Rootfs build completed successfully for {}", self.device.codename);
        Ok(())
    }

    pub fn bootstrap_alarm_tarball(&self, rootfs: &Path) -> Result<()> {
        let arch_name = self.device.arch.as_str();
        let tarball_filename = format!("ArchLinuxARM-{}-latest.tar.gz", arch_name);
        let download_dir = self.config.cache_dir.join("downloads");
        let _ = fs::create_dir_all(&download_dir);
        let tarball_path = download_dir.join(&tarball_filename);

        if !tarball_path.exists() {
            let url = format!("http://os.archlinuxarm.org/os/{}", tarball_filename);
            tracing::info!("Downloading Arch Linux ARM rootfs from {}...", url);
            let status = Command::new("curl")
                .arg("-L")
                .arg("-f")
                .arg("--progress-bar")
                .arg("-o")
                .arg(&tarball_path)
                .arg(&url)
                .status()
                .map_err(PootstrapError::Io)?;
            if !status.success() {
                return Err(PootstrapError::RootfsBuild(format!(
                    "Failed to download Arch Linux ARM tarball from {}",
                    url
                )));
            }
        } else {
            tracing::info!("Using cached Arch Linux ARM rootfs at {:?}", tarball_path);
        }

        tracing::info!("Extracting Arch Linux ARM rootfs into {:?}...", rootfs);
        let status = PrivilegedRunner::command("tar")
            .arg("-xzf")
            .arg(&tarball_path)
            .arg("-C")
            .arg(rootfs)
            .status()
            .map_err(PootstrapError::Io)?;

        if !status.success() {
            return Err(PootstrapError::RootfsBuild(
                "Failed to extract Arch Linux ARM tarball".into(),
            ));
        }

        Ok(())
    }

    fn configure_system(&self, has_binfmt: bool) -> Result<()> {
        let rootfs = &self.target_rootfs;

        // Hostname
        let hostname_path = rootfs.join("etc").join("hostname");
        PrivilegedRunner::write_file(&hostname_path, &format!("{}\n", self.config.hostname))?;

        // Hosts file
        let hosts_path = rootfs.join("etc").join("hosts");
        let hosts_content = format!(
            "127.0.0.1\tlocalhost\n::1\tlocalhost\n127.0.1.1\t{}.localdomain\t{}\n",
            self.config.hostname, self.config.hostname
        );
        PrivilegedRunner::write_file(&hosts_path, &hosts_content)?;

        // Locale
        let locale_gen_path = rootfs.join("etc").join("locale.gen");
        PrivilegedRunner::write_file(&locale_gen_path, "en_US.UTF-8 UTF-8\n")?;

        let locale_conf_path = rootfs.join("etc").join("locale.conf");
        PrivilegedRunner::write_file(&locale_conf_path, "LANG=en_US.UTF-8\n")?;

        // Sudoers configuration
        let sudoers_d = rootfs.join("etc").join("sudoers.d");
        PrivilegedRunner::create_dir_all(&sudoers_d)?;
        let wheel_sudo = sudoers_d.join("10-wheel");
        PrivilegedRunner::write_file(&wheel_sudo, "%wheel ALL=(ALL:ALL) NOPASSWD: ALL\n")?;

        // Fstab generation
        let fstab_path = rootfs.join("etc").join("fstab");
        let fstab_content = r#"# /etc/fstab: static file system information.
LABEL=PARCH_BOOT  /boot  vfat  defaults  0  2
LABEL=PARCH_ROOT  /      ext4  defaults,noatime  0  1
"#;
        PrivilegedRunner::write_file(&fstab_path, fstab_content)?;

        // If host has binfmt emulation, run in-chroot commands
        if has_binfmt {
            let _mount_guard = MountGuard::mount_pseudo_filesystems(rootfs)?;

            let _ = ChrootRunner::run_in_chroot(rootfs, &["locale-gen"]);

            let user = &self.config.default_user;
            let useradd_cmd = &[
                "useradd",
                "-m",
                "-g",
                "users",
                "-G",
                "wheel,video,audio,input,storage",
                "-s",
                "/bin/bash",
                user,
            ];
            let _ = ChrootRunner::run_in_chroot(rootfs, useradd_cmd);

            let passwd_script = format!(
                "echo '{user}:{user}' | chpasswd && echo 'root:parch' | chpasswd",
                user = user
            );
            let _ = ChrootRunner::run_in_chroot(rootfs, &["/bin/sh", "-c", &passwd_script]);

            for service in &self.flavour.services {
                let _ = ChrootRunner::run_in_chroot(rootfs, &["systemctl", "enable", service]);
            }

            tracing::info!("Generating target initramfs with mkinitcpio...");
            let _ = ChrootRunner::run_in_chroot(rootfs, &["mkinitcpio", "-P"]);
        }

        Ok(())
    }
}
