use crate::error::{PootstrapError, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct FastbootFlasher;

#[allow(dead_code)]
impl FastbootFlasher {
    pub fn check_fastboot() -> Result<()> {
        which::which("fastboot").map_err(|_| PootstrapError::MissingTool {
            tool: "fastboot (android-tools)".into(),
        })?;
        Ok(())
    }

    pub fn list_devices() -> Result<Vec<String>> {
        Self::check_fastboot()?;
        let output = Command::new("fastboot")
            .arg("devices")
            .output()
            .map_err(PootstrapError::Io)?;

        if !output.status.success() {
            return Err(PootstrapError::FlashError(
                "Failed to query fastboot devices".into(),
            ));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let devices: Vec<String> = stdout
            .lines()
            .filter_map(|line| {
                let parts: Vec<&str> = line.split_whitespace().collect();
                parts.first().map(|s| s.to_string())
            })
            .collect();

        Ok(devices)
    }

    pub fn query_max_download_size() -> Result<Option<u64>> {
        Self::check_fastboot()?;
        let devices = Self::list_devices()?;
        if devices.is_empty() {
            return Ok(None);
        }

        let output = Command::new("fastboot")
            .arg("getvar")
            .arg("max-download-size")
            .output()
            .map_err(PootstrapError::Io)?;

        let combined = format!(
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );

        for line in combined.lines() {
            if let Some(val) = line.strip_prefix("max-download-size:") {
                let trimmed = val.trim();
                let parsed = if let Some(hex) = trimmed.strip_prefix("0x") {
                    u64::from_str_radix(hex, 16).ok()
                } else {
                    trimmed.parse::<u64>().ok()
                };
                if let Some(size) = parsed {
                    return Ok(Some(size));
                }
            }
        }

        Ok(None)
    }

    pub fn flash_partition(partition: &str, image_path: &Path) -> Result<()> {
        Self::check_fastboot()?;
        tracing::info!("Flashing partition '{}' with {:?}...", partition, image_path);

        let status = Command::new("fastboot")
            .arg("flash")
            .arg(partition)
            .arg(image_path)
            .status()
            .map_err(PootstrapError::Io)?;

        if !status.success() {
            return Err(PootstrapError::FlashError(format!(
                "fastboot flash {} failed with exit code {:?}",
                partition,
                status.code()
            )));
        }

        tracing::info!("Successfully flashed partition '{}'", partition);
        Ok(())
    }

    pub fn flash_sparse_chunks(partition: &str, chunks: &[PathBuf]) -> Result<()> {
        Self::check_fastboot()?;
        tracing::info!(
            "Flashing {} sparse chunks to partition '{}'...",
            chunks.len(),
            partition
        );

        for (idx, chunk) in chunks.iter().enumerate() {
            tracing::info!(
                "Flashing sparse chunk {}/{} ({:?}) to '{}'...",
                idx + 1,
                chunks.len(),
                chunk,
                partition
            );

            let status = Command::new("fastboot")
                .arg("flash")
                .arg(partition)
                .arg(chunk)
                .status()
                .map_err(PootstrapError::Io)?;

            if !status.success() {
                return Err(PootstrapError::FlashError(format!(
                    "fastboot flash {} failed on chunk {:?} with exit code {:?}",
                    partition,
                    chunk,
                    status.code()
                )));
            }
        }

        tracing::info!(
            "Successfully flashed all {} sparse chunks to partition '{}'",
            chunks.len(),
            partition
        );
        Ok(())
    }

    pub fn boot_kernel(boot_image: &Path) -> Result<()> {
        Self::check_fastboot()?;
        tracing::info!("Booting kernel image {:?} via fastboot...", boot_image);

        let status = Command::new("fastboot")
            .arg("boot")
            .arg(boot_image)
            .status()
            .map_err(PootstrapError::Io)?;

        if !status.success() {
            return Err(PootstrapError::FlashError(format!(
                "fastboot boot failed with exit code {:?}",
                status.code()
            )));
        }

        Ok(())
    }
}
