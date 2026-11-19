use crate::config::Config;
use crate::error::{PootstrapError, Result};
use crate::profile::FlavourProfile;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

pub struct IsoBuilder<'a> {
    config: &'a Config,
    flavour: &'a FlavourProfile,
    rootfs_dir: PathBuf,
    output_iso: PathBuf,
    volume_id: String,
}

impl<'a> IsoBuilder<'a> {
    pub fn new(
        config: &'a Config,
        flavour: &'a FlavourProfile,
        rootfs_dir: PathBuf,
        output_iso: PathBuf,
    ) -> Self {
        Self {
            config,
            flavour,
            rootfs_dir,
            output_iso,
            volume_id: "PARCH_AARCH64".to_string(),
        }
    }

    pub fn with_volume_id(mut self, volume_id: String) -> Self {
        self.volume_id = volume_id;
        self
    }

    pub fn build(&self) -> Result<PathBuf> {
        which::which("xorriso").map_err(|_| PootstrapError::MissingTool {
            tool: "xorriso".into(),
        })?;
        which::which("mksquashfs").map_err(|_| PootstrapError::MissingTool {
            tool: "mksquashfs (squashfs-tools)".into(),
        })?;
        which::which("mkfs.fat").map_err(|_| PootstrapError::MissingTool {
            tool: "mkfs.fat (dosfstools)".into(),
        })?;

        tracing::info!(
            "Building Parch Linux aarch64 Live ISO (flavour: {})",
            self.flavour.name
        );

        let staging = self.config.work_dir.join("iso_staging");
        if staging.exists() {
            let _ = fs::remove_dir_all(&staging);
        }
        fs::create_dir_all(&staging).map_err(PootstrapError::Io)?;

        let airootfs_dir = staging.join("parch").join("aarch64");
        let boot_dir = staging.join("parch").join("boot").join("aarch64");
        let efi_dir = staging.join("EFI");
        let loader_entries_dir = staging.join("loader").join("entries");

        fs::create_dir_all(&airootfs_dir).map_err(PootstrapError::Io)?;
        fs::create_dir_all(&boot_dir).map_err(PootstrapError::Io)?;
        fs::create_dir_all(&efi_dir).map_err(PootstrapError::Io)?;
        fs::create_dir_all(&loader_entries_dir).map_err(PootstrapError::Io)?;

        let squashfs_path = airootfs_dir.join("airootfs.sfs");
        tracing::info!("Compressing rootfs into SquashFS archive: {:?}", squashfs_path);
        let mksquash_status = Command::new("mksquashfs")
            .arg(&self.rootfs_dir)
            .arg(&squashfs_path)
            .arg("-comp")
            .arg("zstd")
            .arg("-Xcompression-level")
            .arg("15")
            .arg("-noappend")
            .status()
            .map_err(PootstrapError::Io)?;

        if !mksquash_status.success() {
            return Err(PootstrapError::ImageBuild(
                "mksquashfs failed to create airootfs.sfs".into(),
            ));
        }

        let kernel_src = [
            self.rootfs_dir.join("boot").join("Image"),
            self.rootfs_dir.join("boot").join("vmlinuz-linux"),
        ]
        .into_iter()
        .find(|p| p.exists())
        .ok_or_else(|| {
            PootstrapError::ImageBuild(
                "No kernel image (Image or vmlinuz-linux) found in rootfs /boot".into(),
            )
        })?;

        let initrd_src = [
            self.rootfs_dir.join("boot").join("initramfs-linux.img"),
            self.rootfs_dir.join("boot").join("initrd.img"),
        ]
        .into_iter()
        .find(|p| p.exists())
        .ok_or_else(|| {
            PootstrapError::ImageBuild(
                "No initramfs (initramfs-linux.img) found in rootfs /boot".into(),
            )
        })?;

        let target_kernel = boot_dir.join("vmlinuz-linux");
        let target_initrd = boot_dir.join("initramfs-linux.img");

        let _ = fs::copy(&kernel_src, &target_kernel);
        let _ = fs::copy(&initrd_src, &target_initrd);

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = fs::Permissions::from_mode(0o644);
            let _ = fs::set_permissions(&target_kernel, perms.clone());
            let _ = fs::set_permissions(&target_initrd, perms);
        }

