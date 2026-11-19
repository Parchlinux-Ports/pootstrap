mod builder;
mod cli;
mod config;
mod distro;
mod error;
mod flash;
mod interactive;
mod isolation;
mod profile;
mod qemu;

use builder::ImageFormat;
use clap::Parser;
use cli::{
    BinfmtSubcommand, Cli, Commands, ConfigSubcommand, DeviceSubcommand, FlavourSubcommand,
    ImageSubcommand, IsoSubcommand, PkgSubcommand, RepoSubcommand, RootfsSubcommand,
};
use config::Config;
use error::{PootstrapError, Result};
use isolation::ChrootRunner;
use profile::{Architecture, DeviceProfile, FlavourProfile};
use qemu::QemuRunner;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

fn setup_logging(verbose: bool) {
    let filter = if verbose {
        EnvFilter::new("pootstrap=debug,info")
    } else {
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("pootstrap=info"))
    };

    tracing_subscriber::registry()
        .with(filter)
        .with(
            tracing_subscriber::fmt::layer()
                .with_target(false)
                .without_time(),
        )
        .init();
}

fn main() {
    let args = Cli::parse();
    setup_logging(args.verbose);

    if let Err(err) = run(args) {
        tracing::error!("{}", err);
        std::process::exit(1);
    }
}

fn run(args: Cli) -> Result<()> {
    let custom_path = args.config.clone();
    let mut config = match &args.config {
        Some(path) => {
            if path.exists() {
                Config::load_from_path(path)?
            } else {
                Config::default()
            }
        }
        None => Config::load_or_default(),
    };

    match args.command {
        None | Some(Commands::Interactive) => interactive::start_interactive_session(&mut config),
        Some(Commands::Init(init_args)) => handle_init(&config, custom_path, init_args),
        Some(Commands::Config(config_args)) => handle_config(&config, config_args),
        Some(Commands::Device(dev_args)) => handle_device(dev_args),
        Some(Commands::Flavour(flavour_args)) => handle_flavour(flavour_args),
        Some(Commands::Rootfs(rootfs_args)) => handle_rootfs(&config, rootfs_args),
        Some(Commands::Image(image_args)) => handle_image(&config, image_args),
        Some(Commands::Iso(iso_args)) => handle_iso(&config, iso_args),
        Some(Commands::Run(run_args)) => handle_run(&config, run_args),
        Some(Commands::Flash(flash_args)) => handle_flash(&config, flash_args),
        Some(Commands::Chroot(chroot_args)) => handle_chroot(&config, chroot_args),
        Some(Commands::Pkg(pkg_args)) => handle_pkg(&config, pkg_args),
        Some(Commands::Repo(repo_args)) => handle_repo(&config, repo_args),
        Some(Commands::Binfmt(binfmt_args)) => handle_binfmt(binfmt_args),
    }
}

fn handle_init(config: &Config, custom_path: Option<std::path::PathBuf>, args: cli::InitArgs) -> Result<()> {
    let mut new_config = config.clone();

    if let Some(dev) = args.device {
        if DeviceProfile::find(&dev).is_none() {
            return Err(PootstrapError::DeviceNotFound(dev));
        }
        new_config.default_device = dev;
    }

    if let Some(flavour) = args.flavour {
        if FlavourProfile::find(&flavour).is_none() {
            return Err(PootstrapError::FlavourNotFound(flavour));
        }
        new_config.default_flavour = flavour;
    }

    let config_path = custom_path.unwrap_or_else(Config::config_file_path);
    new_config.save(&config_path)?;
    new_config.ensure_directories()?;

    tracing::info!("Initialized pootstrap configuration at {:?}", config_path);
    tracing::info!("Default device: {}", new_config.default_device);
    tracing::info!("Default flavour: {}", new_config.default_flavour);
    Ok(())
}

fn handle_config(config: &Config, args: cli::ConfigArgs) -> Result<()> {
    match args.command.unwrap_or(ConfigSubcommand::Show) {
        ConfigSubcommand::Show => {
            let toml_str = toml::to_string_pretty(config)
                .map_err(|e| PootstrapError::Serialization(e.to_string()))?;
            println!("{}", toml_str);
        }
        ConfigSubcommand::Path => {
            println!("{}", Config::config_file_path().display());
        }
    }
    Ok(())
}

