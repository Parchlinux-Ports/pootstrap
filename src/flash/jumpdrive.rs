use crate::error::{PootstrapError, Result};
use crate::isolation::PrivilegedRunner;
use std::fs;
use std::path::{Path, PathBuf};

pub struct JumpdriveFlasher;

impl JumpdriveFlasher {
    pub fn find_jumpdrive_devices() -> Result<Vec<PathBuf>> {
        let disk_by_id = Path::new("/dev/disk/by-id");
        let mut candidates = Vec::new();

        if let Ok(entries) = fs::read_dir(disk_by_id) {
            for entry in entries.flatten() {
                let file_name = entry.file_name().to_string_lossy().to_string();
                if file_name.to_lowercase().contains("jumpdrive")
                    || file_name.to_lowercase().contains("pine64")
                {
                    candidates.push(entry.path());
                }
            }
        }

        Ok(candidates)
    }

    pub fn flash_block_device(image_path: &Path, target_dev: &Path) -> Result<()> {
        which::which("dd").map_err(|_| PootstrapError::MissingTool {
            tool: "dd (coreutils)".into(),
        })?;

        tracing::info!(
            "Flashing image {:?} to block device {:?} via dd...",
            image_path,
            target_dev
        );

        let status = PrivilegedRunner::command("dd")
            .arg(format!("if={}", image_path.to_str().unwrap()))
            .arg(format!("of={}", target_dev.to_str().unwrap()))
            .arg("bs=4M")
            .arg("oflag=direct")
            .arg("status=progress")
            .arg("conv=fsync")
            .status()
            .map_err(PootstrapError::Io)?;

        if !status.success() {
            return Err(PootstrapError::FlashError(format!(
                "dd command failed with exit code {:?}",
                status.code()
            )));
        }

        tracing::info!("Image successfully written to {:?}", target_dev);
        Ok(())
    }
}
