//! OpenFlash CLI - Command-line interface for flash programming
//!
//! # Usage
//! ```bash
//! openflash scan                    # Scan for devices
//! openflash detect                  # Detect connected chip
//! openflash read -o dump.bin        # Read full chip
//! openflash write -i firmware.bin   # Write firmware
//! openflash analyze dump.bin        # AI analysis
//! openflash batch jobs.toml         # Run batch jobs
//! openflash server start            # Start server mode (v2.0)
//! openflash job submit read         # Submit job to server (v2.0)
//! ```

use clap::{Parser, Subcommand};
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::PathBuf;

mod commands;
mod connection;
mod device_commands;

/// OpenFlash - Open-source NAND/eMMC/NOR flash programmer
#[derive(Parser)]
#[command(name = "openflash")]
#[command(version)]
#[command(about = "Command-line interface for flash programming and analysis")]
#[command(long_about = None)]
pub struct Cli {
    /// Output format (text, json)
    #[arg(short = 'f', long, default_value = "text", global = true)]
    pub format: String,

    /// Verbose output
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Quiet mode: only errors, no banner, no progress bars
    #[arg(short, long, global = true)]
    pub quiet: bool,

    /// USB device to use, by serial number or bus-address.
    ///
    /// Omit it when exactly one device is attached. Run `openflash scan` to see
    /// what is connected.
    #[arg(short = 'd', long, global = true)]
    pub device: Option<String>,

    /// Reach an SBC agent over TCP, as host:port
    #[arg(long, global = true, value_name = "HOST:PORT")]
    pub tcp: Option<String>,

    /// Reach an SBC agent over a Unix socket
    #[arg(long, global = true, value_name = "PATH")]
    pub unix: Option<String>,

    /// Run against the in-process emulator with a chip of this many bytes.
    ///
    /// No hardware is touched and nothing real is read or written. Intended for
    /// trying out commands and for the test suite; every emulated run is
    /// labelled as such on stderr. The size must be a power of two of at least
    /// 4096 bytes.
    #[arg(long, global = true, value_name = "BYTES")]
    pub emulate: Option<u64>,

    /// Back the emulated chip with a file so its contents survive between runs.
    ///
    /// Created blank at --emulate bytes (2 MiB by default) if it does not exist.
    /// Without this, each command gets a freshly erased emulated chip, so a
    /// write in one invocation is invisible to a read in the next.
    #[arg(long, global = true, value_name = "PATH", requires = "emulate")]
    pub emulate_image: Option<PathBuf>,

    /// Per-exchange timeout in milliseconds
    #[arg(long, global = true, default_value = "5000")]
    pub timeout_ms: u64,

    /// Do not ask before an operation that modifies the chip
    #[arg(long, global = true)]
    pub yes: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Scan for connected OpenFlash devices
    Scan,

    /// Detect and identify connected flash chip
    Detect,

    /// Read/dump flash chip contents
    Read {
        /// Output file path
        #[arg(short, long)]
        output: PathBuf,

        /// Start address (hex or decimal)
        #[arg(short, long, default_value = "0")]
        start: String,

        /// Length to read (default: full chip)
        #[arg(short, long)]
        length: Option<String>,

        /// Include OOB/spare area data (NAND interfaces only)
        #[arg(long)]
        oob: bool,
    },

    /// Write/program flash chip
    Write {
        /// Input file path
        #[arg(short, long)]
        input: PathBuf,

        /// Start address (hex or decimal)
        #[arg(short, long, default_value = "0")]
        start: String,

        /// Read the region back and compare it after writing
        #[arg(long, default_value = "true", action = clap::ArgAction::Set)]
        verify: bool,

        /// Erase the affected sectors before writing.
        ///
        /// Programming can only clear bits, so writing over data that was not
        /// erased produces the bitwise AND of old and new. Turn this off only
        /// when the target range is known to be blank.
        #[arg(long, default_value = "true", action = clap::ArgAction::Set)]
        erase: bool,
    },

