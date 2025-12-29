# etchr

<h3 align="center">A fast, safe, and modular disk imaging utility.</h3>

<p align="center">
  Tired of cryptic `dd` commands? Worried you'll accidentally wipe your system drive?
  <br />
  <code>etchr</code> is a modern, reliable tool that makes flashing SD cards and USB drives simple and safe. It is built as a modular workspace containing a core library and a command-line front-end.
</p>

---

This repository is a Cargo workspace containing the following crates:

* [`etchr`](./etchr/README.md): A fast, safe, and interactive CLI for flashing disk images. This is the crate most users will interact with.
* [`etchr-core`](./etchr-core/README.md): The core, UI-agnostic library for `etchr`. It provides the business logic for device discovery, reading, and writing, and can be used by any front-end.

## 🚀 Installation & Usage

For most users, you will want to install and use the command-line application, `etchr`.

Please see the [**`etchr` README**](./etchr/README.md) for detailed installation and usage instructions.

A quick-start for installation is:

```bash
cargo install etchr
```

## ✨ Project Goals

* **Modularity:** The core logic is completely decoupled from the UI, allowing for different front-ends (CLI, GUI) to be built on the same foundation.
* **Safety:** The primary goal is to prevent users from accidentally wiping the wrong disk. The interactive-only device selection is a key part of this.
* **Performance:** Use unbuffered, direct I/O where possible to achieve the best possible speeds.
* **Cross-Platform:** The architecture is designed to support multiple operating systems (Linux, Windows, macOS) by abstracting platform-specific code into a dedicated layer.

## 🗺️ Roadmap

* [x] Refactor into a `core` library and a `cli` application.
* [ ] Add full implementation for Windows device discovery.
* [ ] Add implementation for macOS device discovery.
* [ ] Add a `etchr-gui` crate using a framework like [Tauri](https://tauri.app/) or [Iced](https://github.com/iced-rs/iced).
* [ ] Smarter reading (e.g., only reading partitions, not the whole empty disk).
* [ ] Multi-write: Flashing one image to multiple devices at once.

## Contributing

Contributions are welcome! This project is structured as a workspace. Please make sure your changes are made in the appropriate crate.

* For changes to the core logic (reading, writing, verification, platform support), please contribute to `etchr-core`.
* For changes to the command-line interface (parsing, prompts, output), please contribute to `etchr`.

Feel free to open an issue or start a discussion.

## License

This project is licensed under the MIT License.
