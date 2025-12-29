use std::fmt;
use std::path::PathBuf;

/// Represents a block device discovered on the system.
///
/// This struct holds cross-platform information about a device, such as its
/// system path, size, and mount point. It is populated by the platform-specific
/// discovery functions in the [`crate::platform`] module.
#[derive(Clone, Debug)]
pub struct Device {
    /// The system path to the device (e.g., `/dev/sda` or `\\.\PhysicalDrive0`).
    pub path: PathBuf,
    /// The kernel-provided name of the device (e.g., "sda").
    pub name: String,
    /// The total size of the device in gigabytes (GB).
    pub size_gb: f64,
    /// The primary mount point of the device, if any.
    pub mount_point: String,
}

impl fmt::Display for Device {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mount_info = if !self.mount_point.is_empty() {
            format!("[Mounted at {}]", self.mount_point)
        } else {
            "[Not mounted]".to_string()
        };

        write!(
            f,
            "{:<15} {:.1} GB {}",
            self.path.display(),
            self.size_gb,
            mount_info
        )
    }
}