    /// Erase flash chip (full or partial)
    ///
    /// The range must be sector-aligned: erase granularity is a property of the
    /// chip, and erasing beyond what was asked would destroy neighbouring data.
    Erase {
        /// Start address (hex or decimal)
        #[arg(short, long)]
        start: Option<String>,

        /// Length to erase (default: to the end of the chip)
        #[arg(short, long)]
        length: Option<String>,
    },

    /// Verify flash contents against file
    Verify {
        /// File to verify against
        // No short form: -f is the global --format.
        #[arg(long)]
        file: PathBuf,

        /// Start address
        #[arg(short, long, default_value = "0")]
        start: String,
    },

    /// AI-powered dump analysis
    Analyze {
        /// Input dump file (or use last read)
        input: Option<PathBuf>,

        /// Output report file
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Deep scan mode (slower but more thorough)
        #[arg(long)]
        deep: bool,

        /// Report format (md, html, json)
        #[arg(long, default_value = "md")]
        report_format: String,
    },

    /// Compare two dump files
    Compare {
        /// First dump file
        file1: PathBuf,

        /// Second dump file
        file2: PathBuf,

        /// Output diff report
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Clone chip-to-chip
    Clone {
        /// Clone mode (exact, skip-bad, wear-aware)
        #[arg(short, long, default_value = "skip-bad")]
        mode: String,

        /// Verify after clone
        #[arg(long, default_value = "true")]
        verify: bool,
    },

    /// Run batch processing jobs
    Batch {
        /// Batch job file (TOML format)
        file: PathBuf,

        /// Stop on first error
        #[arg(long)]
        stop_on_error: bool,
    },

    /// Run Python/Lua script
    Script {
        /// Script file path
        file: PathBuf,

        /// Script arguments
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,
    },

    /// List supported flash chips
    Chips {
        /// Filter by interface (nand, spi-nand, spi-nor, emmc, ufs)
        #[arg(short, long)]
        interface: Option<String>,

        /// Filter by manufacturer
        #[arg(short, long)]
        manufacturer: Option<String>,

        /// Search by model name
        #[arg(short, long)]
        search: Option<String>,
    },

    /// Show device information
    Info,

    /// Set flash interface
    Interface {
        /// Interface type (nand, spi-nand, spi-nor, emmc, ufs)
        interface: String,
    },

    /// Configuration management
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },

    /// Unpack firmware (binwalk-like)
    Unpack {
        /// Input dump file
        input: PathBuf,

        /// Output directory
        #[arg(short, long)]
        output: PathBuf,

        /// Maximum extraction depth
        #[arg(long, default_value = "5")]
        depth: u32,

        /// Recursive extraction
        #[arg(long, default_value = "true")]
        recursive: bool,
    },

    /// Extract root filesystem
    Rootfs {
        /// Input dump file
        input: PathBuf,

        /// Output directory
        #[arg(short, long)]
        output: PathBuf,

        /// Extract file contents
        #[arg(long, default_value = "true")]
        contents: bool,
    },

    /// Scan for vulnerabilities
    Vulnscan {
        /// Input dump file
        input: PathBuf,

        /// Output report file
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Check for hardcoded credentials
        #[arg(long, default_value = "true")]
        credentials: bool,

        /// Check for weak crypto
        #[arg(long, default_value = "true")]
        weak_crypto: bool,
    },

    /// ML-based chip identification
    Identify {
        /// Input dump file
        input: PathBuf,

        /// Show top N predictions
        #[arg(long, default_value = "3")]
        top: usize,
    },

    /// Custom signature operations
    Signatures {
        #[command(subcommand)]
        action: SignaturesAction,
    },

    // ========== v2.0 - Multi-device & Enterprise ==========
    /// Server mode operations
    Server {
        #[command(subcommand)]
        action: ServerAction,
    },

    /// Device pool management
    Device {
        #[command(subcommand)]
        action: DeviceAction,
    },