fn handle_device(args: cli::DeviceArgs) -> Result<()> {
    match args.command {
        DeviceSubcommand::List => {
            println!("{:<22} {:<18} {:<10} {:<24}", "CODENAME", "MANUFACTURER", "ARCH", "NAME");
            println!("{}", "-".repeat(78));
            for dev in DeviceProfile::builtin_profiles() {
                println!(
                    "{:<22} {:<18} {:<10} {:<24}",
                    dev.codename,
                    dev.manufacturer,
                    dev.arch.as_str(),
                    dev.name
                );
            }
        }
        DeviceSubcommand::Show { codename } => {
            let dev = DeviceProfile::find(&codename)
                .ok_or(PootstrapError::DeviceNotFound(codename))?;
            println!("Device: {} ({})", dev.name, dev.manufacturer);
            println!("Codename: {}", dev.codename);
            println!("Architecture: {}", dev.arch);
            println!("SoC: {}", dev.soc);
            println!("Kernel package: {}", dev.kernel_package);
            if let Some(dtb) = &dev.dtb {
                println!("Device Tree: {}", dtb);
            }
            println!("Bootloader: {:?}", dev.bootloader);
            println!("Flash method: {:?}", dev.flash_method);
            println!("Boot partition: {} MB ({:?})", dev.boot_size_mb, dev.boot_fs);
            println!("Root partition: {} MB ({:?})", dev.root_size_mb, dev.root_fs);
            if !dev.extra_packages.is_empty() {
                println!("Extra packages: {}", dev.extra_packages.join(", "));
            }
        }
    }
    Ok(())
}

fn handle_flavour(args: cli::FlavourArgs) -> Result<()> {
    match args.command {
        FlavourSubcommand::List => {
            println!("{:<16} {:<24} {:<40}", "NAME", "DISPLAY NAME", "DESCRIPTION");
            println!("{}", "-".repeat(80));
            for f in FlavourProfile::builtin_flavours() {
                println!("{:<16} {:<24} {:<40}", f.name, f.display_name, f.description);
            }
        }
        FlavourSubcommand::Show { name } => {
            let f = FlavourProfile::find(&name)
                .ok_or(PootstrapError::FlavourNotFound(name))?;
            println!("Flavour: {}", f.display_name);
            println!("Description: {}", f.description);
            println!("Packages: {}", f.packages.join(", "));
            println!("Services: {}", f.services.join(", "));
        }
    }
    Ok(())
}

fn handle_rootfs(config: &Config, args: cli::RootfsArgs) -> Result<()> {
    match args.command {
        RootfsSubcommand::Build {
            device,
            flavour,
            output,
        } => {
            let dev_name = device.unwrap_or_else(|| config.default_device.clone());
            let dev = DeviceProfile::find(&dev_name)
                .ok_or(PootstrapError::DeviceNotFound(dev_name))?;

            let flv_name = flavour.unwrap_or_else(|| config.default_flavour.clone());
            let flv = FlavourProfile::find(&flv_name)
                .ok_or(PootstrapError::FlavourNotFound(flv_name))?;

            let out_rootfs = output.unwrap_or_else(|| {
                config
                    .work_dir
                    .join("rootfs")
                    .join(format!("{}-{}", dev.codename, flv.name))
            });

            let builder = builder::RootfsBuilder::new(config, &dev, &flv, out_rootfs);
            builder.build()?;
        }
    }
    Ok(())
}

fn handle_image(config: &Config, args: cli::ImageArgs) -> Result<()> {
    match args.command {
        ImageSubcommand::Build {
            device,
            flavour,
            rootfs,
            output,
            format,
        } => {
            let dev_name = device.unwrap_or_else(|| config.default_device.clone());
            let dev = DeviceProfile::find(&dev_name)
                .ok_or(PootstrapError::DeviceNotFound(dev_name))?;

            let flv_name = flavour.unwrap_or_else(|| config.default_flavour.clone());
            let flv = FlavourProfile::find(&flv_name)
                .ok_or(PootstrapError::FlavourNotFound(flv_name))?;

            let rootfs_dir = rootfs.unwrap_or_else(|| {
                config
                    .work_dir
                    .join("rootfs")
                    .join(format!("{}-{}", dev.codename, flv.name))
            });

            if !builder::is_valid_rootfs(&rootfs_dir) {
                tracing::info!("Rootfs directory {:?} is not populated. Building it first...", rootfs_dir);
                let rootfs_builder = builder::RootfsBuilder::new(config, &dev, &flv, rootfs_dir.clone());
                rootfs_builder.build()?;
            }

            let is_qcow2 = format.to_lowercase() == "qcow2";
            let ext = if is_qcow2 { "qcow2" } else { "img" };

            let output_img = output.unwrap_or_else(|| {
                config
                    .output_dir
                    .join(format!("parch-{}-{}.{}", dev.codename, flv.name, ext))
            });

            let image_format = if is_qcow2 {
                ImageFormat::Qcow2
            } else {
                ImageFormat::Raw
            };

            let image_builder = builder::ImageBuilder::new(&dev, &rootfs_dir, output_img)
                .with_format(image_format);

            image_builder.build()?;
        }
    }
    Ok(())
}

