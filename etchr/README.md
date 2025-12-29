# etchr

<h3 align="center">A fast, safe, and interactive CLI for flashing disk images.</h3>

<p align="center">
  Tired of cryptic `dd` commands? Worried you'll accidentally wipe your system drive?
  <br />
  <code>etchr</code> is a modern, reliable tool that makes flashing SD cards and USB drives simple and safe, right from your terminal. It is the command-line front-end for the `etchr` disk imaging utility.
</p>

---

## ✨ Features

* **🛡️ Interactive Safety First**
    `etchr` doesn't let you pass a device path. Instead, it shows an interactive menu of **only removable devices**, making it nearly impossible to flash your system drive by mistake.

* **🚀 Decompression On-the-Fly**
    Automatically decompresses `.gz`, `.xz`, and `.zst` images while writing. No need to extract them first.

* **⚡ Blazingly Fast**
    Built on the `etchr-core` library, which is optimized for high-speed, unbuffered I/O to flash images as fast as your hardware allows.

* **✅ Guaranteed Verification**
    Automatically verifies the disk with a SHA256 hash after writing to ensure the data is perfect, bit-for-bit. (You can skip this with `--no-verify`).

* **📊 Detailed Progress**
    A beautiful progress bar shows your speed, data transferred, and ETA, so you're never left guessing.

* **🛑 Graceful Cancel**
    Press `Ctrl+C` at any time to safely cancel the operation. `etchr` cleans up after itself, leaving no temporary files or half-written states.

## 🚀 Installation

### With `cargo` (Recommended)

This is the easiest way to get the latest version if you have the Rust toolchain.

```bash
cargo install etchr
```

### From GitHub Releases

Download the pre-compiled binary for your platform from the [Releases page](https://github.com/sskartheekadivi/etchr/releases).

### From Source

```bash
git clone https://github.com/sskartheekadivi/etchr.git
cd etchr
cargo build -p etchr --release
sudo cp ./target/release/etchr /usr/local/bin/etchr
```

## 💡 Usage

`etchr` is designed to be simple. The commands guide you.

### `etchr list`

List all detected removable devices and their mount points.

```
$ etchr list
Found 1 removable devices:

  DEVICE       NAME                 SIZE LOCATION
  ----------   -----------------   ----- ----------
  /dev/sdd     Cruzer Blade       29.5 GB /media/user/USB_DISK
```

### `etchr write`

Write an image to a device. You will be prompted to select a target from a safe, interactive list.

```bash
# You can use compressed or uncompressed images
etchr write ~/Downloads/raspberry-pi-os.img.xz
```

This will start the interactive prompt:

```
✔ Select the target device to WRITE to · /dev/sdd     29.5 GB [Mounted at /media/user/USB_DISK]
WARNING: This will erase all data on 'sdd' (29.5 GB).
  Device: /dev/sdd
  Image:  /home/user/Downloads/raspberry-pi-os.img.xz

✔ Are you sure you want to proceed? · yes

Decompress [00:00:10] [■■■■■■■■■■■■■■■■] 1.53 GiB (150.37 MiB/s) Decompression complete.
Writing    [00:01:28] [■■■■■■■■■■■■■■■■] 8.00 GiB (90.12 MiB/s) Write complete.
Verifying  [00:01:00] [■■■■■■■■■■■■■■■■] 8.00 GiB (133.33 MiB/s) Verification complete.

✨ Successfully flashed /dev/sdd with raspberry-pi-os.img.xz.
```

**Options:**

* `--no-verify`: Skips the verification step after writing.

### `etchr read`

Create an image file by reading an entire device. You will be prompted to select a source.

```bash
etchr read ~/Backups/my-sd-card-backup.img
```

This will start the interactive prompt:

```
✔ Select the source device to READ from · /dev/sdd     29.5 GB [Mounted at /media/user/USB_DISK]
This will read 29.5 GB from 'sdd'.
  Device: /dev/sdd
  Output: /home/user/Backups/my-sd-card-backup.img

✔ Are you sure you want to proceed? · yes

Reading    [00:05:00] [■■■■■■■■■■■■■■■■] 29.5 GiB (100.0 MiB/s) Read complete.

✨ Successfully read /dev/sdd to my-sd-card-backup.img.
```
