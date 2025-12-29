//! Provides platform-specific functionality.
//!
//! This module contains the logic for interacting with the operating system to
//! perform tasks that are not cross-platform, such as discovering removable
//! block devices.
//!
//! It uses conditional compilation (`#[cfg]`) to expose the correct implementation
//! for the target OS (e.g., Linux, Windows). The goal is for each submodule
//! to expose the same public API, so that the rest of the library can use it
//! without worrying about the underlying platform.

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub use self::linux::*;

#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
pub use self::windows::*;