fn handle_iso(config: &Config, args: cli::IsoArgs) -> Result<()> {
    match args.command {
        IsoSubcommand::Build {
            flavour,
            rootfs,
            output,
            volume_id,
        } => {
            let flv_name = flavour.unwrap_or_else(|| config.default_flavour.clone());
            let flv = FlavourProfile::find(&flv_name)
                .ok_or(PootstrapError::FlavourNotFound(flv_name))?;

            let dev = DeviceProfile::find("qemu-aarch64")
                .or_else(|| DeviceProfile::find(&config.default_device))
                .ok_or_else(|| PootstrapError::DeviceNotFound("qemu-aarch64".into()))?;

            let rootfs_dir = rootfs.unwrap_or_else(|| {
                config
                    .work_dir
                    .join("rootfs")
                    .join(format!("{}-{}", dev.codename, flv.name))
            });

            if !builder::is_valid_rootfs(&rootfs_dir) {
                tracing::info!(
                    "Rootfs directory {:?} is not populated. Building it first...",
                    rootfs_dir
                );
                let rootfs_builder =
                    builder::RootfsBuilder::new(config, &dev, &flv, rootfs_dir.clone());
                rootfs_builder.build()?;
            }

            let output_iso = output.unwrap_or_else(|| {
                config
                    .output_dir
                    .join(format!("parch-aarch64-{}.iso", flv.name))
            });

            let iso_builder = builder::IsoBuilder::new(config, &flv, rootfs_dir, output_iso)
                .with_volume_id(volume_id);

            iso_builder.build()?;
        }
    }
    Ok(())
}

fn handle_run(config: &Config, args: cli::RunArgs) -> Result<()> {
    let (image_path, dev_codename, flv_name) = match args.image {
        Some(path) => {
            let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
            let mut detected_device = args.device.clone();
            let mut detected_flavour = args.flavour.clone();
            if let Some(stripped) = stem.strip_prefix("parch-") {
                let parts: Vec<&str> = stripped.split('-').collect();
                if parts.len() >= 2 {
                    if detected_device.is_none() {
                        let dev_part = parts[..parts.len() - 1].join("-");
                        detected_device = Some(dev_part);
                    }
                    if detected_flavour.is_none() {
                        detected_flavour = Some(parts[parts.len() - 1].to_string());
                    }
                }
            }
            let dev = detected_device.unwrap_or_else(|| config.default_device.clone());
            let flv = detected_flavour.unwrap_or_else(|| config.default_flavour.clone());
            (path, dev, flv)
        }
        None => {
            let dev = args.device.clone().unwrap_or_else(|| config.default_device.clone());
            let flv = args.flavour.clone().unwrap_or_else(|| config.default_flavour.clone());

            let candidate_qcow2 = config
                .output_dir
                .join(format!("parch-{}-{}.qcow2", dev, flv));
            let candidate_raw = config
                .output_dir
                .join(format!("parch-{}-{}.img", dev, flv));

            if candidate_qcow2.exists() {
                (candidate_qcow2, dev, flv)
            } else if candidate_raw.exists() {
                (candidate_raw, dev, flv)
            } else {
                let mut found = None;
                if let Ok(entries) = std::fs::read_dir(&config.output_dir) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        let has_valid_ext = path
                            .extension()
                            .and_then(|e| e.to_str())
                            .is_some_and(|ext| ext == "qcow2" || ext == "img");
                        let has_parch_prefix = path
                            .file_name()
                            .and_then(|n| n.to_str())
                            .is_some_and(|n| n.starts_with("parch-"));

                        if has_valid_ext && has_parch_prefix {
                            let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                            if let Some(stripped) = stem.strip_prefix("parch-") {
                                let parts: Vec<&str> = stripped.split('-').collect();
                                if parts.len() >= 2 {
                                    let d = parts[..parts.len() - 1].join("-");
                                    let f = parts[parts.len() - 1].to_string();
                                    found = Some((path, d, f));
                                    break;
                                }
                            }
                        }
                    }
                }

                match found {
                    Some((path, d, f)) => {
                        tracing::info!("Auto-detected built image at {:?}", path);
                        (path, d, f)
                    }
                    None => {
                        return Err(PootstrapError::ImageBuild(format!(
                            "No built image found for {}-{}. Run 'pootstrap image build' first.",
                            dev, flv
                        )));
                    }
                }
            }
        }
    };

    let (kernel, initrd) = QemuRunner::find_kernel_and_initrd(
        &config.work_dir,
        &dev_codename,
        &flv_name,
    );
    QemuRunner::launch(&image_path, kernel.as_deref(), initrd.as_deref(), args.ram, args.cpus)?;
    Ok(())
}

