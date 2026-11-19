use crate::error::{PootstrapError, Result};
use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::Command;

pub struct BootImgBuilder {
    kernel_path: PathBuf,
    ramdisk_path: PathBuf,
    dtb_path: Option<PathBuf>,
    cmdline: String,
    base_address: u32,
    page_size: u32,
    output_path: PathBuf,
}

impl BootImgBuilder {
    pub const DEFAULT_BASE: u32 = 0x80000000;
    pub const DEFAULT_PAGE_SIZE: u32 = 4096;

    pub fn new(
        kernel_path: PathBuf,
        ramdisk_path: PathBuf,
        output_path: PathBuf,
    ) -> Self {
        Self {
            kernel_path,
            ramdisk_path,
            dtb_path: None,
            cmdline: String::new(),
            base_address: Self::DEFAULT_BASE,
            page_size: Self::DEFAULT_PAGE_SIZE,
            output_path,
        }
    }

    pub fn with_dtb(mut self, dtb: Option<PathBuf>) -> Self {
        self.dtb_path = dtb;
        self
    }

    pub fn with_cmdline(mut self, cmdline: String) -> Self {
        self.cmdline = cmdline;
        self
    }

    pub fn with_base(mut self, base: u32) -> Self {
        self.base_address = base;
        self
    }

    pub fn with_page_size(mut self, page_size: u32) -> Self {
        self.page_size = page_size;
        self
    }

    pub fn build(&self) -> Result<PathBuf> {
        if !self.kernel_path.exists() {
            return Err(PootstrapError::ImageBuild(format!(
                "Kernel image not found: {:?}",
                self.kernel_path
            )));
        }

        if !self.ramdisk_path.exists() {
            return Err(PootstrapError::ImageBuild(format!(
                "Ramdisk image not found: {:?}",
                self.ramdisk_path
            )));
        }

        tracing::info!(
            "Building Android boot.img at {:?} (base: 0x{:08x}, page_size: {})",
            self.output_path,
            self.base_address,
            self.page_size
        );

        if which::which("mkbootimg").is_ok() {
            self.build_with_mkbootimg()
        } else {
            self.build_native()
        }
    }

    fn build_with_mkbootimg(&self) -> Result<PathBuf> {
        tracing::info!("Packing Android boot.img using host mkbootimg...");
        let mut cmd = Command::new("mkbootimg");
        cmd.arg("--kernel")
            .arg(&self.kernel_path)
            .arg("--ramdisk")
            .arg(&self.ramdisk_path)
            .arg("--base")
            .arg(format!("0x{:08x}", self.base_address))
            .arg("--pagesize")
            .arg(self.page_size.to_string())
            .arg("--cmdline")
            .arg(&self.cmdline)
            .arg("-o")
            .arg(&self.output_path);

        if let Some(dtb) = self.dtb_path.as_ref().filter(|d| d.exists()) {
            cmd.arg("--dtb").arg(dtb);
        }

        let status = cmd.status().map_err(PootstrapError::Io)?;
        if !status.success() {
            return Err(PootstrapError::ImageBuild(
                "mkbootimg command failed to package boot.img".into(),
            ));
        }

        Ok(self.output_path.clone())
    }

    fn build_native(&self) -> Result<PathBuf> {
        tracing::info!("Packing Android boot.img using native Rust generator...");
        let mut kernel_bytes = Vec::new();
        File::open(&self.kernel_path)
            .map_err(PootstrapError::Io)?
            .read_to_end(&mut kernel_bytes)
            .map_err(PootstrapError::Io)?;

        let mut ramdisk_bytes = Vec::new();
        File::open(&self.ramdisk_path)
            .map_err(PootstrapError::Io)?
            .read_to_end(&mut ramdisk_bytes)
            .map_err(PootstrapError::Io)?;

        let dtb_bytes = if let Some(dtb) = &self.dtb_path {
            if dtb.exists() {
                let mut b = Vec::new();
                File::open(dtb)
                    .map_err(PootstrapError::Io)?
                    .read_to_end(&mut b)
                    .map_err(PootstrapError::Io)?;
                Some(b)
            } else {
                None
            }
        } else {
            None
        };

        if let Some(parent) = self.output_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        let mut out = File::create(&self.output_path).map_err(PootstrapError::Io)?;

        // Boot image header v0
        let mut header = [0u8; 2048];
        header[0..8].copy_from_slice(b"ANDROID!");

        let kernel_size = kernel_bytes.len() as u32;
        let kernel_addr = self.base_address + 0x00008000;
        let ramdisk_size = ramdisk_bytes.len() as u32;
        let ramdisk_addr = self.base_address + 0x01000000;
        let second_size: u32 = 0;
        let second_addr = self.base_address + 0x00F00000;
        let tags_addr = self.base_address + 0x00000100;
        let page_size = self.page_size;

        header[8..12].copy_from_slice(&kernel_size.to_le_bytes());
        header[12..16].copy_from_slice(&kernel_addr.to_le_bytes());
        header[16..20].copy_from_slice(&ramdisk_size.to_le_bytes());
        header[20..24].copy_from_slice(&ramdisk_addr.to_le_bytes());
        header[24..28].copy_from_slice(&second_size.to_le_bytes());
        header[28..32].copy_from_slice(&second_addr.to_le_bytes());
        header[32..36].copy_from_slice(&tags_addr.to_le_bytes());
        header[36..40].copy_from_slice(&page_size.to_le_bytes());

        let cmdline_bytes = self.cmdline.as_bytes();
        let cmdline_len = cmdline_bytes.len().min(512);
        header[64..64 + cmdline_len].copy_from_slice(&cmdline_bytes[..cmdline_len]);

        out.write_all(&header).map_err(PootstrapError::Io)?;
        Self::pad_to_page(&mut out, header.len() as u64, page_size)?;

        out.write_all(&kernel_bytes).map_err(PootstrapError::Io)?;
        Self::pad_to_page(&mut out, kernel_bytes.len() as u64, page_size)?;

        out.write_all(&ramdisk_bytes).map_err(PootstrapError::Io)?;
        Self::pad_to_page(&mut out, ramdisk_bytes.len() as u64, page_size)?;

        if let Some(dtb) = dtb_bytes {
            out.write_all(&dtb).map_err(PootstrapError::Io)?;
            Self::pad_to_page(&mut out, dtb.len() as u64, page_size)?;
        }

        Ok(self.output_path.clone())
    }

    fn pad_to_page(file: &mut File, size: u64, page_size: u32) -> Result<()> {
        let remainder = size % (page_size as u64);
        if remainder != 0 {
            let padding = (page_size as u64) - remainder;
            let zeros = vec![0u8; padding as usize];
            file.write_all(&zeros).map_err(PootstrapError::Io)?;
        }
        Ok(())
    }
}
