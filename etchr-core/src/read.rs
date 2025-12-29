//! Contains the logic for reading data from a device to an image file.
use crate::os_options::OpenOptionsExt;
use anyhow::{anyhow, Result};
use nix::ioctl_read;
use std::fs::File;
use std::io::{Read, Write};
#[cfg(unix)]
use std::os::unix::io::AsRawFd;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

// Use a 1 MiB buffer for I/O operations.
const BUFFER_SIZE: usize = 1024 * 1024;

ioctl_read!(blkgetsize64, 0x12, 114, u64);

/// Reads the entire contents of a block device to an image file.
///
/// This function performs a raw, block-by-block read from the specified device
/// and writes the data to a new file. It determines the device size using a
/// platform-specific `ioctl` call and reports progress via callbacks.
///
/// # Arguments
///
/// * `device_path` - The path to the block device to read from.
/// * `image_path` - The path where the output image file will be created.
/// * `running` - An `Arc<AtomicBool>` used to gracefully cancel the operation.
///   If the flag is set to `false`, the operation will be aborted.
/// * `on_read_start` - A closure that is called once at the beginning of the
///   operation, providing the total number of bytes that will be read.
/// * `on_progress` - A closure that is called repeatedly as data is read. It
///   receives the total number of bytes read so far.
///
/// # Errors
///
/// This function will return an error if:
/// - The device cannot be opened or its size cannot be determined.
/// - The output file cannot be created.
/// - An I/O error occurs during reading or writing.
/// - The operation is cancelled by the user.
pub fn run<F>(
    device_path: &Path,
    image_path: &Path,
    running: Arc<AtomicBool>,
    on_read_start: impl FnOnce(u64),
    mut on_progress: F,
) -> Result<()>
where
    F: FnMut(u64),
{
    let mut device_file = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECT)
        .open(device_path)?;

    // Get the device size in bytes using a platform-specific ioctl.
    #[cfg(unix)]
    let fd = device_file.as_raw_fd();
    let mut size_bytes: u64 = 0;
    #[cfg(unix)]
    unsafe {
        blkgetsize64(fd, &mut size_bytes)?;
    }

    if size_bytes == 0 {
        return Err(anyhow!("Device size is reported as zero"));
    }

    on_read_start(size_bytes);

    let mut image_file = File::create(image_path)?;

    // O_DIRECT requires buffers to be memory-aligned.
    let block_size = 512;
    let mut buf = vec![0u8; BUFFER_SIZE + block_size];
    let offset = buf.as_ptr().align_offset(block_size);
    let buffer = &mut buf[offset..offset + BUFFER_SIZE];

    let mut read_total: u64 = 0;
    while read_total < size_bytes {
        if !running.load(Ordering::SeqCst) {
            std::fs::remove_file(image_path)?;
            return Err(anyhow!("Operation cancelled by user"));
        }

        let to_read = std::cmp::min(BUFFER_SIZE as u64, size_bytes - read_total) as usize;

        device_file.read_exact(&mut buffer[..to_read])?;
        image_file.write_all(&buffer[..to_read])?;

        read_total += to_read as u64;
        on_progress(read_total);
    }

    image_file.flush()?;
    Ok(())
}