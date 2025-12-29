use anyhow::{Result, anyhow};
use clap::{Parser, Subcommand};
use console::style;
use dialoguer::{Confirm, Select, theme::ColorfulTheme};
use etchr_core::device::Device;
use indicatif::{ProgressBar, ProgressStyle};
use std::io::{IsTerminal, stdout};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

#[cfg(unix)]
use libc::ECHOCTL;
#[cfg(unix)]
use std::os::unix::io::AsRawFd;
#[cfg(unix)]
use termios::{TCSANOW, Termios, tcsetattr};

#[derive(Parser)]
#[command(name = "etchr")]
#[command(about = "A safe, interactive disk imaging tool", version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Write an image to a device interactively
    Write {
        /// Image file to write
        #[arg(required = true)]
        image: PathBuf,

        /// Skip write verification
        #[arg(short = 'n', long = "no-verify")]
        no_verify: bool,
    },
    /// Read a device to an image file interactively
    Read {
        /// Output image file
        #[arg(required = true)]
        image: PathBuf,
    },
    /// List available removable devices
    List,
}

/// A helper struct that, on Unix, disables `ECHOCTL` for the terminal.
///
/// `ECHOCTL` is the terminal flag that causes Ctrl+C to be printed as `^C`.
/// By disabling it, we can have a cleaner exit when the user cancels the
/// operation, as the `ctrlc` handler will print its own message.
/// The original terminal state is restored when this struct is dropped.
struct TermRestorer {
    #[cfg(unix)]
    original_termios: Option<Termios>,
}

impl TermRestorer {
    fn new() -> Self {
        #[cfg(unix)]
        {
            let fd = stdout().as_raw_fd();
            if !stdout().is_terminal() {
                return Self {
                    original_termios: None,
                };
            }

            if let Ok(original_termios) = Termios::from_fd(fd) {
                let mut new_termios = original_termios;
                // Disable printing of control characters.
                new_termios.c_lflag &= !ECHOCTL;

                if tcsetattr(fd, TCSANOW, &new_termios).is_ok() {
                    Self {
                        original_termios: Some(original_termios),
                    }
                } else {
                    Self {
                        original_termios: None,
                    }
                }
            } else {
                Self {
                    original_termios: None,
                }
            }
        }
        #[cfg(not(unix))]
        {
            // This is a no-op on non-Unix platforms.
            Self {}
        }
    }
}

impl Drop for TermRestorer {
    fn drop(&mut self) {
        #[cfg(unix)]
        if let Some(ref original_termios) = self.original_termios {
            let fd = stdout().as_raw_fd();
            // Restore the original terminal settings.
            tcsetattr(fd, TCSANOW, original_termios).ok();
        }
    }
}

/// Presents an interactive menu for the user to select a device.
fn select_device(devices: &[Device], prompt: &str) -> Result<Device> {
    if devices.is_empty() {
        return Err(anyhow!("No removable devices found."));
    }

    let items: Vec<String> = devices.iter().map(|d| d.to_string()).collect();

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt(prompt)
        .items(&items)
        .default(0)
        .interact()?;

    Ok(devices[selection].clone())
}

/// Presents a final "Yes/No" confirmation to the user.
fn confirm_operation(prompt: &str) -> Result<bool> {
    let confirmation = Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt(prompt)
        .default(false)
        .interact()?;

    Ok(confirmation)
}

