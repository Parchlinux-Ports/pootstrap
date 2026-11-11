use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Architecture {
    Aarch64,
    Armv7h,
    X86_64,
}

impl Architecture {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Aarch64 => "aarch64",
            Self::Armv7h => "armv7h",
            Self::X86_64 => "x86_64",
        }
    }

    pub fn qemu_binary(&self) -> &'static str {
        match self {
            Self::Aarch64 => "qemu-aarch64-static",
            Self::Armv7h => "qemu-arm-static",
            Self::X86_64 => "qemu-x86_64-static",
        }
    }
}

impl fmt::Display for Architecture {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for Architecture {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "aarch64" | "arm64" => Ok(Self::Aarch64),
            "armv7h" | "armv7" | "armhf" => Ok(Self::Armv7h),
            "x86_64" | "amd64" => Ok(Self::X86_64),
            other => Err(format!("Unsupported architecture: {}", other)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BootloaderType {
    UBoot,
    Lk2nd,
    Abootimg,
    Uefi,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FlashMethod {
    Fastboot,
    Jumpdrive,
    DirectBlock,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FilesystemType {
    Ext4,
    F2fs,
    Fat32,
}

#[allow(dead_code)]
impl FilesystemType {
    pub fn mkfs_cmd(&self) -> &'static str {
        match self {
            Self::Ext4 => "mkfs.ext4",
            Self::F2fs => "mkfs.f2fs",
            Self::Fat32 => "mkfs.vfat",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenConfig {
    pub width: u32,
    pub height: u32,
    pub dpi: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceProfile {
    pub codename: String,
    pub name: String,
    pub manufacturer: String,
    pub arch: Architecture,
    pub soc: String,
    pub kernel_package: String,
    pub dtb: Option<String>,
    pub bootloader: BootloaderType,
    pub flash_method: FlashMethod,
    pub screen: Option<ScreenConfig>,
    pub boot_fs: FilesystemType,
    pub root_fs: FilesystemType,
    pub boot_size_mb: u64,
    pub root_size_mb: u64,
    pub extra_packages: Vec<String>,
}

impl DeviceProfile {
    pub fn builtin_profiles() -> Vec<Self> {
        vec![
            Self {
                codename: "qemu-aarch64".into(),
                name: "QEMU ARM64 Virtual Machine".into(),
                manufacturer: "QEMU".into(),
                arch: Architecture::Aarch64,
                soc: "ARMv8 Virt Platform".into(),
                kernel_package: "linux-aarch64".into(),
                dtb: None,
                bootloader: BootloaderType::Uefi,
                flash_method: FlashMethod::DirectBlock,
                screen: None,
                boot_fs: FilesystemType::Fat32,
                root_fs: FilesystemType::Ext4,
                boot_size_mb: 256,
                root_size_mb: 3072,
                extra_packages: vec![],
            },
            Self {
                codename: "pine64-pinephone".into(),
                name: "PinePhone".into(),
                manufacturer: "PINE64".into(),
                arch: Architecture::Aarch64,
                soc: "Allwinner A64".into(),
                kernel_package: "linux-pinephone".into(),
                dtb: Some("allwinner/sun50i-a64-pinephone-1.2.dtb".into()),
                bootloader: BootloaderType::UBoot,
                flash_method: FlashMethod::Jumpdrive,
                screen: Some(ScreenConfig {
                    width: 720,
                    height: 1440,
                    dpi: Some(282),
                }),
                boot_fs: FilesystemType::Ext4,
                root_fs: FilesystemType::Ext4,
                boot_size_mb: 256,
                root_size_mb: 4096,
                extra_packages: vec![
                    "u-boot-pinephone".into(),
                    "pinephone-modem-scripts".into(),
                    "rtl8723bt-firmware".into(),
                ],
            },
            Self {
                codename: "pine64-pinetab2".into(),
                name: "PineTab2".into(),
                manufacturer: "PINE64".into(),
                arch: Architecture::Aarch64,
                soc: "Rockchip RK3566".into(),
                kernel_package: "linux-pinetab2".into(),
                dtb: Some("rockchip/rk3566-pinetab2-v2.0.dtb".into()),
                bootloader: BootloaderType::UBoot,
                flash_method: FlashMethod::Jumpdrive,
                screen: Some(ScreenConfig {
                    width: 800,
                    height: 1280,
                    dpi: Some(150),
                }),
                boot_fs: FilesystemType::Ext4,
                root_fs: FilesystemType::Ext4,
                boot_size_mb: 256,
                root_size_mb: 4096,
                extra_packages: vec!["u-boot-pinetab2".into()],
            },
            Self {
                codename: "purism-librem5".into(),
                name: "Librem 5".into(),
                manufacturer: "Purism".into(),
                arch: Architecture::Aarch64,
                soc: "NXP i.MX8MQ".into(),
                kernel_package: "linux-librem5".into(),
                dtb: Some("freescale/imx8mq-librem5-r4.dtb".into()),
                bootloader: BootloaderType::UBoot,
                flash_method: FlashMethod::Fastboot,
                screen: Some(ScreenConfig {
                    width: 720,
                    height: 1440,
                    dpi: Some(282),
                }),
                boot_fs: FilesystemType::Ext4,
                root_fs: FilesystemType::Ext4,
                boot_size_mb: 256,
                root_size_mb: 4096,
                extra_packages: vec!["u-boot-librem5".into()],
            },
            Self {
                codename: "xiaomi-scorpio".into(),
                name: "Mi Note 2 (Scorpio)".into(),
                manufacturer: "Xiaomi".into(),
                arch: Architecture::Aarch64,
                soc: "Qualcomm Snapdragon 821".into(),
                kernel_package: "linux-postmarketos-qcom-msm8996".into(),
                dtb: Some("qcom/msm8996-xiaomi-scorpio.dtb".into()),
                bootloader: BootloaderType::Lk2nd,
                flash_method: FlashMethod::Fastboot,
                screen: Some(ScreenConfig {
                    width: 1080,
                    height: 1920,
                    dpi: Some(386),
                }),
                boot_fs: FilesystemType::Ext4,
                root_fs: FilesystemType::Ext4,
                boot_size_mb: 128,
                root_size_mb: 4096,
                extra_packages: vec!["lk2nd-msm8996".into()],
            },
            Self {
                codename: "raspberry-pi4".into(),
                name: "Raspberry Pi 4 / 400".into(),
                manufacturer: "Raspberry Pi".into(),
                arch: Architecture::Aarch64,
                soc: "Broadcom BCM2711".into(),
                kernel_package: "linux-rpi".into(),
                dtb: Some("broadcom/bcm2711-rpi-4-b.dtb".into()),
                bootloader: BootloaderType::Uefi,
                flash_method: FlashMethod::DirectBlock,
                screen: None,
                boot_fs: FilesystemType::Fat32,
                root_fs: FilesystemType::Ext4,
                boot_size_mb: 256,
                root_size_mb: 4096,
                extra_packages: vec!["raspberrypi-bootloader".into()],
            },
            Self {
                codename: "generic-aarch64".into(),
                name: "Generic aarch64 (UEFI)".into(),
                manufacturer: "Generic".into(),
                arch: Architecture::Aarch64,
                soc: "Generic ARM64".into(),
                kernel_package: "linux-aarch64".into(),
                dtb: None,
                bootloader: BootloaderType::Uefi,
                flash_method: FlashMethod::DirectBlock,
                screen: None,
                boot_fs: FilesystemType::Fat32,
                root_fs: FilesystemType::Ext4,
                boot_size_mb: 256,
                root_size_mb: 4096,
                extra_packages: vec![],
            },
        ]
    }

    pub fn find(codename: &str) -> Option<Self> {
        Self::builtin_profiles()
            .into_iter()
            .find(|d| d.codename == codename)
    }
}