    /// Job queue management
    Job {
        #[command(subcommand)]
        action: JobAction,
    },

    /// Parallel dump across multiple devices
    ParallelDump {
        /// Output directory
        #[arg(short, long)]
        output: PathBuf,

        /// Number of devices to use
        // No short form: -d is the global --device.
        #[arg(long, default_value = "4")]
        devices: usize,

        /// Chunk size per device
        #[arg(long, default_value = "64M")]
        chunk_size: String,

        /// Merge output files
        #[arg(long, default_value = "true")]
        merge: bool,
    },

    /// Production line mode
    Production {
        #[command(subcommand)]
        action: ProductionAction,
    },
}

#[derive(Subcommand)]
enum ConfigAction {
    /// Show current configuration
    Show,
    /// Set configuration value
    Set { key: String, value: String },
    /// Reset to defaults
    Reset,
}

#[derive(Subcommand)]
enum SignaturesAction {
    /// Load custom signatures from file
    Load {
        /// Signature file (YAML format)
        file: PathBuf,
    },
    /// Scan dump with custom signatures
    Scan {
        /// Input dump file
        input: PathBuf,
        /// Signature file (optional, uses loaded)
        #[arg(short, long)]
        signatures: Option<PathBuf>,
    },
    /// Export signature database
    Export {
        /// Output file
        output: PathBuf,
    },
    /// List loaded signatures
    List,
}

// ========== v2.0 - Multi-device & Enterprise Subcommands ==========

#[derive(Subcommand)]
enum ServerAction {
    /// Start OpenFlash server
    Start {
        /// Listen host
        #[arg(long, default_value = "0.0.0.0")]
        host: String,

        /// Listen port
        #[arg(long, default_value = "8080")]
        port: u16,

        /// Configuration file
        #[arg(short, long)]
        config: Option<PathBuf>,
    },
    /// Stop OpenFlash server
    Stop,
    /// Get server status
    Status {
        /// Server URL
        #[arg(long)]
        url: Option<String>,
    },
}

#[derive(Subcommand)]
enum DeviceAction {
    /// List devices in pool
    List {
        /// Server URL
        #[arg(long)]
        url: Option<String>,
    },
    /// Add device to pool
    Add {
        /// Device name
        #[arg(short, long)]
        name: String,

        /// Device URI (serial://, tcp://, ws://)
        #[arg(short, long)]
        uri: String,

        /// Device platform (RP2040, STM32F4, ESP32)
        #[arg(long, default_value = "RP2040")]
        platform: String,

        /// Device tags
        #[arg(long)]
        tags: Vec<String>,
    },
    /// Remove device from pool
    Remove {
        /// Device ID
        device_id: String,
    },
}

#[derive(Subcommand)]
enum JobAction {
    /// Submit a job to the queue
    Submit {
        /// Job type (read, write, erase, analyze)
        job_type: String,

        /// Job parameters
        #[arg(trailing_var_arg = true)]
        params: Vec<String>,

        /// Specific device ID
        // No short form: -d is the global --device.
        #[arg(long)]
        device: Option<String>,

        /// Job priority (low, normal, high, critical)
        #[arg(long)]
        priority: Option<String>,
    },
    /// Get job status
    Status {
        /// Job ID
        job_id: u64,
    },
    /// Cancel a job
    Cancel {
        /// Job ID
        job_id: u64,
    },
    /// List jobs
    List {
        /// Filter by status
        #[arg(long)]
        status: Option<String>,

        /// Maximum number of jobs to show
        #[arg(long, default_value = "20")]
        limit: usize,
    },
}

#[derive(Subcommand)]
enum ProductionAction {
    /// Start production mode
    Start {
        /// Configuration file
        #[arg(short, long)]
        config: PathBuf,

        /// Production line ID
        #[arg(long)]
        line: Option<String>,
    },
    /// Get production status
    Status {
        /// Production line ID
        #[arg(long)]
        line: Option<String>,
    },
}