fn main() -> Result<()> {
    // This guard will be dropped when main() exits, restoring the terminal.
    let _term_restorer = TermRestorer::new();

    // This flag allows for graceful cancellation of operations.
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    // Set up the Ctrl+C handler to toggle the `running` flag.
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })?;

    let cli = Cli::parse();

    match cli.command {
        Commands::Write { image, no_verify } => {
            let devices = etchr_core::platform::get_removable_devices()?;
            let device = select_device(&devices, "Select the target device to WRITE to")?;

            println!(
                "{} This will erase all data on '{}' ({:.1} GB).",
                style("WARNING:").red().bold(),
                device.name,
                device.size_gb,
            );
            println!("  Device: {}", style(device.path.display()).cyan());
            println!("  Image:  {}", style(image.display()).cyan());
            println!();

            if !confirm_operation("Are you sure you want to proceed?")? {
                println!("Write operation cancelled.");
                return Ok(());
            }

            println!();

            // Set up progress bars for the multi-stage write process.
            // Conditionally create progress bars so they don't flash on screen if not needed.
            let is_compressed = image.extension().and_then(|e| e.to_str()).map_or(false, |e| {
                matches!(e.to_lowercase().as_str(), "gz" | "gzip" | "xz" | "zst" | "zstd")
            });

            let decompress_pb = if is_compressed {
                ProgressBar::new_spinner()
            } else {
                ProgressBar::hidden()
            };

            let write_pb = ProgressBar::new(0);

            let verify_pb = if !no_verify {
                ProgressBar::new(0)
            } else {
                ProgressBar::hidden()
            };


            // These closures connect the core library's progress reporting to our UI.
            let on_decompress_start = || {
                decompress_pb.set_prefix("Decompress");
                decompress_pb.set_style(
                    ProgressStyle::default_spinner()
                        .template("{prefix:12} [{elapsed_precise}] [{spinner}] {bytes} ({bytes_per_sec}) {msg}")
                        .unwrap()
                        .tick_strings(&[
                            &style("■■  ■  ■  ■  ■  ■                       ")
                                .blue()
                                .to_string(),
                            &style("■  ■  ■  ■  ■  ■  ■                     ")
                                .blue()
                                .to_string(),
                            &style(" ■  ■  ■  ■  ■  ■  ■                    ")
                                .blue()
                                .to_string(),
                            &style("  ■  ■  ■  ■  ■  ■  ■                   ")
                                .blue()
                                .to_string(),
                            &style("   ■  ■  ■  ■  ■  ■  ■                  ")
                                .blue()
                                .to_string(),
                            &style("    ■  ■  ■  ■  ■  ■  ■                 ")
                                .blue()
                                .to_string(),
                            &style("     ■  ■  ■  ■  ■  ■  ■                ")
                                .blue()
                                .to_string(),
                            &style("      ■  ■  ■  ■  ■  ■  ■               ")
                                .blue()
                                .to_string(),
                            &style("       ■  ■  ■  ■  ■  ■  ■              ")
                                .blue()
                                .to_string(),
                            &style("        ■  ■  ■  ■  ■  ■  ■             ")
                                .blue()
                                .to_string(),
                            &style("         ■  ■  ■  ■  ■  ■  ■            ")
                                .blue()
                                .to_string(),
                            &style("          ■  ■  ■  ■  ■  ■  ■           ")
                                .blue()
                                .to_string(),
                            &style("           ■  ■  ■  ■  ■  ■  ■          ")
                                .blue()
                                .to_string(),
                            &style("            ■  ■  ■  ■  ■  ■  ■         ")
                                .blue()
                                .to_string(),
                            &style("             ■  ■  ■  ■  ■  ■  ■        ")
                                .blue()
                                .to_string(),
                            &style("              ■  ■  ■  ■  ■  ■  ■       ")
                                .blue()
                                .to_string(),
                            &style("               ■  ■  ■  ■  ■  ■  ■      ")
                                .blue()
                                .to_string(),
                            &style("                ■  ■  ■  ■  ■  ■  ■     ")
                                .blue()
                                .to_string(),
                            &style("                 ■  ■  ■  ■  ■  ■  ■    ")
                                .blue()
                                .to_string(),
                            &style("                  ■  ■  ■  ■  ■  ■  ■   ")
                                .blue()
                                .to_string(),
                            &style("                   ■  ■  ■  ■  ■  ■  ■  ")
                                .blue()
                                .to_string(),
                            &style("                    ■  ■  ■  ■  ■  ■  ■ ")
                                .blue()
                                .to_string(),
                            &style("                     ■  ■  ■  ■  ■  ■  ■")
                                .blue()
                                .to_string(),
                            &style("                       ■  ■  ■  ■  ■  ■■")
                                .blue()
                                .to_string(),
                            &style("                     ■  ■  ■  ■  ■  ■  ■")
                                .blue()
                                .to_string(),
                            &style("                    ■  ■  ■  ■  ■  ■  ■ ")
                                .blue()
                                .to_string(),
                            &style("                   ■  ■  ■  ■  ■  ■  ■  ")
                                .blue()
                                .to_string(),
                            &style("                  ■  ■  ■  ■  ■  ■  ■   ")
                                .blue()
                                .to_string(),
                            &style("                 ■  ■  ■  ■  ■  ■  ■    ")
                                .blue()
                                .to_string(),
                            &style("                ■  ■  ■  ■  ■  ■  ■     ")
                                .blue()
                                .to_string(),
                            &style("               ■  ■  ■  ■  ■  ■  ■      ")
                                .blue()
                                .to_string(),
                            &style("              ■  ■  ■  ■  ■  ■  ■       ")
                                .blue()
                                .to_string(),
                            &style("             ■  ■  ■  ■  ■  ■  ■        ")
                                .blue()
                                .to_string(),
                            &style("            ■  ■  ■  ■  ■  ■  ■         ")
                                .blue()
                                .to_string(),
                            &style("           ■  ■  ■  ■  ■  ■  ■          ")
                                .blue()
                                .to_string(),
                            &style("          ■  ■  ■  ■  ■  ■  ■           ")
                                .blue()
                                .to_string(),
                            &style("         ■  ■  ■  ■  ■  ■  ■            ")
                                .blue()
                                .to_string(),
                            &style("        ■  ■  ■  ■  ■  ■  ■             ")
                                .blue()
                                .to_string(),
                            &style("       ■  ■  ■  ■  ■  ■  ■              ")
                                .blue()
                                .to_string(),
                            &style("      ■  ■  ■  ■  ■  ■  ■               ")
                                .blue()
                                .to_string(),
                            &style("     ■  ■  ■  ■  ■  ■  ■                ")
                                .blue()
                                .to_string(),
                            &style("    ■  ■  ■  ■  ■  ■  ■                 ")
                                .blue()
                                .to_string(),
                            &style("   ■  ■  ■  ■  ■  ■  ■                  ")
                                .blue()
                                .to_string(),
                            &style("  ■  ■  ■  ■  ■  ■  ■                   ")
                                .blue()
                                .to_string(),
                            &style(" ■  ■  ■  ■  ■  ■  ■                    ")
                                .blue()
                                .to_string(),
                            &style("■  ■  ■  ■  ■  ■  ■                     ")
                                .blue()
                                .to_string(),
                        ]),
                );
                decompress_pb.enable_steady_tick(Duration::from_millis(100));
            };
            let on_decompress_progress = |bytes| decompress_pb.set_position(bytes);

            let on_write_start = |len| {
                if is_compressed {
                    decompress_pb.finish_with_message("Decompression complete.");
                }
                write_pb.set_length(len);
                write_pb.set_prefix("Writing");
                write_pb.set_style(
                    ProgressStyle::default_bar()
                        .template(
                            "{prefix:12} [{elapsed_precise}] [{bar:40.green/black}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})",
                        )
                        .unwrap()
                        .progress_chars("■ "),
                );
            };
            let on_write_progress = |bytes| write_pb.set_position(bytes);

            let on_verify_start = |len| {
                write_pb.finish_with_message("Write complete.");
                verify_pb.set_length(len);
                verify_pb.set_prefix("Verifying");
                verify_pb.set_style(
                    ProgressStyle::default_bar()
                        .template(
                            "{prefix:12} [{elapsed_precise}] [{bar:40.magenta/black}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})",
                        )
                        .unwrap()
                        .progress_chars("■ "),
                );
            };
            let on_verify_progress = |bytes| verify_pb.set_position(bytes);

            // Execute the write operation.
            let result = etchr_core::write::run(
                &image,
                &device.path,
                !no_verify,
                running,
                on_decompress_start,
                on_decompress_progress,
                on_write_start,
                on_write_progress,
                on_verify_start,
                on_verify_progress,
            );

            // Cleanly finish progress bars based on the result.
            match result {
                Ok(_) => {
                    if !no_verify {
                        verify_pb.finish_with_message("Verification successful.");
                    } else {
                        // The write bar is already finished, but this sets a final message.
                        write_pb.finish_with_message("Write complete (verification skipped).");
                    }
                    println!(
                        "\n✨ Successfully flashed {} with {}.",
                        style(device.path.display()).cyan(),
                        style(image.display()).cyan()
                    );
                }
                Err(e) => {
                    // On error, finish all bars with a failure message to unblock the terminal.
                    if is_compressed {
                        decompress_pb.finish_with_message("❌ Operation failed.");
                    }
                    write_pb.finish_and_clear();
                    if !no_verify {
                        verify_pb.finish_and_clear();
                    }
                    return Err(e);
                }
            }
        }
        Commands::Read { image } => {
            let devices = etchr_core::platform::get_removable_devices()?;
            let device = select_device(&devices, "Select the source device to READ from")?;

            println!(
                "This will read {:.1} GB from '{}'.",
                device.size_gb, device.name
            );
            println!("  Device: {}", style(device.path.display()).cyan());
            println!("  Output: {}", style(image.display()).cyan());
            println!();

            if !confirm_operation("Are you sure you want to proceed?")? {
                println!("Read operation cancelled.");
                return Ok(());
            }

            println!();

            let read_pb = ProgressBar::new(0);

            let on_read_start = |len| {
                read_pb.set_length(len);
                read_pb.set_prefix("Reading");
                read_pb.set_style(
                    ProgressStyle::default_bar()
                        .template(
                            "{prefix:12} [{elapsed_precise}] [{bar:40.green/black}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})",
                        )
                        .unwrap()
                        .progress_chars("■ "),
                );
            };
            let on_progress = |bytes| read_pb.set_position(bytes);

            let result =
                etchr_core::read::run(&device.path, &image, running, on_read_start, on_progress);

            match result {
                Ok(_) => {
                    read_pb.finish_with_message("Read complete.");
                    println!(
                        "\n✨ Successfully read {} to {}.",
                        style(device.path.display()).cyan(),
                        style(image.display()).cyan()
                    );
                }
                Err(e) => {
                    read_pb.finish_with_message("❌ Operation failed.");
                    return Err(e);
                }
            }
        }
        Commands::List => {
            let devices = etchr_core::platform::get_removable_devices()?;
            if devices.is_empty() {
                println!("No removable devices found.");
                return Ok(());
            }

            println!("Found {} removable devices:", devices.len());
            println!(
                "\n  {:<12} {:<25} {:<10} {}",
                "DEVICE", "NAME", "SIZE", "LOCATION"
            );
            println!("  {:-<12} {:-<25} {:-<10} {:-<20}", "", "", "", "");
            for device in devices {
                let location = if device.mount_point.is_empty() {
                    "(Not mounted)".to_string()
                } else {
                    device.mount_point
                };
                println!(
                    "  {:<12} {:<25} {:>8.1} GB  {}",
                    device.path.display(),
                    device.name,
                    device.size_gb,
                    location
                );
            }
        }
    }

    Ok(())
}
