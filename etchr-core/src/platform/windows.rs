use crate::device::Device;
use anyhow::Result;

/// Scans for all removable block devices on a Windows system.
///
/// # Returns
///
/// A `Result<Vec<Device>>`.
///
/// # Panics
///
/// This function currently panics because Windows support is not yet implemented.
pub fn get_removable_devices() -> Result<Vec<Device>> {
    // TODO: Implement device discovery for Windows using the Win32 API.
    // This will likely involve using functions like `SetupDiGetClassDevsW`,
    // `SetupDiEnumDeviceInfo`, and `DeviceIoControl` to query for disk devices
    // and their properties (e.g., removable, size).
    unimplemented!("Windows support is not yet implemented.");
}
