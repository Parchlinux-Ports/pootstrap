use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
#[allow(dead_code)]
pub enum PootstrapError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Command execution failed: {command}\nExit code: {exit_code:?}\nOutput: {stderr}")]
    CommandFailed {
        command: String,
        exit_code: Option<i32>,
        stderr: String,
    },

    #[error("Missing required host tool: '{tool}'. Please install it first.")]
    MissingTool { tool: String },

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Unknown device '{0}'. Run 'pootstrap device list' to see available devices.")]
    DeviceNotFound(String),

    #[error("Unknown flavour '{0}'. Run 'pootstrap flavour list' to see available flavours.")]
    FlavourNotFound(String),

    #[error("Architecture error: {0}")]
    Architecture(String),

    #[error("Mount failure at '{path}': {reason}")]
    MountError { path: PathBuf, reason: String },

    #[error("Unmount failure at '{path}': {reason}")]
    UnmountError { path: PathBuf, reason: String },

    #[error("Rootfs build error: {0}")]
    RootfsBuild(String),

    #[error("Disk image creation error: {0}")]
    ImageBuild(String),

    #[error("Flashing error: {0}")]
    FlashError(String),

    #[error("Root privileges required: {0}. Please run with sudo.")]
    PrivilegeRequired(String),

    #[error("Serialization error: {0}")]
    Serialization(String),
}

pub type Result<T> = std::result::Result<T, PootstrapError>;
