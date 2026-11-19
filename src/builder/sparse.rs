use crate::error::{PootstrapError, Result};
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

pub const SPARSE_HEADER_MAGIC: u32 = 0xed26ff3a;
pub const MAJOR_VERSION: u16 = 1;
pub const MINOR_VERSION: u16 = 0;
pub const FILE_HEADER_SIZE: u16 = 28;
pub const CHUNK_HEADER_SIZE: u16 = 12;
pub const DEFAULT_BLOCK_SIZE: u32 = 4096;

pub const CHUNK_TYPE_RAW: u16 = 0xCAC1;
#[allow(dead_code)]
pub const CHUNK_TYPE_FILL: u16 = 0xCAC2;
pub const CHUNK_TYPE_DONT_CARE: u16 = 0xCAC3;
#[allow(dead_code)]
pub const CHUNK_TYPE_CRC32: u16 = 0xCAC4;

pub struct SparseImageBuilder {
    raw_path: PathBuf,
    output_path: PathBuf,
    block_size: u32,
    max_chunk_size: Option<u64>,
}

#[allow(dead_code)]
impl SparseImageBuilder {
    pub fn new(raw_path: PathBuf, output_path: PathBuf) -> Self {
        Self {
            raw_path,
            output_path,
            block_size: DEFAULT_BLOCK_SIZE,
            max_chunk_size: None,
        }
    }

    pub fn with_block_size(mut self, block_size: u32) -> Self {
        self.block_size = block_size;
        self
    }

    pub fn with_max_chunk_size(mut self, max_size: Option<u64>) -> Self {
        self.max_chunk_size = max_size;
        self
    }

    pub fn is_sparse(path: &Path) -> bool {
        if let Ok(mut f) = File::open(path) {
            let mut magic = [0u8; 4];
            if f.read_exact(&mut magic).is_ok() {
                return u32::from_le_bytes(magic) == SPARSE_HEADER_MAGIC;
            }
        }
        false
    }

    pub fn build(&self) -> Result<Vec<PathBuf>> {
        if !self.raw_path.exists() {
            return Err(PootstrapError::ImageBuild(format!(
                "Raw image does not exist: {:?}",
                self.raw_path
            )));
        }

        if let Some(parent) = self.output_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        tracing::info!(
            "Converting raw image {:?} to Android sparse format at {:?}",
            self.raw_path,
            self.output_path
        );

        if self.max_chunk_size.is_none() && which::which("img2simg").is_ok() {
            let status = Command::new("img2simg")
                .arg(&self.raw_path)
                .arg(&self.output_path)
                .arg(self.block_size.to_string())
                .status()
                .map_err(PootstrapError::Io)?;

            if status.success() {
                tracing::info!("Sparse image created successfully with host img2simg");
                return Ok(vec![self.output_path.clone()]);
            }
            tracing::warn!("img2simg failed, falling back to native sparse generator");
        }

        self.build_native()
    }

