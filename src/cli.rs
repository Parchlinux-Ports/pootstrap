use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "pootstrap",
    author = "Parch Linux Developers",
    version = "0.1.0",
    about = "Clean-room bootstrap and image builder for Parch Linux Mobile"
)]
pub struct Cli {
    #[arg(short, long, global = true, help = "Custom configuration file path")]
    pub config: Option<PathBuf>,

    #[arg(short, long, global = true, help = "Enable verbose debug logging")]
    pub verbose: bool,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    #[command(about = "Start interactive device selection and build session")]
    Interactive,

    #[command(about = "Initialize user configuration and workspaces")]
    Init(InitArgs),

    #[command(about = "Manage configuration settings")]
    Config(ConfigArgs),

    #[command(about = "Manage supported mobile devices")]
    Device(DeviceArgs),

    #[command(about = "Manage supported mobile desktop flavours")]
    Flavour(FlavourArgs),

    #[command(about = "Build root filesystem")]
    Rootfs(RootfsArgs),

    #[command(about = "Build flashable or virtual disk image (raw or qcow2)")]
    Image(ImageArgs),

    #[command(about = "Launch built disk image in QEMU virtual machine")]
    Run(RunArgs),

    #[command(about = "Flash image to connected hardware")]
    Flash(FlashArgs),

    #[command(about = "Enter interactive chroot shell in rootfs")]
    Chroot(ChrootArgs),

    #[command(about = "Check foreign architecture binfmt_misc emulation")]
    Binfmt(BinfmtArgs),
}

#[derive(Args, Debug)]
pub struct InitArgs {
    #[arg(long, help = "Default device codename (e.g. qemu-aarch64, pine64-pinephone)")]
    pub device: Option<String>,

    #[arg(long, help = "Default flavour (e.g. phosh, minimal)")]
    pub flavour: Option<String>,
}

#[derive(Args, Debug)]
pub struct ConfigArgs {
    #[command(subcommand)]
    pub command: Option<ConfigSubcommand>,
}

#[derive(Subcommand, Debug)]
pub enum ConfigSubcommand {
    #[command(about = "Display active configuration")]
    Show,
    #[command(about = "Display path to configuration file")]
    Path,
}

#[derive(Args, Debug)]
pub struct DeviceArgs {
    #[command(subcommand)]
    pub command: DeviceSubcommand,
}

#[derive(Subcommand, Debug)]
pub enum DeviceSubcommand {
    #[command(about = "List all supported device profiles")]
    List,
    #[command(about = "Show detailed specifications for a device")]
    Show {
        #[arg(help = "Device codename (e.g. qemu-aarch64, pine64-pinephone)")]
        codename: String,
    },
}

#[derive(Args, Debug)]
pub struct FlavourArgs {
    #[command(subcommand)]
    pub command: FlavourSubcommand,
}

#[derive(Subcommand, Debug)]
pub enum FlavourSubcommand {
    #[command(about = "List all available mobile UI flavours")]
    List,
    #[command(about = "Show packages and services for a flavour")]
    Show {
        #[arg(help = "Flavour name (e.g. phosh, plasma-mobile, sxmo, minimal)")]
        name: String,
    },
}

#[derive(Args, Debug)]
pub struct RootfsArgs {
    #[command(subcommand)]
    pub command: RootfsSubcommand,
}

#[derive(Subcommand, Debug)]
pub enum RootfsSubcommand {
    #[command(about = "Build a rootfs for a target device and flavour")]
    Build {
        #[arg(short, long, help = "Target device codename")]
        device: Option<String>,

        #[arg(short, long, help = "Target flavour")]
        flavour: Option<String>,

        #[arg(short, long, help = "Output directory for the rootfs")]
        output: Option<PathBuf>,
    },
}

#[derive(Args, Debug)]
pub struct ImageArgs {
    #[command(subcommand)]
    pub command: ImageSubcommand,
}

#[derive(Subcommand, Debug)]
pub enum ImageSubcommand {
    #[command(about = "Build a disk image from rootfs")]
    Build {
        #[arg(short, long, help = "Target device codename")]
        device: Option<String>,

        #[arg(short, long, help = "Target flavour")]
        flavour: Option<String>,

        #[arg(short, long, help = "Path to pre-existing rootfs directory")]
        rootfs: Option<PathBuf>,

        #[arg(short, long, help = "Output disk image file path (.img or .qcow2)")]
        output: Option<PathBuf>,

        #[arg(long, default_value = "qcow2", help = "Image format: qcow2 or raw")]
        format: String,
    },
}

#[derive(Args, Debug)]
pub struct RunArgs {
    #[arg(short, long, help = "Path to disk image (.qcow2 or .img)")]
    pub image: Option<PathBuf>,

    #[arg(short, long, help = "Target device codename (e.g. qemu-aarch64)")]
    pub device: Option<String>,

    #[arg(short, long, help = "Target flavour (e.g. minimal)")]
    pub flavour: Option<String>,

    #[arg(short, long, default_value_t = 2048, help = "RAM size in MB")]
    pub ram: u32,

    #[arg(short, long, default_value_t = 2, help = "CPU core count")]
    pub cpus: u32,
}

#[derive(Args, Debug)]
pub struct FlashArgs {
    #[arg(short, long, help = "Flashing method (fastboot or jumpdrive)")]
    pub method: Option<String>,

    #[arg(short, long, help = "Target device codename")]
    pub device: Option<String>,

    #[arg(short, long, help = "Path to the disk image to flash")]
    pub image: PathBuf,

    #[arg(short, long, help = "Target block device for Jumpdrive (e.g. /dev/sdb)")]
    pub target: Option<PathBuf>,
}

#[derive(Args, Debug)]
pub struct ChrootArgs {
    #[arg(short, long, help = "Path to rootfs directory")]
    pub rootfs: Option<PathBuf>,
}

#[derive(Args, Debug)]
pub struct BinfmtArgs {
    #[command(subcommand)]
    pub command: BinfmtSubcommand,
}

#[derive(Subcommand, Debug)]
pub enum BinfmtSubcommand {
    #[command(about = "Check host binfmt_misc and QEMU user static emulation status")]
    Check,
}