        let loader_conf = staging.join("loader").join("loader.conf");
        fs::write(
            &loader_conf,
            "default parch.conf\ntimeout 5\nconsole-mode max\n",
        )
        .map_err(PootstrapError::Io)?;

        let parch_entry = loader_entries_dir.join("parch.conf");
        let entry_content = format!(
            "title Parch Linux ARM64 Live ({flavour})\n\
             linux /parch/boot/aarch64/vmlinuz-linux\n\
             initrd /parch/boot/aarch64/initramfs-linux.img\n\
             options archisobasedir=parch archisolabel={volid} console=tty0 console=ttyAMA0,115200 rootwait\n",
            flavour = self.flavour.display_name,
            volid = self.volume_id
        );
        fs::write(&parch_entry, entry_content).map_err(PootstrapError::Io)?;

        let efiboot_img = efi_dir.join("efiboot.img");
        tracing::info!("Formatting EFI boot partition image: {:?}", efiboot_img);

        let mkfs_status = Command::new("mkfs.fat")
            .arg("-C")
            .arg(&efiboot_img)
            .arg("32768")
            .status()
            .map_err(PootstrapError::Io)?;

        if !mkfs_status.success() {
            return Err(PootstrapError::ImageBuild(
                "mkfs.fat failed to format efiboot.img".into(),
            ));
        }

        let efi_candidates = [
            self.rootfs_dir
                .join("usr/lib/systemd/boot/efi/systemd-bootaa64.efi"),
            PathBuf::from("/usr/lib/systemd/boot/efi/systemd-bootaa64.efi"),
        ];

        let efi_bin = efi_candidates.into_iter().find(|p| p.exists());

        if let Some(bin) = efi_bin {
            let iso_efi_boot = staging.join("EFI").join("BOOT");
            let _ = fs::create_dir_all(&iso_efi_boot);
            let _ = fs::copy(&bin, iso_efi_boot.join("BOOTAA64.EFI"));

            if which::which("mmd").is_ok() && which::which("mcopy").is_ok() {
                let _ = Command::new("mmd")
                    .arg("-i")
                    .arg(&efiboot_img)
                    .arg("::EFI")
                    .arg("::EFI/BOOT")
                    .arg("::loader")
                    .arg("::loader/entries")
                    .status();

                let _ = Command::new("mcopy")
                    .arg("-i")
                    .arg(&efiboot_img)
                    .arg(&bin)
                    .arg("::EFI/BOOT/BOOTAA64.EFI")
                    .status();

                let _ = Command::new("mcopy")
                    .arg("-i")
                    .arg(&efiboot_img)
                    .arg(&loader_conf)
                    .arg("::loader/loader.conf")
                    .status();

                let _ = Command::new("mcopy")
                    .arg("-i")
                    .arg(&efiboot_img)
                    .arg(&parch_entry)
                    .arg("::loader/entries/parch.conf")
                    .status();
            }
        } else {
            tracing::warn!("systemd-bootaa64.efi not located; EFI image created without preloaded bootloader binary");
        }

        if let Some(parent) = self.output_iso.parent() {
            let _ = fs::create_dir_all(parent);
        }

        tracing::info!("Mastering hybrid aarch64 ISO to {:?}", self.output_iso);
        let xorriso_status = Command::new("xorriso")
            .arg("-as")
            .arg("mkisofs")
            .arg("-iso-level")
            .arg("3")
            .arg("-full-iso9660-filenames")
            .arg("-volid")
            .arg(&self.volume_id)
            .arg("-eltorito-alt-boot")
            .arg("-e")
            .arg("EFI/efiboot.img")
            .arg("-no-emul-boot")
            .arg("-isohybrid-gpt-basdat")
            .arg("-output")
            .arg(&self.output_iso)
            .arg(&staging)
            .status()
            .map_err(PootstrapError::Io)?;

        if !xorriso_status.success() {
            return Err(PootstrapError::ImageBuild(
                "xorriso failed to master hybrid aarch64 ISO".into(),
            ));
        }

        tracing::info!(
            "Parch Linux aarch64 Live ISO successfully created: {:?}",
            self.output_iso
        );
        Ok(self.output_iso.clone())
    }
}
