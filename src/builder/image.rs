use crate::error::{PootstrapError, Result};
use crate::isolation::PrivilegedRunner;
use crate::profile::DeviceProfile;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFormat {
    Raw,
    Qcow2,
}

pub struct LoopDeviceGuard {
    device_path: PathBuf,
    detached: bool,
}

impl LoopDeviceGuard {
    pub fn attach(image_path: &Path) -> Result<Self> {
        let output = PrivilegedRunner::command("losetup")
            .arg("-P")
            .arg("-f")
            .arg("--show")
            .arg(image_path)
            .output()
            .map_err(PootstrapError::Io)?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            return Err(PootstrapError::ImageBuild(format!(
                "losetup failed: {}",
                stderr
            )));
        }

        let dev_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let device_path = PathBuf::from(dev_str);
        tracing::info!("Attached image to loop device: {:?}", device_path);
        Ok(Self {
            device_path,
            detached: false,
        })
    }

    pub fn device(&self) -> &Path {
        &self.device_path
    }

    pub fn partition(&self, num: u32) -> PathBuf {
        let dev_str = self.device_path.to_string_lossy();
        if dev_str.ends_with(|c: char| c.is_ascii_digit()) {
            PathBuf::from(format!("{}p{}", dev_str, num))
        } else {
            PathBuf::from(format!("{}{}", dev_str, num))
        }
    }

    pub fn detach(&mut self) -> Result<()> {
        if !self.detached && self.device_path.exists() {
            self.detached = true;
            let status = PrivilegedRunner::command("losetup")
                .arg("-d")
                .arg(&self.device_path)
                .status()
                .map_err(PootstrapError::Io)?;

            if !status.success() {
                tracing::warn!("Failed to detach loop device {:?}", self.device_path);
            } else {
                tracing::debug!("Detached loop device {:?}", self.device_path);
            }
        }
        Ok(())
    }
}

impl Drop for LoopDeviceGuard {
    fn drop(&mut self) {
        let _ = self.detach();
    }
}

pub struct ImageBuilder<'a> {
    device: &'a DeviceProfile,
    rootfs_dir: &'a Path,
    output_path: PathBuf,
    format: ImageFormat,
}

impl<'a> ImageBuilder<'a> {
    pub fn new(device: &'a DeviceProfile, rootfs_dir: &'a Path, output_path: PathBuf) -> Self {
        let format = if output_path
            .extension()
            .is_some_and(|ext| ext == "qcow2")
        {
            ImageFormat::Qcow2
        } else {
            ImageFormat::Raw
        };

        Self {
            device,
            rootfs_dir,
            output_path,
            format,
        }
    }

    pub fn with_format(mut self, format: ImageFormat) -> Self {
        self.format = format;
        self
    }

