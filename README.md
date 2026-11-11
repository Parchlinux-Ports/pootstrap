# pootstrap

A clean-room mobile bootstrap and system image builder for Parch Linux, written in Rust.

`pootstrap` simplifies the process of creating, configuring, and testing bootable mobile Linux root filesystems and disk images for smartphones, tablets, SBCs, and virtual machines.

---

## Features

- **Non-Root Friendly**: Runs under regular user privileges, calling elevated privileges (`sudo`) only when strictly necessary for disk partitioning and loop mounts.
- **Dual Mode Operation**:
  - Interactive terminal UI (TUI) powered by `dialoguer` for guided device selection, flavour choosing, and one-click builds.
  - Full CLI interface with scriptable subcommands for automated CI/CD pipelines.
- **Built-in QEMU Virtualization**: Instant test booting of aarch64 images with virtio networking, serial console, and direct kernel booting.
- **Multiple Mobile Environments**: Out of the box profiles for Phosh, Plasma Mobile, Sxmo, and Minimal base installations.
- **Clean Architecture**: Single compiled Rust binary with no Python or interpreter runtime dependencies.
- **Hardware Flashing**: Fastboot partition flashing and Jumpdrive mass-storage disk writer integrations.

---

## Supported Targets

### Devices
- `qemu-aarch64`: Generic ARM64 Virtual Machine (QEMU `virt` machine)
- `pine64-pinephone`: Pine64 PinePhone (Allwinner A64)
- `pine64-pinetab2`: Pine64 PineTab2 (Rockchip RK3566)
- `purism-librem5`: Purism Librem 5 (NXP i.MX8MQ)
- `xiaomi-scorpio`: Xiaomi Mi Note 2 (Snapdragon 821)
- `raspberry-pi4`: Raspberry Pi 4 Model B (BCM2711)
- `generic-aarch64`: Generic 64-bit ARM target

### UI Flavours
- `phosh`: Wayland shell for mobile based on GNOME technologies
- `plasma-mobile`: KDE Plasma touch-friendly mobile workspace
- `sxmo`: Simple X/Wayland Mobile environment controlled via hardware buttons and gestures
- `minimal`: Lightweight console-only installation for development and debugging

---

## System Requirements

To build and run target images, ensure the following host tools are available:

- **Rust toolchain** (1.80+ or latest stable)
- **Host utilities**: `curl`, `tar`, `parted`, `dosfstools`, `e2fsprogs`, `qemu-img`
- **Optional (for QEMU testing)**: `qemu-system-aarch64`
- **Optional (for device flashing)**: `fastboot`, `android-tools`

---

## Installation & Building

Clone the repository and build using Cargo:

```bash
git clone https://github.com/Parchlinux-Ports/pootstrap.git
cd pootstrap
cargo build --release
```

The compiled binary will be located at `./target/release/pootstrap`.

---

## Usage

### Interactive Mode

Simply run the binary without arguments to launch the interactive setup menu:

```bash
./target/release/pootstrap
```

From here you can select your target device, pick a desktop environment, build the filesystem, generate disk images, and test boot them in QEMU.

---

### Command-Line Interface

#### 1. Device and Flavour Discovery

List supported devices:
```bash
pootstrap device list
pootstrap device show qemu-aarch64
```

List supported UI flavours:
```bash
pootstrap flavour list
pootstrap flavour show phosh
```

#### 2. Building a Root Filesystem

Build an ARM64 root filesystem for QEMU:
```bash
pootstrap rootfs build --device qemu-aarch64 --flavour minimal
```

#### 3. Building a Disk Image

Generate a bootable `.qcow2` image (or `.img` raw disk image):
```bash
# Generate QCOW2 format
pootstrap image build --device qemu-aarch64 --flavour minimal --format qcow2

# Generate Raw image format
pootstrap image build --device pine64-pinephone --flavour phosh --format raw
```

Output images are saved to `~/.local/share/pootstrap/output/`.

#### 4. Running in QEMU

Test boot your image directly:
```bash
pootstrap run --device qemu-aarch64 --flavour minimal --ram 2048 --cpus 2
```

Or pass an image path explicitly:
```bash
pootstrap run --image ~/.local/share/pootstrap/output/parch-qemu-aarch64-minimal.qcow2
```

Inside QEMU:
- Default user login: `alarm` (password: `alarm`)
- Root login: `root` (password: `root`)
- To exit QEMU serial console: press `Ctrl + A` then press `X`

#### 5. Flashing to Physical Devices

Flash an image to a device connected in Fastboot mode:
```bash
pootstrap flash --image output/parch-xiaomi-scorpio-phosh.img --method fastboot
```

Write to an SD card or eMMC exposed through Jumpdrive:
```bash
pootstrap flash --image output/parch-pine64-pinephone-phosh.img --method jumpdrive --target /dev/sdX
```

---

## Configuration

Default settings live in `~/.config/pootstrap/config.toml`. You can inspect and change them via the CLI:

```bash
pootstrap config show
pootstrap config set default_device pine64-pinephone
pootstrap config set default_flavour phosh
```

---

## License

This project is licensed under the **GNU Affero General Public License v3.0 or later** (AGPL-3.0-or-later). See the [LICENSE](LICENSE) file for complete terms and details.
