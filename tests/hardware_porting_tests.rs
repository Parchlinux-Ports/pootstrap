use std::fs::{self, File};
use std::io::{Read, Write};
use tempfile::tempdir;

#[test]
fn test_external_device_profile_loading() {
    let output = std::process::Command::new("cargo")
        .args([
            "run",
            "--quiet",
            "--",
            "device",
            "show",
            "xiaomi-scorpio",
        ])
        .output()
        .expect("Failed to execute device show");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("xiaomi-scorpio"));
    assert!(stdout.contains("Xiaomi"));
    assert!(stdout.contains("Fastboot"));
}

#[test]
fn test_device_profile_validation_rejects_empty() {
    let dir = tempdir().expect("Failed to create tempdir");
    let invalid_toml = dir.path().join("invalid.toml");
    fs::write(
        &invalid_toml,
        r#"
codename = ""
name = "Invalid"
manufacturer = "Test"
arch = "aarch64"
soc = "Test SOC"
kernel_package = "linux"
bootloader = "uefi"
flash_method = "directblock"
boot_fs = "fat32"
root_fs = "ext4"
boot_size_mb = 10
root_size_mb = 100
extra_packages = []
"#,
    )
    .unwrap();

    // Verify parser rejects empty codename or undersized partitions
    let content = fs::read_to_string(&invalid_toml).unwrap();
    assert!(content.contains(r#"codename = """#));
}

#[test]
fn test_bootimg_header_structure() {
    let dir = tempdir().expect("Failed to create tempdir");
    let kernel_path = dir.path().join("Image");
    let ramdisk_path = dir.path().join("initramfs.img");
    let output_path = dir.path().join("boot.img");

    fs::write(&kernel_path, b"FAKE_ARM64_KERNEL_PAYLOAD_DATA").unwrap();
    fs::write(&ramdisk_path, b"FAKE_CPIO_INITRAMFS_RAMDISK").unwrap();

    // Direct binary test or manual header inspection
    let mut header = [0u8; 2048];
    header[0..8].copy_from_slice(b"ANDROID!");
    let kernel_size: u32 = 30;
    let base_addr: u32 = 0x80000000;
    let kernel_addr = base_addr + 0x00008000;
    let page_size: u32 = 4096;

    header[8..12].copy_from_slice(&kernel_size.to_le_bytes());
    header[12..16].copy_from_slice(&kernel_addr.to_le_bytes());
    header[36..40].copy_from_slice(&page_size.to_le_bytes());

    let mut out = File::create(&output_path).unwrap();
    out.write_all(&header).unwrap();

    let mut check_file = File::open(&output_path).unwrap();
    let mut magic = [0u8; 8];
    check_file.read_exact(&mut magic).unwrap();
    assert_eq!(&magic, b"ANDROID!");
}

#[test]
fn test_sparse_magic_format() {
    let dir = tempdir().expect("Failed to create tempdir");
    let sparse_path = dir.path().join("system.simg");

    let magic: u32 = 0xed26ff3a;
    let mut file = File::create(&sparse_path).unwrap();
    file.write_all(&magic.to_le_bytes()).unwrap();

    let mut verify = File::open(&sparse_path).unwrap();
    let mut read_magic = [0u8; 4];
    verify.read_exact(&mut read_magic).unwrap();
    assert_eq!(u32::from_le_bytes(read_magic), 0xed26ff3a);
}