    pub fn build(&self) -> Result<PathBuf> {
        tracing::info!(
            "Building disk image for '{}' at {:?} (format: {:?})",
            self.device.codename,
            self.output_path,
            self.format
        );

        for tool in &["truncate", "parted", "losetup", "mkfs.ext4", "mkfs.vfat", "rsync"] {
            which::which(tool).map_err(|_| PootstrapError::MissingTool {
                tool: tool.to_string(),
            })?;
        }

        if self.format == ImageFormat::Qcow2 {
            which::which("qemu-img").map_err(|_| PootstrapError::MissingTool {
                tool: "qemu-img".to_string(),
            })?;
        }

        if let Some(parent) = self.output_path.parent() {
            let _ = fs::create_dir_all(parent).or_else(|_| PrivilegedRunner::create_dir_all(parent));
        }

        let raw_image_path = if self.format == ImageFormat::Qcow2 {
            self.output_path.with_extension("raw.tmp")
        } else {
            self.output_path.clone()
        };

        let total_size_mb = self.device.boot_size_mb + self.device.root_size_mb + 32;

        // 1. Create sparse image file
        tracing::info!("Allocating {} MB disk image file...", total_size_mb);
        let status = Command::new("truncate")
            .arg("-s")
            .arg(format!("{}M", total_size_mb))
            .arg(&raw_image_path)
            .status()
            .map_err(PootstrapError::Io)?;

        if !status.success() {
            return Err(PootstrapError::ImageBuild(
                "truncate failed to create sparse file".into(),
            ));
        }

        // 2. Partition image
        tracing::info!("Creating partition table...");
        let boot_end_mb = 4 + self.device.boot_size_mb;

        let boot_end_str = format!("{}MiB", boot_end_mb);
        let output_str = raw_image_path.to_str().unwrap();

        let parted_cmds: [&[&str]; 4] = [
            &["-s", output_str, "mklabel", "msdos"],
            &[
                "-s",
                output_str,
                "mkpart",
                "primary",
                "fat32",
                "4MiB",
                &boot_end_str,
            ],
            &[
                "-s",
                output_str,
                "set",
                "1",
                "boot",
                "on",
            ],
            &[
                "-s",
                output_str,
                "mkpart",
                "primary",
                "ext4",
                &boot_end_str,
                "100%",
            ],
        ];

        for args in parted_cmds {
            let status = PrivilegedRunner::command("parted")
                .args(args)
                .status()
                .map_err(PootstrapError::Io)?;
            if !status.success() {
                return Err(PootstrapError::ImageBuild(format!(
                    "parted failed executing {:?}",
                    args
                )));
            }
        }

        // 3. Attach loop device
        let mut loop_guard = LoopDeviceGuard::attach(&raw_image_path)?;
        let boot_part = loop_guard.partition(1);
        let root_part = loop_guard.partition(2);

        let _ = PrivilegedRunner::command("partprobe")
            .arg(loop_guard.device())
            .status();
        std::thread::sleep(std::time::Duration::from_millis(500));

        // 4. Format partitions
        tracing::info!("Formatting boot partition ({:?})...", boot_part);
        let status = PrivilegedRunner::command("mkfs.vfat")
            .arg("-F32")
            .arg("-n")
            .arg("PARCH_BOOT")
            .arg(&boot_part)
            .status()
            .map_err(PootstrapError::Io)?;
        if !status.success() {
            return Err(PootstrapError::ImageBuild("mkfs.vfat failed on boot partition".into()));
        }

        tracing::info!("Formatting root partition ({:?})...", root_part);
        let status = PrivilegedRunner::command("mkfs.ext4")
            .arg("-F")
            .arg("-L")
            .arg("PARCH_ROOT")
            .arg("-O")
            .arg("^metadata_csum_seed")
            .arg(&root_part)
            .status()
            .map_err(PootstrapError::Io)?;
        if !status.success() {
            return Err(PootstrapError::ImageBuild("mkfs.ext4 failed on root partition".into()));
        }

        // 5. Mount partitions and copy rootfs
        let tmp_mount = tempfile::tempdir().map_err(PootstrapError::Io)?;
        let mnt_root = tmp_mount.path();
        let mnt_boot = mnt_root.join("boot");

        tracing::info!("Mounting target image partitions...");
        let status = PrivilegedRunner::command("mount")
            .arg(&root_part)
            .arg(mnt_root)
            .status()
            .map_err(PootstrapError::Io)?;
        if !status.success() {
            return Err(PootstrapError::MountError {
                path: mnt_root.to_path_buf(),
                reason: "Failed to mount root partition".into(),
            });
        }

        PrivilegedRunner::create_dir_all(&mnt_boot)?;
        let status = PrivilegedRunner::command("mount")
            .arg(&boot_part)
            .arg(&mnt_boot)
            .status()
            .map_err(PootstrapError::Io)?;
        if !status.success() {
            let _ = PrivilegedRunner::command("umount").arg("-l").arg(mnt_root).status();
            return Err(PootstrapError::MountError {
                path: mnt_boot,
                reason: "Failed to mount boot partition".into(),
            });
        }

        tracing::info!("Copying rootfs files into disk image...");
        let status = PrivilegedRunner::command("rsync")
            .arg("-aHAX")
            .arg("--info=progress2")
            .arg(format!("{}/", self.rootfs_dir.to_str().unwrap()))
            .arg(format!("{}/", mnt_root.to_str().unwrap()))
            .status()
            .map_err(PootstrapError::Io)?;

        if !status.success() {
            let _ = PrivilegedRunner::command("umount").arg("-l").arg(&mnt_boot).status();
            let _ = PrivilegedRunner::command("umount").arg("-l").arg(mnt_root).status();
            return Err(PootstrapError::ImageBuild("rsync failed copying rootfs".into()));
        }

        let _ = PrivilegedRunner::command("sync").status();

        let _ = PrivilegedRunner::command("umount").arg("-l").arg(&mnt_boot).status();
        let _ = PrivilegedRunner::command("umount").arg("-l").arg(mnt_root).status();

        loop_guard.detach()?;

        // 6. Convert to QCOW2 if requested
        if self.format == ImageFormat::Qcow2 {
            tracing::info!("Converting raw image to QCOW2 ({:?})...", self.output_path);
            let status = Command::new("qemu-img")
                .arg("convert")
                .arg("-f")
                .arg("raw")
                .arg("-O")
                .arg("qcow2")
                .arg(&raw_image_path)
                .arg(&self.output_path)
                .status()
                .map_err(PootstrapError::Io)?;

            let _ = fs::remove_file(&raw_image_path);

            if !status.success() {
                return Err(PootstrapError::ImageBuild(
                    "qemu-img convert failed to produce qcow2".into(),
                ));
            }
        }

        tracing::info!("Disk image build complete: {:?}", self.output_path);
        Ok(self.output_path.clone())
    }
}
