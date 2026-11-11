use crate::builder::{ImageBuilder, ImageFormat, RootfsBuilder};
use crate::config::Config;
use crate::error::{PootstrapError, Result};
use crate::isolation::ChrootRunner;
use crate::profile::{DeviceProfile, FlavourProfile};
use crate::qemu::QemuRunner;
use dialoguer::{theme::ColorfulTheme, Confirm, Input, Password, Select};

pub fn start_interactive_session(config: &mut Config) -> Result<()> {
    let theme = ColorfulTheme::default();

    println!("\n==============================================================");
    println!("          Parch Linux Mobile Bootstrapper (pootstrap)         ");
    println!("==============================================================\n");

    loop {
        let menu_items = vec![
            format!("Select Target Device (currently: {})", config.default_device),
            format!("Select UI Flavour (currently: {})", config.default_flavour),
            format!("Configure User & Password (currently: {})", config.default_user),
            "Build ARM64 QCOW2 Image (for QEMU / KVM)".to_string(),
            "Build Raw Disk Image (.img)".to_string(),
            "Build Rootfs Directory".to_string(),
            "Launch in QEMU VM".to_string(),
            "Enter Chroot Shell".to_string(),
            "Exit".to_string(),
        ];

        let selection = Select::with_theme(&theme)
            .with_prompt("Select an action to proceed")
            .items(&menu_items)
            .default(0)
            .interact()
            .map_err(|e| PootstrapError::Config(e.to_string()))?;

        match selection {
            0 => select_device(config, &theme)?,
            1 => select_flavour(config, &theme)?,
            2 => configure_user_workflow(config, &theme)?,
            3 => build_qcow2_workflow(config, &theme)?,
            4 => build_raw_image_workflow(config, &theme)?,
            5 => build_rootfs_workflow(config, &theme)?,
            6 => launch_qemu_workflow(config)?,
            7 => enter_chroot_workflow(config)?,
            _ => {
                println!("Exiting pootstrap. Goodbye!");
                break;
            }
        }
    }

    Ok(())
}

fn select_device(config: &mut Config, theme: &ColorfulTheme) -> Result<()> {
    let devices = DeviceProfile::builtin_profiles();
    let items: Vec<String> = devices
        .iter()
        .map(|d| format!("{:<22} - {} ({})", d.codename, d.name, d.soc))
        .collect();

    let selection = Select::with_theme(theme)
        .with_prompt("Choose target mobile/virtual hardware device")
        .items(&items)
        .default(0)
        .interact()
        .map_err(|e| PootstrapError::Config(e.to_string()))?;

    let chosen = &devices[selection];
    config.default_device = chosen.codename.clone();

    let config_path = Config::config_file_path();
    config.save(&config_path)?;
    println!("\nDevice set to '{}'. Configuration saved.\n", chosen.name);
    Ok(())
}

fn select_flavour(config: &mut Config, theme: &ColorfulTheme) -> Result<()> {
    let flavours = FlavourProfile::builtin_flavours();
    let items: Vec<String> = flavours
        .iter()
        .map(|f| format!("{:<16} - {}", f.name, f.display_name))
        .collect();

    let selection = Select::with_theme(theme)
        .with_prompt("Choose target UI flavour")
        .items(&items)
        .default(0)
        .interact()
        .map_err(|e| PootstrapError::Config(e.to_string()))?;

    let chosen = &flavours[selection];
    config.default_flavour = chosen.name.clone();

    let config_path = Config::config_file_path();
    config.save(&config_path)?;
    println!("\nFlavour set to '{}'. Configuration saved.\n", chosen.display_name);
    Ok(())
}

fn configure_user_workflow(config: &mut Config, theme: &ColorfulTheme) -> Result<()> {
    let username: String = Input::with_theme(theme)
        .with_prompt("Enter username for the target system")
        .default(config.default_user.clone())
        .interact_text()
        .map_err(|e| PootstrapError::Config(e.to_string()))?;

    let password = Password::with_theme(theme)
        .with_prompt(format!("Enter password for user '{}'", username))
        .with_confirmation("Confirm password", "Passwords do not match. Try again.")
        .interact()
        .map_err(|e| PootstrapError::Config(e.to_string()))?;

    config.default_user = username;
    config.default_password = Some(password);

    let config_path = Config::config_file_path();
    config.save(&config_path)?;
    println!("\nUser credentials configured. Configuration saved.\n");
    Ok(())
}

fn ensure_user_credentials(config: &mut Config, theme: &ColorfulTheme) -> Result<()> {
    if config.default_password.is_none() {
        println!("\nNo target user password configured yet.");
        configure_user_workflow(config, theme)?;
    }
    Ok(())
}

