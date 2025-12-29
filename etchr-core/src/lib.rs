//! The core, UI-agnostic library for the `etchr` disk imaging utility.
//!
//! `etchr-core` is designed to be used as a library by any front-end, whether it's
//! a command-line interface (like `etchr`) or a graphical user interface. It
//! handles the complexities of device discovery, image decompression, high-speed
//! I/O, and verification.
//!
//! The library is structured into several key modules:
//! - [`device`]: Contains the cross-platform `Device` struct.
//! - [`platform`]: Provides platform-specific logic, primarily for discovering
//!   removable block devices.
//! - [`mod@read`]: Contains the logic for reading data from a device to an image file.
//! - [`mod@write`]: Contains the logic for writing an image file to a device.
//!
//! The primary entry points for imaging operations are the [`read::run`] and
//! [`write::run`] functions. These functions are designed to be asynchronous in

//! nature and report their progress via callbacks, allowing the calling application
//! to display progress in any way it chooses.
//!
//! ## Example: Writing an Image with Progress Reporting
//!
//! ```rust,no_run
//! use etchr_core::{device::Device, platform, write};
//! use std::path::Path;
//! use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
//! use anyhow::Result;
//!
//! fn main() -> Result<()> {
//!     let image_path = Path::new("path/to/image.img.xz");
//!     let devices = platform::get_removable_devices()?;
//!     let device_to_write = devices.first().expect("No removable devices found.");
//!
//!     // A shared flag to allow for graceful cancellation.
//!     let running = Arc::new(AtomicBool::new(true));
//!     
//!     // A simple closure to handle progress updates. A real app might use this
//!     // to update a progress bar widget.
//!     let on_write_progress = |bytes_written: u64| {
//!         println!("{} bytes written", bytes_written);
//!     };
//!
//!     println!("Starting write...");
//!
//!     write::run(
//!         image_path,
//!         &device_to_write.path,
//!         true, // Enable verification
//!         running.clone(),
//!         || {}, // on_decompress_start
//!         |_| {}, // on_decompress_progress
//!         |_| {}, // on_write_start
//!         on_write_progress,
//!         |_| {}, // on_verify_start
//!         |_| {}, // on_verify_progress
//!     )?;
//!
//!     println!("Write complete!");
//!
//!     Ok(())
//! }
//! ```

pub mod device;
mod os_options;
pub mod platform;
pub mod read;
pub mod write;

