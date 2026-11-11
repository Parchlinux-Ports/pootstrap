use crate::error::{PootstrapError, Result};
use crate::isolation::PrivilegedRunner;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub cache_dir: PathBuf,
    pub work_dir: PathBuf,
    pub output_dir: PathBuf,
    pub default_device: String,
    pub default_flavour: String,
    pub hostname: String,
    pub default_user: String,
    #[serde(default)]
    pub default_password: Option<String>,
    pub alarm_mirror: String,
    pub parch_mirror: String,
    pub x86_64_mirror: String,
}

impl Default for Config {
    fn default() -> Self {
        let (cache_dir, work_dir, output_dir) = if let Ok(home) = std::env::var("HOME") {
            let home_path = PathBuf::from(home);
            (
                home_path.join(".cache").join("pootstrap"),
                home_path
                    .join(".local")
                    .join("share")
                    .join("pootstrap")
                    .join("work"),
                home_path
                    .join(".local")
                    .join("share")
                    .join("pootstrap")
                    .join("output"),
            )
        } else {
            (
                PathBuf::from("/var/cache/pootstrap"),
                PathBuf::from("/var/lib/pootstrap/work"),
                PathBuf::from("./output"),
            )
        };

        Self {
            cache_dir,
            work_dir,
            output_dir,
            default_device: "pine64-pinephone".into(),
            default_flavour: "phosh".into(),
            hostname: "parch-mobile".into(),
            default_user: "parch".into(),
            default_password: None,
            alarm_mirror: "http://mirror.archlinuxarm.org/$arch/$repo".into(),
            parch_mirror: "https://repo.parchlinux.com/$arch".into(),
            x86_64_mirror: "https://geo.mirror.pkgbuild.com/$repo/os/$arch".into(),
        }
    }
}

impl Config {
    pub fn config_file_path() -> PathBuf {
        if let Ok(config_home) = std::env::var("XDG_CONFIG_HOME") {
            PathBuf::from(config_home).join("pootstrap").join("config.toml")
        } else if let Ok(home) = std::env::var("HOME") {
            PathBuf::from(home)
                .join(".config")
                .join("pootstrap")
                .join("config.toml")
        } else {
            PathBuf::from("/etc/pootstrap/config.toml")
        }
    }

    pub fn load_or_default() -> Self {
        let path = Self::config_file_path();
        if path.exists() {
            match Self::load_from_path(&path) {
                Ok(cfg) => cfg,
                Err(err) => {
                    tracing::warn!("Failed to load config from {:?}: {}. Using defaults.", path, err);
                    Self::default()
                }
            }
        } else {
            Self::default()
        }
    }

    pub fn load_from_path(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)
            .map_err(|e| PootstrapError::Config(format!("Failed to parse TOML config: {}", e)))?;
        Ok(config)
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent).or_else(|_| PrivilegedRunner::create_dir_all(parent));
        }
        let content = toml::to_string_pretty(self)
            .map_err(|e| PootstrapError::Serialization(e.to_string()))?;

        if fs::write(path, &content).is_err() {
            PrivilegedRunner::write_file(path, &content)?;
        }
        Ok(())
    }

    pub fn ensure_directories(&self) -> Result<()> {
        for dir in [&self.cache_dir, &self.work_dir, &self.output_dir] {
            if fs::create_dir_all(dir).is_err() {
                PrivilegedRunner::create_dir_all(dir)?;
            }
        }
        Ok(())
    }
}