fn build_qcow2_workflow(config: &mut Config, theme: &ColorfulTheme) -> Result<()> {
    ensure_user_credentials(config, theme)?;

    let dev = DeviceProfile::find(&config.default_device)
        .ok_or(PootstrapError::DeviceNotFound(config.default_device.clone()))?;
    let flv = FlavourProfile::find(&config.default_flavour)
        .ok_or(PootstrapError::FlavourNotFound(config.default_flavour.clone()))?;

    println!("\nPreparing to build QCOW2 image:");
    println!("  Target Device:  {} ({})", dev.name, dev.arch);
    println!("  UI Flavour:     {}", flv.display_name);
    println!("  User Account:   {}", config.default_user);
    println!("  Output format:  QEMU QCOW2\n");

    let rootfs_dir = config
        .work_dir
        .join("rootfs")
        .join(format!("{}-{}", dev.codename, flv.name));

    if !crate::builder::is_valid_rootfs(&rootfs_dir) {
        println!("Rootfs directory {:?} is not populated. Building rootfs...", rootfs_dir);
        let builder = RootfsBuilder::new(config, &dev, &flv, rootfs_dir.clone());
        builder.build()?;
    }

    let output_qcow2 = config
        .output_dir
        .join(format!("parch-{}-{}.qcow2", dev.codename, flv.name));

    let image_builder = ImageBuilder::new(&dev, &rootfs_dir, output_qcow2.clone())
        .with_format(ImageFormat::Qcow2);

    let final_path = image_builder.build()?;
    println!("\nSuccess! QCOW2 image generated at: {:?}\n", final_path);

    let should_launch = Confirm::with_theme(theme)
        .with_prompt("Would you like to test boot this image in QEMU now?")
        .default(true)
        .interact()
        .unwrap_or(false);

    if should_launch {
        let (kernel, initrd) = QemuRunner::find_kernel_and_initrd(
            &config.work_dir,
            &dev.codename,
            &flv.name,
        );
        QemuRunner::launch(&final_path, kernel.as_deref(), initrd.as_deref(), 2048, 2)?;
    }

    Ok(())
}

fn build_raw_image_workflow(config: &mut Config, theme: &ColorfulTheme) -> Result<()> {
    ensure_user_credentials(config, theme)?;

    let dev = DeviceProfile::find(&config.default_device)
        .ok_or(PootstrapError::DeviceNotFound(config.default_device.clone()))?;
    let flv = FlavourProfile::find(&config.default_flavour)
        .ok_or(PootstrapError::FlavourNotFound(config.default_flavour.clone()))?;

    let rootfs_dir = config
        .work_dir
        .join("rootfs")
        .join(format!("{}-{}", dev.codename, flv.name));

    if !crate::builder::is_valid_rootfs(&rootfs_dir) {
        println!("Building rootfs for {}...", dev.name);
        let builder = RootfsBuilder::new(config, &dev, &flv, rootfs_dir.clone());
        builder.build()?;
    }

    let output_raw = config
        .output_dir
        .join(format!("parch-{}-{}.img", dev.codename, flv.name));

    let image_builder = ImageBuilder::new(&dev, &rootfs_dir, output_raw);
    let final_path = image_builder.build()?;
    println!("\nSuccess! Disk image generated at: {:?}\n", final_path);
    Ok(())
}

fn build_rootfs_workflow(config: &mut Config, theme: &ColorfulTheme) -> Result<()> {
    ensure_user_credentials(config, theme)?;

    let dev = DeviceProfile::find(&config.default_device)
        .ok_or(PootstrapError::DeviceNotFound(config.default_device.clone()))?;
    let flv = FlavourProfile::find(&config.default_flavour)
        .ok_or(PootstrapError::FlavourNotFound(config.default_flavour.clone()))?;

    let rootfs_dir = config
        .work_dir
        .join("rootfs")
        .join(format!("{}-{}", dev.codename, flv.name));

    let builder = RootfsBuilder::new(config, &dev, &flv, rootfs_dir.clone());
    builder.build()?;
    println!("\nSuccess! Rootfs built at: {:?}\n", rootfs_dir);
    Ok(())
}

fn launch_qemu_workflow(config: &Config) -> Result<()> {
    let dev = DeviceProfile::find(&config.default_device)
        .ok_or(PootstrapError::DeviceNotFound(config.default_device.clone()))?;
    let flv = FlavourProfile::find(&config.default_flavour)
        .ok_or(PootstrapError::FlavourNotFound(config.default_flavour.clone()))?;

    let candidate_qcow2 = config
        .output_dir
        .join(format!("parch-{}-{}.qcow2", dev.codename, flv.name));

    let candidate_raw = config
        .output_dir
        .join(format!("parch-{}-{}.img", dev.codename, flv.name));

    let image_path = if candidate_qcow2.exists() {
        candidate_qcow2
    } else if candidate_raw.exists() {
        candidate_raw
    } else {
        return Err(PootstrapError::ImageBuild(format!(
            "No disk image found for {}-{}. Please build it first.",
            dev.codename, flv.name
        )));
    };

    let (kernel, initrd) = QemuRunner::find_kernel_and_initrd(
        &config.work_dir,
        &dev.codename,
        &flv.name,
    );
    QemuRunner::launch(&image_path, kernel.as_deref(), initrd.as_deref(), 2048, 2)?;
    Ok(())
}

fn enter_chroot_workflow(config: &Config) -> Result<()> {
    let dev = DeviceProfile::find(&config.default_device)
        .ok_or(PootstrapError::DeviceNotFound(config.default_device.clone()))?;
    let flv = FlavourProfile::find(&config.default_flavour)
        .ok_or(PootstrapError::FlavourNotFound(config.default_flavour.clone()))?;

    let rootfs_dir = config
        .work_dir
        .join("rootfs")
        .join(format!("{}-{}", dev.codename, flv.name));

    if !rootfs_dir.exists() {
        return Err(PootstrapError::RootfsBuild(format!(
            "Rootfs directory {:?} does not exist. Please build it first.",
            rootfs_dir
        )));
    }

    ChrootRunner::run_interactive_shell(&rootfs_dir)?;
    Ok(())
}
