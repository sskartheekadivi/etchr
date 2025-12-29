//! Contains the logic for writing an image file to a device.
//!
//! This module handles the multi-stage process of writing, which includes:
//! 1.  Decompressing the image file on-the-fly if it is compressed (`.gz`, `.xz`, `.zst`).
//! 2.  Writing the (decompressed) image data to the target device.
//! 3.  Optionally verifying the written data against the source image.
use crate::os_options::OpenOptionsExt;
use anyhow::{anyhow, Result};
use flate2::read::GzDecoder;
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tempfile::{NamedTempFile, TempPath};
use xz2::read::XzDecoder;
use zstd::stream::read::Decoder as ZstdDecoder;

const BUFFER_SIZE: usize = 1024 * 1024; // 1 MiB

/// Manages the lifetime of a decompressed image file.
/// If the image was decompressed to a temp file, this struct holds the handle
/// and will delete the file on drop.
struct DecompressedImage {
    path: PathBuf,
    _temp_handle: Option<TempPath>,
}

impl AsRef<Path> for DecompressedImage {
    fn as_ref(&self) -> &Path {
        &self.path
    }
}

/// Decompresses an image to a temporary file if necessary.
fn decompress_image<F>(
    input_path: &Path,
    running: Arc<AtomicBool>,
    mut on_progress: F,
) -> io::Result<DecompressedImage>
where
    F: FnMut(u64),
{
    let ext = input_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    let input_file = File::open(input_path)?;

    // Create a reader based on the file extension.
    let mut reader: Box<dyn Read> = match ext.as_str() {
        "gz" | "gzip" => Box::new(GzDecoder::new(BufReader::new(input_file))),
        "xz" => Box::new(XzDecoder::new(BufReader::new(input_file))),
        "zst" | "zstd" => Box::new(ZstdDecoder::new(BufReader::new(input_file))?),
        // Not a compressed file, return a path to the original.
        _ => {
            return Ok(DecompressedImage {
                path: input_path.to_path_buf(),
                _temp_handle: None,
            });
        }
    };

    let mut temp_file = NamedTempFile::new()?;
    {
        let mut writer = BufWriter::new(&mut temp_file);
        let mut buffer = [0u8; 8192];
        let mut total: u64 = 0;

        loop {
            if !running.load(Ordering::SeqCst) {
                return Err(io::Error::new(
                    io::ErrorKind::Interrupted,
                    "Operation cancelled by user",
                ));
            }

            let n = reader.read(&mut buffer)?;
            if n == 0 {
                break;
            }
            writer.write_all(&buffer[..n])?;
            total += n as u64;
            on_progress(total);
        }
        writer.flush()?;
    }

    // Hand over ownership of the temp file to the DecompressedImage struct.
    let temp_path = temp_file.into_temp_path();
    Ok(DecompressedImage {
        path: temp_path.to_path_buf(),
        _temp_handle: Some(temp_path),
    })
}

/// Writes an image file to a block device, with optional verification.
///
/// This is the main entry point for the writing process. It orchestrates the
/// decompression, writing, and verification stages, reporting progress for each
/// stage via callbacks.
///
/// # Arguments
///
/// * `image_path` - Path to the source image file. Can be compressed.
/// * `device_path` - Path to the target block device.
/// * `verify` - If `true`, a verification pass will be performed after writing.
/// * `running` - An `Arc<AtomicBool>` to allow for graceful cancellation.
/// * `on_decompress_start` - Closure called when decompression begins.
/// * `on_decompress_progress` - Closure called with the number of bytes decompressed.
/// * `on_write_start` - Closure called when writing begins, providing the total image size.
/// * `on_write_progress` - Closure called with the number of bytes written.
/// * `on_verify_start` - Closure called when verification begins, providing the total image size.
/// * `on_verify_progress` - Closure called with the number of bytes verified.
///
/// # Errors
///
/// This function will return an error if:
/// - The image file or device cannot be accessed.
/// - An I/O error occurs during any stage.
/// - The verification hash does not match.
/// - The operation is cancelled.
pub fn run<F1, F2, F3>(
    image_path: &Path,
    device_path: &Path,
    verify: bool,
    running: Arc<AtomicBool>,
    on_decompress_start: impl FnOnce(),
    mut on_decompress_progress: F1,
    on_write_start: impl FnOnce(u64),
    mut on_write_progress: F2,
    on_verify_start: impl FnOnce(u64),
    mut on_verify_progress: F3,
) -> Result<()>
where
    F1: FnMut(u64),
    F2: FnMut(u64),
    F3: FnMut(u64),
{
    on_decompress_start();
    let image = match decompress_image(image_path, running.clone(), &mut on_decompress_progress) {
        Ok(img) => img,
        Err(e) if e.kind() == io::ErrorKind::Interrupted => {
            return Err(anyhow!("Operation cancelled by user"));
        }
        Err(e) => return Err(e.into()),
    };

    let mut image_file = File::open(&image)?;
    let image_len = image_file.metadata()?.len();

    let mut device_file = std::fs::OpenOptions::new()
        .write(true)
        .custom_flags(libc::O_DIRECT) // Use O_DIRECT for unbuffered I/O
        .open(device_path)?;

    on_write_start(image_len);

    // Align buffer to 512 bytes for O_DIRECT compatibility.
    let block_size = 512;
    let mut buf = vec![0u8; BUFFER_SIZE + block_size];
    let offset = buf.as_ptr().align_offset(block_size);
    let buffer = &mut buf[offset..offset + BUFFER_SIZE];

    let mut written: u64 = 0;
    while written < image_len {
        if !running.load(Ordering::SeqCst) {
            return Err(anyhow!("Operation cancelled by user"));
        }

        let to_read = std::cmp::min(BUFFER_SIZE as u64, image_len - written) as usize;
        image_file.read_exact(&mut buffer[..to_read])?;

        // The last chunk of data may not be a multiple of the block size.
        // We need to pad it with zeros to satisfy O_DIRECT requirements.
        let padded_size = if to_read % block_size != 0 {
            let pad = (to_read + block_size - 1) / block_size * block_size;
            buffer[to_read..pad].fill(0);
            pad
        } else {
            to_read
        };

        device_file.write_all(&buffer[..padded_size])?;
        written += to_read as u64;
        on_write_progress(written);
    }

    device_file.flush()?;

    if verify {
        let mut image_file = File::open(&image)?;
        let mut device_file = File::open(device_path)?;

        on_verify_start(image_len);

        let mut image_hasher = Sha256::new();
        let mut device_hasher = Sha256::new();

        let mut image_buf = vec![0u8; BUFFER_SIZE];
        let mut device_buf = vec![0u8; BUFFER_SIZE];

        let mut remaining = image_len;
        while remaining > 0 {
            if !running.load(Ordering::SeqCst) {
                return Err(anyhow!("Operation cancelled by user"));
            }

            let chunk = std::cmp::min(BUFFER_SIZE as u64, remaining) as usize;
            image_file.read_exact(&mut image_buf[..chunk])?;
            device_file.read_exact(&mut device_buf[..chunk])?;

            image_hasher.update(&image_buf[..chunk]);
            device_hasher.update(&device_buf[..chunk]);

            remaining -= chunk as u64;
            on_verify_progress(image_len - remaining);
        }

        let hash1 = image_hasher.finalize();
        let hash2 = device_hasher.finalize();

        if hash1 != hash2 {
            return Err(anyhow!("Verification failed: hash mismatch."));
        }
    }

    Ok(())
}