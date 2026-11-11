use crate::error::{PootstrapError, Result};
use std::path::Path;
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