fn main() {
    let cli = Cli::parse();

    // The banner goes to stderr and only in text mode: printed on stdout it
    // would sit in front of `--format json` output and make it unparseable.
    if !cli.quiet && cli.format == "text" {
        print_banner();
    }

    let result = match &cli.command {
        // Commands that talk to hardware.
        Commands::Scan => device_commands::scan(&cli).map_err(Into::into),
        Commands::Detect => device_commands::detect(&cli).map_err(Into::into),
        Commands::Info => device_commands::info(&cli).map_err(Into::into),
        Commands::Read {
            output,
            start,
            length,
            oob,
        } => device_commands::read(&cli, output.clone(), start, length.as_deref(), *oob)
            .map_err(Into::into),
        Commands::Write {
            input,
            start,
            verify,
            erase,
        } => device_commands::write(&cli, input.clone(), start, *verify, *erase, cli.yes)
            .map_err(Into::into),
        Commands::Erase { start, length } => {
            device_commands::erase(&cli, start.as_deref(), length.as_deref(), cli.yes)
                .map_err(Into::into)
        }
        Commands::Verify { file, start } => {
            device_commands::verify(&cli, file.clone(), start).map_err(Into::into)
        }
        Commands::Analyze {
            input,
            output,
            deep,
            report_format,
        } => commands::analyze(&cli, input.clone(), output.clone(), *deep, report_format),
        Commands::Compare {
            file1,
            file2,
            output,
        } => commands::compare(&cli, file1.clone(), file2.clone(), output.clone()),
        Commands::Clone { .. } => commands::not_implemented(
            "chip-to-chip cloning",
            "It needs two devices open at once, which the device layer does not do \n\
             yet. In the meantime: `openflash read -o image.bin` from the source \n\
             chip, then `openflash write -i image.bin` to the destination.",
        ),
        Commands::Batch { .. } => commands::not_implemented(
            "batch jobs",
            "The batch job types exist in openflash_core::scripting but no runner \n\
             executes them. The previous implementation printed a fixed list of \n\
             three completed jobs without reading the file at all.",
        ),
        Commands::Script { .. } => commands::not_implemented(
            "script execution",
            "No script interpreter is embedded. Use the Python bindings \n\
             (pyopenflash) to drive the device from a script.",
        ),
        Commands::Chips {
            interface,
            manufacturer,
            search,
        } => commands::list_chips(
            &cli,
            interface.clone(),
            manufacturer.clone(),
            search.clone(),
        ),
        Commands::Interface { interface } => {
            device_commands::set_interface(&cli, interface).map_err(Into::into)
        }
        Commands::Config { action } => match action {
            ConfigAction::Show => commands::config_show(&cli),
            ConfigAction::Set { key, value } => commands::config_set(&cli, key, value),
            ConfigAction::Reset => commands::config_reset(&cli),
        },
        Commands::Unpack {
            input,
            output,
            depth,
            recursive,
        } => commands::unpack(&cli, input.clone(), output.clone(), *depth, *recursive),
        Commands::Rootfs {
            input,
            output,
            contents,
        } => commands::rootfs(&cli, input.clone(), output.clone(), *contents),
        Commands::Vulnscan {
            input,
            output,
            credentials,
            weak_crypto,
        } => commands::vulnscan(
            &cli,
            input.clone(),
            output.clone(),
            *credentials,
            *weak_crypto,
        ),
        Commands::Identify { input, top } => commands::identify(&cli, input.clone(), *top),
        Commands::Signatures { action } => match action {
            SignaturesAction::Load { file } => commands::signatures_load(&cli, file.clone()),
            SignaturesAction::Scan { input, signatures } => {
                commands::signatures_scan(&cli, input.clone(), signatures.clone())
            }
            SignaturesAction::Export { output } => {
                commands::signatures_export(&cli, output.clone())
            }
            SignaturesAction::List => commands::signatures_list(&cli),
        },
        // The server, device-farm, job-queue and production-line subsystems
        // exist only as data types in `openflash_core::server`: there is no HTTP,
        // WebSocket or gRPC implementation behind them. These commands used to
        // print a configuration summary and exit successfully, which read as
        // though a server had started.
        Commands::Server { .. }
        | Commands::Device { .. }
        | Commands::Job { .. }
        | Commands::ParallelDump { .. }
        | Commands::Production { .. } => commands::not_implemented(
            "server mode",
            "openflash_core::server defines the REST, WebSocket and gRPC types but \n\
             no server implements them yet. Track it at \n\
             https://github.com/bobberdolle1/openflash/issues",
        ),
    };

    if let Err(error) = result {
        // Always reported, including under --quiet: that flag suppresses progress
        // and summaries, not failures. Silencing the reason for a failed flash
        // and leaving only the exit status is how a script ends up logging
        // nothing useful about why a chip was not written.
        eprintln!("{} {error}", "Error:".red().bold());

        // Print the chain, so a verification failure shows the offset and a USB
        // failure shows the underlying errno rather than just the top layer.
        let mut source = error.source();
        while let Some(cause) = source {
            eprintln!("  caused by: {cause}");
            source = cause.source();
        }

        std::process::exit(1);
    }
}

