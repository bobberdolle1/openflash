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
//! ```

use clap::{Parser, Subcommand};
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::PathBuf;

mod commands;

/// OpenFlash - Open-source NAND/eMMC/NOR flash programmer
#[derive(Parser)]
#[command(name = "openflash")]
#[command(author = "OpenFlash Team")]
#[command(version = "1.8.0")]
#[command(about = "Command-line interface for flash programming and analysis")]
#[command(long_about = None)]
struct Cli {
    /// Output format (text, json, csv)
    #[arg(short = 'f', long, default_value = "text", global = true)]
    format: String,

    /// Verbose output
    #[arg(short, long, global = true)]
    verbose: bool,

    /// Quiet mode (minimal output)
    #[arg(short, long, global = true)]
    quiet: bool,

    /// Device port (auto-detect if not specified)
    #[arg(short = 'p', long, global = true)]
    port: Option<String>,

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

        /// Include OOB/spare area data
        #[arg(long)]
        oob: bool,

        /// Skip bad blocks
        #[arg(long, default_value = "true")]
        skip_bad: bool,
    },

    /// Write/program flash chip
    Write {
        /// Input file path
        #[arg(short, long)]
        input: PathBuf,

        /// Start address (hex or decimal)
        #[arg(short, long, default_value = "0")]
        start: String,

        /// Verify after write
        #[arg(long, default_value = "true")]
        verify: bool,

        /// Erase before write
        #[arg(long, default_value = "true")]
        erase: bool,

        /// Skip bad blocks
        #[arg(long, default_value = "true")]
        skip_bad: bool,
    },

    /// Erase flash chip (full or partial)
    Erase {
        /// Start address (hex or decimal)
        #[arg(short, long)]
        start: Option<String>,

        /// Length to erase (default: full chip)
        #[arg(short, long)]
        length: Option<String>,

        /// Force erase without confirmation
        #[arg(long)]
        force: bool,
    },

    /// Verify flash contents against file
    Verify {
        /// File to verify against
        #[arg(short, long)]
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
}

#[derive(Subcommand)]
enum ConfigAction {
    /// Show current configuration
    Show,
    /// Set configuration value
    Set {
        key: String,
        value: String,
    },
    /// Reset to defaults
    Reset,
}


fn main() {
    let cli = Cli::parse();

    if !cli.quiet {
        print_banner();
    }

    let result = match cli.command {
        Commands::Scan => commands::scan(&cli),
        Commands::Detect => commands::detect(&cli),
        Commands::Read { output, start, length, oob, skip_bad } => {
            commands::read(&cli, output, &start, length.as_deref(), oob, skip_bad)
        }
        Commands::Write { input, start, verify, erase, skip_bad } => {
            commands::write(&cli, input, &start, verify, erase, skip_bad)
        }
        Commands::Erase { start, length, force } => {
            commands::erase(&cli, start.as_deref(), length.as_deref(), force)
        }
        Commands::Verify { file, start } => {
            commands::verify(&cli, file, &start)
        }
        Commands::Analyze { input, output, deep, report_format } => {
            commands::analyze(&cli, input, output, deep, &report_format)
        }
        Commands::Compare { file1, file2, output } => {
            commands::compare(&cli, file1, file2, output)
        }
        Commands::Clone { mode, verify } => {
            commands::clone_chip(&cli, &mode, verify)
        }
        Commands::Batch { file, stop_on_error } => {
            commands::batch(&cli, file, stop_on_error)
        }
        Commands::Script { file, args } => {
            commands::script(&cli, file, args)
        }
        Commands::Chips { interface, manufacturer, search } => {
            commands::list_chips(&cli, interface, manufacturer, search)
        }
        Commands::Info => commands::info(&cli),
        Commands::Interface { interface } => {
            commands::set_interface(&cli, &interface)
        }
        Commands::Config { action } => {
            match action {
                ConfigAction::Show => commands::config_show(&cli),
                ConfigAction::Set { key, value } => commands::config_set(&cli, &key, &value),
                ConfigAction::Reset => commands::config_reset(&cli),
            }
        }
    };

    if let Err(e) = result {
        if !cli.quiet {
            eprintln!("{} {}", "Error:".red().bold(), e);
        }
        std::process::exit(1);
    }
}

fn print_banner() {
    println!("{}", r#"
   ____                   _____ _           _     
  / __ \                 |  ___| |         | |    
 | |  | |_ __   ___ _ __ | |_  | | __ _ ___| |__  
 | |  | | '_ \ / _ \ '_ \|  _| | |/ _` / __| '_ \ 
 | |__| | |_) |  __/ | | | |   | | (_| \__ \ | | |
  \____/| .__/ \___|_| |_\_|   |_|\__,_|___/_| |_|
        | |                                       
        |_|   v1.8.0 - Scripting & Automation     
"#.cyan());
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
        s.parse().map_err(|e: std::num::ParseIntError| e.to_string())
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
