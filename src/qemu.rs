use crate::error::{PootstrapError, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct QemuRunner;

impl QemuRunner {
    pub fn find_efi_firmware() -> Option<PathBuf> {
        let candidates = [
            "/usr/share/edk2/aarch64/QEMU_EFI.fd",
            "/usr/share/qemu/edk2-aarch64-code.fd",
            "/usr/share/edk2-arm/QEMU_EFI.fd",
        ];
        for path in candidates {
            let p = PathBuf::from(path);
            if p.exists() {
                return Some(p);
            }
        }
        None
    }

    pub fn find_kernel_and_initrd(
        work_dir: &Path,
        device: &str,
        flavour: &str,
    ) -> (Option<PathBuf>, Option<PathBuf>) {
        let boot_dir = work_dir
            .join("rootfs")
            .join(format!("{}-{}", device, flavour))
            .join("boot");

        let kernel = [boot_dir.join("Image"), boot_dir.join("vmlinuz-linux")]
            .into_iter()
            .find(|p| p.exists());

        let initrd = [boot_dir.join("initramfs-linux.img"), boot_dir.join("initrd.img")]
            .into_iter()
            .find(|p| p.exists());

        (kernel, initrd)
    }

    pub fn launch(
        image_path: &Path,
        kernel: Option<&Path>,
        initrd: Option<&Path>,
        ram_mb: u32,
        cpus: u32,
    ) -> Result<()> {
        which::which("qemu-system-aarch64").map_err(|_| PootstrapError::MissingTool {
            tool: "qemu-system-aarch64".into(),
        })?;

        if let Some(i) = initrd.filter(|p| std::fs::File::open(p).is_err()) {
            let _ = crate::isolation::PrivilegedRunner::command("chmod")
                .arg("a+r")
                .arg(i)
                .status();
        }

        if let Some(k) = kernel.filter(|p| std::fs::File::open(p).is_err()) {
            let _ = crate::isolation::PrivilegedRunner::command("chmod")
                .arg("a+r")
                .arg(k)
                .status();
        }

        let mut cmd = Command::new("qemu-system-aarch64");
        cmd.arg("-M")
            .arg("virt")
            .arg("-cpu")
            .arg("cortex-a57")
            .arg("-m")
            .arg(format!("{}M", ram_mb))
            .arg("-smp")
            .arg(cpus.to_string());

        if let (Some(k), Some(i)) = (kernel, initrd) {
            tracing::info!("Booting directly with kernel {:?} and initrd {:?}", k, i);
            cmd.arg("-kernel")
                .arg(k)
                .arg("-initrd")
                .arg(i)
                .arg("-append")
                .arg("console=ttyAMA0 root=/dev/vda2 rw rootwait");
        } else if let Some(efi) = Self::find_efi_firmware() {
            cmd.arg("-bios").arg(efi);
        } else {
            tracing::warn!("No direct kernel or UEFI firmware found. QEMU will attempt default boot.");
        }

        let is_qcow2 = image_path.extension().is_some_and(|ext| ext == "qcow2");
        let format_str = if is_qcow2 { "qcow2" } else { "raw" };

        cmd.arg("-drive")
            .arg(format!("file={},if=virtio,format={}", image_path.display(), format_str));

        cmd.arg("-net").arg("nic,model=virtio")
            .arg("-net").arg("user,hostfwd=tcp::2222-:22");

        cmd.arg("-nographic");

        tracing::info!("Launching QEMU ARM64 VM with image {:?}", image_path);
        println!("\n==============================================================");
        println!("             Parch Linux ARM64 QEMU Console                   ");
        println!("==============================================================");
        println!("  - Serial console connected directly to terminal.");
        println!("  - Default logins: 'alarm' (pass: 'alarm') or 'root' (pass: 'root')");
        println!("  - SSH port forwarded: localhost:2222 -> ssh -p 2222 alarm@localhost");
        println!("  - To exit QEMU: Press 'Ctrl + A' then release and press 'X'");
        println!("==============================================================\n");

        let status = cmd.status().map_err(PootstrapError::Io)?;
        if !status.success() {
            tracing::warn!("QEMU exited with code {:?}", status.code());
        }
        Ok(())
    }
}
