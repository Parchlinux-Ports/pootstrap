use std::process::Command;

#[test]
fn test_cli_help() {
    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "--help"])
        .output()
        .expect("Failed to execute pootstrap help");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("pootstrap"));
    assert!(stdout.contains("Parch Linux Mobile"));
    assert!(stdout.contains("device"));
    assert!(stdout.contains("flavour"));
    assert!(stdout.contains("rootfs"));
    assert!(stdout.contains("image"));
}

#[test]
fn test_device_list() {
    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "device", "list"])
        .output()
        .expect("Failed to execute pootstrap device list");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("qemu-aarch64"));
    assert!(stdout.contains("pine64-pinephone"));
    assert!(stdout.contains("purism-librem5"));
    assert!(stdout.contains("xiaomi-scorpio"));
    assert!(stdout.contains("raspberry-pi4"));
}

#[test]
fn test_qemu_device_show() {
    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "device", "show", "qemu-aarch64"])
        .output()
        .expect("Failed to execute pootstrap device show");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("QEMU ARM64 Virtual Machine"));
    assert!(stdout.contains("ARMv8 Virt Platform"));
    assert!(stdout.contains("aarch64"));
}

#[test]
fn test_device_show() {
    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "device", "show", "pine64-pinephone"])
        .output()
        .expect("Failed to execute pootstrap device show");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("PinePhone"));
    assert!(stdout.contains("Allwinner A64"));
    assert!(stdout.contains("linux-pinephone"));
    assert!(stdout.contains("aarch64"));
}

#[test]
fn test_flavour_list() {
    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "flavour", "list"])
        .output()
        .expect("Failed to execute pootstrap flavour list");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("phosh"));
    assert!(stdout.contains("plasma-mobile"));
    assert!(stdout.contains("sxmo"));
    assert!(stdout.contains("minimal"));
}

#[test]
fn test_flavour_show() {
    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "flavour", "show", "phosh"])
        .output()
        .expect("Failed to execute pootstrap flavour show");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Phosh (GNOME Mobile)"));
    assert!(stdout.contains("phosh.service"));
}

#[test]
fn test_binfmt_check() {
    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "binfmt", "check"])
        .output()
        .expect("Failed to execute pootstrap binfmt check");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("binfmt_misc"));
}