fn print_banner() {
    eprintln!(
        "{}",
        format!(
            r#"
   ____                   _____ _           _     
  / __ \                 |  ___| |         | |    
 | |  | |_ __   ___ _ __ | |_  | | __ _ ___| |__  
 | |  | | '_ \ / _ \ '_ \|  _| | |/ _` / __| '_ \ 
 | |__| | |_) |  __/ | | | |   | | (_| \__ \ | | |
  \____/| .__/ \___|_| |_\_|   |_|\__,_|___/_| |_|
        | |                                       
        |_|   v{}
"#,
            env!("CARGO_PKG_VERSION")
        )
        .cyan()
    );
}

/// Create a progress bar with OpenFlash style
pub fn create_progress_bar(total: u64, message: &str) -> ProgressBar {
    let pb = ProgressBar::new(total);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta}) {msg}")
            .unwrap()
            .progress_chars("█▓▒░"),
    );
    pb.set_message(message.to_string());
    pb
}

/// Parse address string (supports hex 0x prefix)
pub fn parse_address(s: &str) -> Result<u64, String> {
    if s.starts_with("0x") || s.starts_with("0X") {
        u64::from_str_radix(&s[2..], 16).map_err(|e| e.to_string())
    } else {
        s.parse()
            .map_err(|e: std::num::ParseIntError| e.to_string())
    }
}

/// Format bytes as human-readable size
pub fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::Cli;
    use clap::CommandFactory;

    /// clap validates argument definitions only when a command is actually
    /// parsed, so a duplicate short flag panics at runtime for whichever
    /// subcommand carries it — `verify` was unusable because its `--file` also
    /// claimed `-f`, which the global `--format` already had. This asserts the
    /// whole tree up front.
    #[test]
    fn the_argument_definitions_are_internally_consistent() {
        Cli::command().debug_assert();
    }

    #[test]
    fn parsing_an_address_accepts_decimal_and_hex() {
        assert_eq!(super::parse_address("0"), Ok(0));
        assert_eq!(super::parse_address("4096"), Ok(4096));
        assert_eq!(super::parse_address("0x1000"), Ok(4096));
        assert_eq!(super::parse_address("0X1000"), Ok(4096));
        assert!(super::parse_address("nonsense").is_err());
        assert!(super::parse_address("0xZZ").is_err());
    }

    #[test]
    fn sizes_are_formatted_in_the_largest_sensible_unit() {
        assert_eq!(super::format_size(512), "512 B");
        assert_eq!(super::format_size(2048), "2.00 KB");
        assert_eq!(super::format_size(2 * 1024 * 1024), "2.00 MB");
        assert_eq!(super::format_size(3 * 1024 * 1024 * 1024), "3.00 GB");
    }
}
