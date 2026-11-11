# Project Roadmap & Planned Enhancements

This document tracks planned architectural features and enhancements for `pootstrap`.

---

## Milestone 1: Development & Debugging Utilities

### Issue 1: Interactive Rootfs Shell (`pootstrap chroot`)
- **Objective**: Provide a quick, isolated shell inside any built rootfs directory for troubleshooting and configuration inspection.
- **Scope**:
  - Automatically bind mount `/proc`, `/sys`, `/dev`, and `/run`.
  - Copy host or valid resolv.conf for internet access.
  - Set up foreign architecture execution via `qemu-aarch64-static` if needed.
  - Drop user into `/bin/bash` with environment variables preserved.
  - Safely unmount all mounts on exit using RAII drop guards.

### Issue 2: External Device Tree Definitions
- **Objective**: Decouple device specifications from the compiled binary.
- **Scope**:
  - Support loading device profiles dynamically from `~/.config/pootstrap/devices/*.toml` or `/etc/pootstrap/devices/`.
  - Allow device port maintainers to submit device configuration files with custom partition schemes, kernels, and extra packages without recompiling `pootstrap`.

---

## Milestone 2: Android Hardware Support

### Issue 3: Android `boot.img` Generation & Fastboot Testing
- **Objective**: Package kernel, initramfs, and device tree blob into standard Android boot images.
- **Scope**:
  - Implement native or tool-assisted `mkbootimg` generation.
  - Support automatic extraction of kernel command lines and base address offsets.
  - Implement `pootstrap flash --method fastboot-boot` to run `fastboot boot boot.img` for non-destructive kernel testing directly in RAM.

### Issue 4: Android Sparse Image Support
- **Objective**: Format system partitions for phones with restricted eMMC/UFS partition sizes.
- **Scope**:
  - Convert raw ext4 images to Android sparse image format (`img2simg`).
  - Add chunking logic so images fit within fastboot buffer boundaries.

---

## Milestone 3: Package Building & Local Repository

### Issue 5: Isolated Foreign Package Builder (`pootstrap pkg build`)
- **Objective**: Compile device-specific kernels, udev rules, and audio configs from PKGBUILDs.
- **Scope**:
  - Isolated clean chroot build environment.
  - Cross-architecture compilation via QEMU user emulation or distcc.
  - Output `.pkg.tar.zst` files into a managed local package directory.

### Issue 6: Managed Local Pacman Repository
- **Objective**: Host locally built packages so `pacstrap` and `pacman` consume them during rootfs builds.
- **Scope**:
  - Maintain a local database (`pootstrap-local.db.tar.gz`) via `repo-add`.
  - Dynamically inject the local repository definition into the generated `pacman.conf`.

---

## Milestone 4: Security & Storage

### Issue 7: Full Disk Encryption (LUKS) with On-Screen Keyboard
- **Objective**: Protect user data on mobile devices with LUKS2 encryption.
- **Scope**:
  - Add `--encrypt` option during `pootstrap image build`.
  - Format root partition as LUKS2 container.
  - Integrate touch-friendly unlockers (`unl0kr` / `osk-sdl`) into the initramfs.

### Issue 8: Completely Unprivileged Rootfs & Image Assembly
- **Objective**: Eliminate remaining `sudo` calls for partition formatting and mounting.
- **Scope**:
  - Investigate `bubblewrap` / Linux user namespaces (`unshare -m -U`) for filesystem assembly.
  - Explore unprivileged disk image creation using tools like `systemd-repart` or `mke2fs -d`.