fn handle_flash(_config: &Config, args: cli::FlashArgs) -> Result<()> {
    let method = args.method.unwrap_or_else(|| "fastboot".to_string());

    match method.to_lowercase().as_str() {
        "fastboot" => {
            flash::FastbootFlasher::flash_partition("userdata", &args.image)?;
        }
        "jumpdrive" => {
            let target = match args.target {
                Some(t) => t,
                None => {
                    let candidates = flash::JumpdriveFlasher::find_jumpdrive_devices()?;
                    if candidates.is_empty() {
                        return Err(PootstrapError::FlashError(
                            "No Jumpdrive device detected. Specify target manually via --target /dev/sdX".into(),
                        ));
                    }
                    candidates[0].clone()
                }
            };
            flash::JumpdriveFlasher::flash_block_device(&args.image, &target)?;
        }
        other => {
            return Err(PootstrapError::FlashError(format!(
                "Unsupported flash method '{}'. Supported: fastboot, jumpdrive",
                other
            )));
        }
    }
    Ok(())
}

fn handle_chroot(config: &Config, args: cli::ChrootArgs) -> Result<()> {
    let dev = args.device.unwrap_or_else(|| config.default_device.clone());
    let flv = args.flavour.unwrap_or_else(|| config.default_flavour.clone());

    let rootfs_dir = match args.rootfs {
        Some(p) => p,
        None => {
            let candidate = config
                .work_dir
                .join("rootfs")
                .join(format!("{}-{}", dev, flv));
            if candidate.exists() {
                candidate
            } else {
                let rootfs_parent = config.work_dir.join("rootfs");
                let mut found = None;
                if let Ok(entries) = std::fs::read_dir(&rootfs_parent) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.is_dir() && builder::is_valid_rootfs(&path) {
                            found = Some(path);
                            break;
                        }
                    }
                }
                match found {
                    Some(f) => {
                        tracing::info!("Auto-detected rootfs directory at {:?}", f);
                        f
                    }
                    None => candidate,
                }
            }
        }
    };

    if !rootfs_dir.exists() {
        return Err(PootstrapError::RootfsBuild(format!(
            "Rootfs directory {:?} does not exist. Run 'pootstrap rootfs build' first.",
            rootfs_dir
        )));
    }

    ChrootRunner::run_interactive_shell(&rootfs_dir)?;
    Ok(())
}

fn handle_pkg(config: &Config, args: cli::PkgArgs) -> Result<()> {
    match args.command {
        PkgSubcommand::Build { path, arch } => {
            let pkg_dir = path.unwrap_or_else(|| {
                std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."))
            });
            let target_arch = match arch.to_lowercase().as_str() {
                "aarch64" => Architecture::Aarch64,
                "armv7h" => Architecture::Armv7h,
                "x86_64" => Architecture::X86_64,
                other => {
                    return Err(PootstrapError::Config(format!(
                        "Unsupported architecture '{}'. Supported: aarch64, armv7h, x86_64",
                        other
                    )));
                }
            };

            let pkg_builder = builder::PackageBuilder::new(config, pkg_dir, target_arch);
            let packages = pkg_builder.build()?;
            println!("\nSuccessfully built {} package(s):", packages.len());
            for p in packages {
                println!("  - {:?}", p);
            }
            println!();
        }
    }
    Ok(())
}

fn handle_repo(config: &Config, args: cli::RepoArgs) -> Result<()> {
    match args.command {
        RepoSubcommand::Update { arch } => {
            let repo_dir = config.work_dir.join("repo").join(&arch);
            let db_path = distro::LocalRepoManager::update_repo(&repo_dir, None)?;
            println!("Local repository updated at {:?}", db_path);
        }
        RepoSubcommand::List { arch } => {
            let repo_dir = config.work_dir.join("repo").join(&arch);
            let packages = distro::LocalRepoManager::list_packages(&repo_dir)?;
            println!("Local packages in {:?} ({} total):", repo_dir, packages.len());
            for p in packages {
                if let Some(name) = p.file_name().and_then(|n| n.to_str()) {
                    println!("  {}", name);
                }
            }
        }
    }
    Ok(())
}

fn handle_binfmt(args: cli::BinfmtArgs) -> Result<()> {
    match args.command {
        BinfmtSubcommand::Check => {
            println!("Checking binfmt_misc kernel support...");
            let binfmt_mounted = isolation::BinfmtChecker::is_binfmt_misc_mounted();
            println!(
                "  /proc/sys/fs/binfmt_misc: {}",
                if binfmt_mounted { "Mounted" } else { "Not mounted" }
            );

            for arch in [Architecture::Aarch64, Architecture::Armv7h] {
                let bin = isolation::BinfmtChecker::find_qemu_static(arch);
                let supported = isolation::BinfmtChecker::check_support(arch).is_ok();
                match bin {
                    Some(path) => println!("  {:<8}: Available at {:?} (ready: {})", arch, path, supported),
                    None => println!("  {:<8}: Not found (needs {}, ready: {})", arch, arch.qemu_binary(), supported),
                }
            }
        }
    }
    Ok(())
}
