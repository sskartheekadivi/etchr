# etchr-core

This crate provides the core, UI-agnostic logic for the `etchr` disk imaging utility.

`etchr-core` is designed to be used as a library by any front-end, whether it's a command-line interface (like `etchr`) or a graphical user interface. It handles the complexities of device discovery, image decompression, high-speed I/O, and verification, all while being completely decoupled from how progress or prompts are displayed to the user.

## ✨ Features

* **Platform-Agnostic API:** Provides a consistent interface for discovering block devices across different operating systems (currently Linux, with planned support for Windows and macOS).
* **Decompression On-the-Fly:** Automatically decompresses `.gz`, `.xz`, and `.zst` images.
* **Safe I/O:** Uses unbuffered I/O and platform-specific flags (`O_DIRECT` on Linux) for high-speed operations.
* **Progress Reporting via Callbacks:** The `read::run` and `write::run` functions are asynchronous in nature and report their progress (e.g., bytes written, total bytes) via closures provided by the caller. This allows any UI to hook into the process and display progress in its own way.
* **Verification:** Includes a SHA256 verification mechanism to ensure data integrity after a write.

## Usage

Add `etchr-core` as a dependency in your `Cargo.toml`. Then, you can use its functions to perform imaging operations.

### Example: Writing an Image with Progress Reporting

```rust,no_run
use etchr_core::{platform, write};
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

fn main() -> anyhow::Result<()> {
    let image_path = Path::new("path/to/image.img.xz");
    let devices = platform::get_removable_devices()?;
    let device_to_write = devices.first().ok_or_else(|| anyhow::anyhow!("No devices found"))?;

    let running = Arc::new(AtomicBool::new(true));
    // A simple progress handler that prints to the console.
    // A real GUI would use this to update a progress bar widget.
    let mut last_progress = 0;
    let progress_handler = |bytes_done: u64| {
        // Update UI here
        println!("Progress: {} bytes written", bytes_done);
        last_progress = bytes_done;
    };
    
    let on_start = |total_bytes: u64| {
        println!("Starting write of {} bytes", total_bytes);
    };


    write::run(
        image_path,
        &device_to_write.path,
        true, // Enable verification
        running,
        || {}, // on_decompress_start
        |_| {}, // on_decompress_progress
        on_start,
        progress_handler,
        |_| {}, // on_verify_start
        |_| {}, // on_verify_progress
    )?;

    println!("Write complete!");

    Ok(())
}
```

This crate is the foundation of the `etchr` tool and is designed to be robust, flexible, and easy to integrate into other projects that require disk imaging capabilities.
