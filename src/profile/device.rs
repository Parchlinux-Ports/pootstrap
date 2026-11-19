use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::{Path, PathBuf};

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
    #[serde(default)]
    pub base_address: Option<u64>,
    #[serde(default)]
    pub page_size: Option<u32>,
    #[serde(default)]
    pub kernel_cmdline: Option<String>,
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
                base_address: None,
                page_size: None,
                kernel_cmdline: None,
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
                base_address: None,
                page_size: None,
                kernel_cmdline: None,
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
                base_address: None,
                page_size: None,
                kernel_cmdline: None,
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
                base_address: None,
                page_size: None,
                kernel_cmdline: None,
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
                base_address: Some(0x80000000),
                page_size: Some(4096),
                kernel_cmdline: Some("console=ttyMSM0,115200,n8 androidboot.hardware=qcom user_debug=31".into()),
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
                base_address: None,
                page_size: None,
                kernel_cmdline: None,
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
                base_address: None,
                page_size: None,
                kernel_cmdline: None,
            },
        ]
    }

    pub fn validate(&self) -> std::result::Result<(), String> {
        if self.codename.trim().is_empty() {
            return Err("Device codename cannot be empty".into());
        }
        if !self.codename.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
            return Err(format!(
                "Device codename '{}' contains invalid characters (only alphanumeric, hyphen, underscore allowed)",
                self.codename
            ));
        }
        if self.name.trim().is_empty() {
            return Err("Device name cannot be empty".into());
        }
        if self.kernel_package.trim().is_empty() {
            return Err("Kernel package name cannot be empty".into());
        }
        if self.boot_size_mb < 32 {
            return Err(format!(
                "Boot partition size ({} MB) must be at least 32 MB",
                self.boot_size_mb
            ));
        }
        if self.root_size_mb < 512 {
            return Err(format!(
                "Root partition size ({} MB) must be at least 512 MB",
                self.root_size_mb
            ));
        }
        if let Some(ps) = self.page_size.filter(|&ps| ps == 0 || (ps & (ps - 1)) != 0) {
            return Err(format!("Page size ({}) must be a non-zero power of 2", ps));
        }
        Ok(())
    }

    pub fn load_from_file(path: &Path) -> std::result::Result<Self, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read file {:?}: {}", path, e))?;
        let profile: Self = toml::from_str(&content)
            .map_err(|e| format!("Failed to parse device profile {:?}: {}", path, e))?;
        profile.validate()?;
        Ok(profile)
    }

    pub fn discover_profiles() -> Vec<Self> {
        let mut profiles = Vec::new();
        let mut search_dirs = vec![
            PathBuf::from("./devices"),
            PathBuf::from("/etc/pootstrap/devices"),
        ];

        if let Ok(home) = std::env::var("HOME") {
            search_dirs.push(
                PathBuf::from(home)
                    .join(".config")
                    .join("pootstrap")
                    .join("devices"),
            );
        }

        for dir in search_dirs {
            if let Ok(entries) = std::fs::read_dir(&dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() && path.extension().is_some_and(|ext| ext == "toml") {
                        match Self::load_from_file(&path) {
                            Ok(profile) => {
                                tracing::debug!("Loaded external device profile from {:?}", path);
                                profiles.push(profile);
                            }
                            Err(e) => {
                                tracing::warn!("Ignoring invalid device profile {:?}: {}", path, e);
                            }
                        }
                    }
                }
            }
        }

        profiles
    }

    pub fn all_profiles() -> Vec<Self> {
        let external = Self::discover_profiles();
        let builtin = Self::builtin_profiles();

        let mut map: std::collections::HashMap<String, Self> = std::collections::HashMap::new();

        for b in builtin {
            map.insert(b.codename.clone(), b);
        }

        for ext in external {
            map.insert(ext.codename.clone(), ext);
        }

        let mut list: Vec<Self> = map.into_values().collect();
        list.sort_by(|a, b| a.codename.cmp(&b.codename));
        list
    }

    pub fn find(codename: &str) -> Option<Self> {
        Self::all_profiles()
            .into_iter()
            .find(|d| d.codename == codename)
    }
}
