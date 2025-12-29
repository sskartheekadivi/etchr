use crate::device::Device;
use anyhow::{anyhow, Result};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use sysinfo;

/// Helper to read a specific file from the /sys/block filesystem.
fn read_sys_file(device_name: &str, file: &str) -> io::Result<String> {
    let path = PathBuf::from("/sys/block").join(device_name).join(file);
    fs::read_to_string(path).map(|s| s.trim().to_string())
}

/// Helper to find the parent device of a partition (e.g., /dev/sda1 -> /dev/sda).
/// This is used to find the system drive's parent for exclusion.
fn get_parent_device_path(path: &Path) -> PathBuf {
    let path_str = path.to_string_lossy();

    if path_str.starts_with("/dev/sd") {
        if let Some(index) = path_str.rfind(|c: char| c.is_alphabetic()) {
            return PathBuf::from(&path_str[..=index]);
        }
    } else if path_str.starts_with("/dev/mmcblk") || path_str.starts_with("/dev/nvme") {
        if let Some(index) = path_str.find('p') {
            return PathBuf::from(&path_str[..index]);
        }
    }

    path.to_path_buf()
}

/// Scans for all removable block devices on a Linux system.
///
/// This function discovers devices by iterating through the `/sys/block` directory.
/// It applies several filters to ensure that only suitable, removable devices are
/// returned, excluding the main system drive for safety.
///
/// The filtering logic is as follows:
/// 1.  Find the main system drive (e.g., `/dev/nvme0n1`) and exclude it.
/// 2.  Skip any loop devices (e.g., `loop0`).
/// 3.  Check the `/sys/block/<device>/removable` flag, which is the most reliable
///     indicator of a removable device like a USB drive or SD card.
/// 4.  Check the `/sys/block/<device>/size` to filter out devices that report a size
///     of zero, which often corresponds to empty card readers.
///
/// # Returns
///
/// A `Result<Vec<Device>>` which is a list of discovered [`Device`]s on success,
/// or an error if the system drive cannot be determined or `/sys/block` cannot be read.
pub fn get_removable_devices() -> Result<Vec<Device>> {
    let disks = sysinfo::Disks::new_with_refreshed_list();
    let mut system_disk_parent = None;
    for disk in disks.iter() {
        if disk.mount_point() == Path::new("/") {
            let path = PathBuf::from("/dev/").join(disk.name());
            system_disk_parent = Some(get_parent_device_path(&path));
            break;
        }
    }
    let system_disk_parent =
        system_disk_parent.ok_or_else(|| anyhow!("Could not determine system drive."))?;

    let mut devices = Vec::new();
    let block_dir = fs::read_dir("/sys/block")?;

    for entry in block_dir.filter_map(Result::ok) {
        let device_name = entry.file_name().to_string_lossy().to_string();
        let device_path = PathBuf::from("/dev/").join(&device_name);

        if device_name.starts_with("loop") || device_path == system_disk_parent {
            continue;
        }

        let is_removable = read_sys_file(&device_name, "removable")
            .map(|s| s == "1")
            .unwrap_or(false);

        if !is_removable {
            continue;
        }

        let size_sectors = read_sys_file(&device_name, "size")
            .and_then(|s| {
                s.parse::<u64>()
                    .map_err(|_| io::Error::from(io::ErrorKind::InvalidData))
            })
            .unwrap_or(0);

        if size_sectors == 0 {
            continue;
        }

        let size_gb = (size_sectors * 512) as f64 / (1024.0 * 1024.0 * 1024.0);

        // Try to find a mount point by checking the `sysinfo` list.
        let mut mount_point = "".to_string();
        for disk in disks.iter() {
            if disk.name().to_string_lossy().starts_with(&device_name) {
                let mp = disk.mount_point().to_string_lossy().to_string();
                if !mp.is_empty() {
                    mount_point = mp;
                    break;
                }
            }
        }

        devices.push(Device {
            path: device_path,
            name: device_name,
            size_gb,
            mount_point,
        });
    }

    Ok(devices)
}