    fn build_native(&self) -> Result<Vec<PathBuf>> {
        let raw_file = File::open(&self.raw_path).map_err(PootstrapError::Io)?;
        let raw_len = raw_file.metadata().map_err(PootstrapError::Io)?.len();
        let blk_sz = self.block_size as usize;

        if raw_len == 0 || raw_len % (blk_sz as u64) != 0 {
            return Err(PootstrapError::ImageBuild(format!(
                "Raw image length ({} bytes) is not a non-zero multiple of block size ({})",
                raw_len, blk_sz
            )));
        }

        let total_blocks = (raw_len / (blk_sz as u64)) as u32;
        let mut reader = BufReader::with_capacity(1024 * 1024, raw_file);

        let max_bytes_per_file = self.max_chunk_size.unwrap_or(u64::MAX);

        let mut output_paths = Vec::new();
        let mut file_idx = 0;
        let mut blk_idx = 0u32;

        let zero_block = vec![0u8; blk_sz];
        let mut cur_block = vec![0u8; blk_sz];

        while blk_idx < total_blocks {
            let target_path = if self.max_chunk_size.is_some() {
                let stem = self
                    .output_path
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy();
                self.output_path
                    .with_file_name(format!("{}.simg.{:03}", stem, file_idx))
            } else {
                self.output_path.clone()
            };

            let out_file = File::create(&target_path).map_err(PootstrapError::Io)?;
            let mut writer = BufWriter::with_capacity(1024 * 1024, out_file);

            // Reserve space for file header
            let header_pos = writer.stream_position().map_err(PootstrapError::Io)?;
            writer
                .write_all(&[0u8; FILE_HEADER_SIZE as usize])
                .map_err(PootstrapError::Io)?;

            let mut chunk_count = 0u32;
            let mut file_blocks = 0u32;
            let mut bytes_written = FILE_HEADER_SIZE as u64;

            while blk_idx < total_blocks {
                reader
                    .read_exact(&mut cur_block)
                    .map_err(PootstrapError::Io)?;

                let is_zero = cur_block == zero_block;
                let chunk_type = if is_zero {
                    CHUNK_TYPE_DONT_CARE
                } else {
                    CHUNK_TYPE_RAW
                };

                let mut run_len = 1u32;
                let mut raw_bytes_accum = if is_zero {
                    Vec::new()
                } else {
                    cur_block.clone()
                };

                while blk_idx + run_len < total_blocks {
                    if let Some(limit) = self.max_chunk_size {
                        let potential_bytes = if is_zero {
                            bytes_written + (CHUNK_HEADER_SIZE as u64)
                        } else {
                            bytes_written
                                + (CHUNK_HEADER_SIZE as u64)
                                + ((run_len + 1) as u64) * (blk_sz as u64)
                        };
                        if potential_bytes > limit {
                            break;
                        }
                    }

                    let mut next_block = vec![0u8; blk_sz];
                    reader
                        .read_exact(&mut next_block)
                        .map_err(PootstrapError::Io)?;

                    let next_is_zero = next_block == zero_block;
                    if next_is_zero == is_zero {
                        run_len += 1;
                        if !is_zero {
                            raw_bytes_accum.extend_from_slice(&next_block);
                        }
                    } else {
                        // Rewind one block
                        reader
                            .seek(SeekFrom::Current(-(blk_sz as i64)))
                            .map_err(PootstrapError::Io)?;
                        break;
                    }
                }

                let chunk_data_bytes = if is_zero { 0 } else { raw_bytes_accum.len() };
                let total_sz = (CHUNK_HEADER_SIZE as u32) + (chunk_data_bytes as u32);

                let mut chunk_hdr = [0u8; 12];
                chunk_hdr[0..2].copy_from_slice(&chunk_type.to_le_bytes());
                chunk_hdr[2..4].copy_from_slice(&0u16.to_le_bytes());
                chunk_hdr[4..8].copy_from_slice(&run_len.to_le_bytes());
                chunk_hdr[8..12].copy_from_slice(&total_sz.to_le_bytes());

                writer.write_all(&chunk_hdr).map_err(PootstrapError::Io)?;
                if !is_zero {
                    writer
                        .write_all(&raw_bytes_accum)
                        .map_err(PootstrapError::Io)?;
                }

                chunk_count += 1;
                file_blocks += run_len;
                blk_idx += run_len;
                bytes_written += total_sz as u64;

                if bytes_written >= max_bytes_per_file && blk_idx < total_blocks {
                    break;
                }
            }

            // Write sparse file header
            let mut file_hdr = [0u8; 28];
            file_hdr[0..4].copy_from_slice(&SPARSE_HEADER_MAGIC.to_le_bytes());
            file_hdr[4..6].copy_from_slice(&MAJOR_VERSION.to_le_bytes());
            file_hdr[6..8].copy_from_slice(&MINOR_VERSION.to_le_bytes());
            file_hdr[8..10].copy_from_slice(&FILE_HEADER_SIZE.to_le_bytes());
            file_hdr[10..12].copy_from_slice(&CHUNK_HEADER_SIZE.to_le_bytes());
            file_hdr[12..16].copy_from_slice(&self.block_size.to_le_bytes());
            file_hdr[16..20].copy_from_slice(&file_blocks.to_le_bytes());
            file_hdr[20..24].copy_from_slice(&chunk_count.to_le_bytes());
            file_hdr[24..28].copy_from_slice(&0u32.to_le_bytes());

            writer
                .seek(SeekFrom::Start(header_pos))
                .map_err(PootstrapError::Io)?;
            writer.write_all(&file_hdr).map_err(PootstrapError::Io)?;
            writer.flush().map_err(PootstrapError::Io)?;

            output_paths.push(target_path);
            file_idx += 1;
        }

        Ok(output_paths)
    }
}
