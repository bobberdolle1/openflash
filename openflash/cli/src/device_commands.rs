//! Commands that talk to hardware.
//!
//! Each of these performs the operation it names, or exits non-zero saying why
//! it could not. The previous implementations of `write` and `erase` slept for
//! 100 ms and printed "Write complete!"; `read` filled the output file with
//! `0xFF` and printed a throughput figure derived from a hardcoded duration.

use std::fs::File;
use std::io::{BufWriter, Write as _};
use std::path::PathBuf;
use std::time::Duration;

use anyhow::{bail, Context, Result};
use colored::Colorize;
use indicatif::ProgressBar;

use openflash_core::device::ProgramOptions;
use openflash_core::protocol::FlashInterface;

use crate::connection::{self, Connection, Target};
use crate::{create_progress_bar, format_size, parse_address, Cli};

/// Build the connection the global flags describe.
fn connect(cli: &Cli) -> Result<(Connection, Target)> {
    let target = Target::from_flags(
        cli.device.as_deref(),
        cli.tcp.as_deref(),
        cli.unix.as_deref(),
        cli.emulate,
        cli.emulate_image.as_deref(),
    )?;
    connection::warn_if_emulated(&target, cli.quiet);

    let timeout = Duration::from_millis(cli.timeout_ms);
    let device = connection::open(&target, timeout)?;
    Ok((device, target))
}

/// Progress bar that reports actual transferred bytes, or nothing when quiet.
fn progress_bar(cli: &Cli, total: u64, message: &str) -> Option<ProgressBar> {
    (!cli.quiet).then(|| create_progress_bar(total, message))
}

/// `openflash scan` — list attached devices.
pub fn scan(cli: &Cli) -> Result<()> {
    let devices = connection::scan()?;

    if cli.format == "json" {
        println!("{}", serde_json::to_string_pretty(&devices)?);
        return Ok(());
    }

    if devices.is_empty() {
        // Not an error: nothing is plugged in. But say so plainly instead of
        // printing an invented device list, which is what this used to do.
        println!("{}", "No OpenFlash devices found.".yellow());
        println!(
            "  Connect a device, or use {} to exercise the tool without hardware.",
            "--emulate <bytes>".cyan()
        );
        return Ok(());
    }

    println!("{}", "Found devices:".green().bold());
    for device in devices {
        println!("  {} {device}", "●".green());
    }
    Ok(())
}

/// `openflash info` — what the device says about itself.
pub fn info(cli: &Cli) -> Result<()> {
    let (device, _) = connect(cli)?;
    let version = device.version();

    if cli.format == "json" {
        let json = serde_json::json!({
            "connection": device.kind().to_string(),
            "protocol_version": version.protocol,
            "firmware_version": format!(
                "{}.{}.{}",
                version.firmware.0, version.firmware.1, version.firmware.2
            ),
            "platform": version.platform.map(|p| p.name()),
            "interfaces": FlashInterface::ALL
                .iter()
                .filter(|i| version.supports(**i))
                .map(|i| i.as_str())
                .collect::<Vec<_>>(),
        });
        println!("{}", serde_json::to_string_pretty(&json)?);
        return Ok(());
    }

    println!("{}", "Device:".green().bold());
    println!("  Connection: {}", device.kind().to_string().cyan());
    println!(
        "  Platform:   {}",
        version
            .platform
            .map(|p| p.name())
            .unwrap_or("unknown")
            .yellow()
    );
    println!(
        "  Firmware:   {}.{}.{}",
        version.firmware.0, version.firmware.1, version.firmware.2
    );
    println!("  Protocol:   v{}", version.protocol);
    println!(
        "  Interfaces: {}",
        FlashInterface::ALL
            .iter()
            .filter(|i| version.supports(**i))
            .map(|i| i.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    );
    Ok(())
}

/// `openflash detect` — read the chip id and look it up.
pub fn detect(cli: &Cli) -> Result<()> {
    let (mut device, _) = connect(cli)?;
    let chip = device.identify()?;

    if cli.format == "json" {
        let json = serde_json::json!({
            "manufacturer": chip.manufacturer,
            "model": chip.model,
            "jedec_id": chip.jedec_id.iter().map(|b| format!("{b:02X}")).collect::<Vec<_>>(),
            "capacity": chip.capacity,
            "page_size": chip.page_size,
            "sector_size": chip.sector_size,
            "exact_match": chip.exact_match,
        });
        println!("{}", serde_json::to_string_pretty(&json)?);
        return Ok(());
    }

    println!("{}", "Detected chip:".green().bold());
    println!("  Manufacturer: {}", chip.manufacturer.cyan());
    println!("  Model:        {}", chip.model.cyan());
    println!("  Capacity:     {}", format_size(chip.capacity).yellow());
    println!("  Page size:    {} bytes", chip.page_size);
    println!("  Sector size:  {}", format_size(chip.sector_size as u64));
    println!(
        "  JEDEC ID:     {}",
        chip.jedec_id
            .iter()
            .map(|b| format!("{b:02X}"))
            .collect::<Vec<_>>()
            .join(" ")
    );
    if !chip.exact_match {
        // The database falls back to a capacity-derived guess. Saying so matters:
        // the page and sector sizes below are assumptions, not facts read off
        // the chip.
        println!(
            "  {} this id is not in the database; the geometry above is inferred \
             from the capacity byte",
            "note:".yellow()
        );
    }
    Ok(())
}

/// `openflash read` — dump the chip to a file.
pub fn read(
    cli: &Cli,
    output: PathBuf,
    start: &str,
    length: Option<&str>,
    oob: bool,
) -> Result<()> {
    let start_address = parse_address(start).map_err(anyhow::Error::msg)?;
    let requested = length
        .map(|l| parse_address(l).map_err(anyhow::Error::msg))
        .transpose()?;

    if oob {
        // Better to refuse than to write a file the user believes contains OOB.
        bail!(
            "--oob needs a NAND interface; this build's device layer covers SPI NOR, \
             which has no spare area"
        );
    }

    let (mut device, _) = connect(cli)?;
    // A dump must not be able to alter the chip it is reading.
    device.set_read_only(true);

    let chip = device.identify()?;
    let total = requested.unwrap_or(chip.capacity);

    if !cli.quiet {
        println!(
            "Reading {} from {} to {}",
            format_size(total).yellow(),
            chip.model.cyan(),
            output.display().to_string().cyan()
        );
    }

    let file =
        File::create(&output).with_context(|| format!("cannot create {}", output.display()))?;
    let mut writer = BufWriter::new(file);

    let bar = progress_bar(cli, total, "reading");
    let report = {
        let mut on_progress = |done: u64, _total: u64| {
            if let Some(bar) = &bar {
                bar.set_position(done);
            }
        };
        device.read_into(start_address, total, &mut writer, Some(&mut on_progress))?
    };
    writer.flush().context("cannot flush the dump to disk")?;
    if let Some(bar) = bar {
        bar.finish_and_clear();
    }

    if cli.quiet {
        return Ok(());
    }

    println!("{}", "Read complete.".green().bold());
    println!("  Bytes:    {}", format_size(report.bytes_read));
    println!("  Requests: {}", report.chunks);
    println!("  Duration: {:.2?}", report.duration);
    match report.bytes_per_second() {
        Some(speed) => println!("  Speed:    {}/s", format_size(speed)),
        None => println!("  Speed:    (too fast to measure)"),
    }
    Ok(())
}

/// `openflash write` — program a file onto the chip.
pub fn write(
    cli: &Cli,
    input: PathBuf,
    start: &str,
    verify: bool,
    erase: bool,
    yes: bool,
) -> Result<()> {
    let start_address = parse_address(start).map_err(anyhow::Error::msg)?;
    let data = std::fs::read(&input).with_context(|| format!("cannot read {}", input.display()))?;

    if data.is_empty() {
        bail!("{} is empty; nothing to write", input.display());
    }

    let (mut device, target) = connect(cli)?;
    let chip = device.identify()?;

    if !yes && !target.is_emulated() {
        confirm_destructive(
            cli,
            &format!(
                "About to write {} at {start_address:#x} on {} {} ({}){}",
                format_size(data.len() as u64),
                chip.manufacturer,
                chip.model,
                format_size(chip.capacity),
                if erase {
                    ", erasing the affected sectors first"
                } else {
                    ""
                }
            ),
        )?;
    }

    if !cli.quiet {
        println!(
            "Writing {} from {}",
            format_size(data.len() as u64).yellow(),
            input.display().to_string().cyan()
        );
    }

    let bar = progress_bar(cli, data.len() as u64, "writing");
    let report = {
        let mut on_progress = |done: u64, _total: u64| {
            if let Some(bar) = &bar {
                bar.set_position(done);
            }
        };
        device.program(
            start_address,
            &data,
            ProgramOptions {
                erase_first: erase,
                verify,
                skip_blank_pages: true,
            },
            Some(&mut on_progress),
        )?
    };
    if let Some(bar) = bar {
        bar.finish_and_clear();
    }

    if cli.quiet {
        return Ok(());
    }

    println!("{}", "Write complete.".green().bold());
    println!("  Bytes written:     {}", format_size(report.bytes_written));
    println!("  Pages programmed:  {}", report.pages_written);
    if report.pages_skipped > 0 {
        println!(
            "  Pages skipped:     {} (already erased)",
            report.pages_skipped
        );
    }
    println!("  Sectors erased:    {}", report.sectors_erased);
    if report.sectors_preserved > 0 {
        println!(
            "  Sectors preserved: {} (read back and rewritten so neighbouring data survived)",
            report.sectors_preserved
        );
    }
    println!(
        "  Verified:          {}",
        if report.verified {
            "yes, read back and compared".green()
        } else {
            "no (--verify=false)".yellow()
        }
    );
    Ok(())
}

/// `openflash erase` — erase sectors.
pub fn erase(cli: &Cli, start: Option<&str>, length: Option<&str>, yes: bool) -> Result<()> {
    let start_address = start
        .map(|s| parse_address(s).map_err(anyhow::Error::msg))
        .transpose()?
        .unwrap_or(0);

    let (mut device, target) = connect(cli)?;
    let chip = device.identify()?;

    let requested = length
        .map(|l| parse_address(l).map_err(anyhow::Error::msg))
        .transpose()?
        .unwrap_or(chip.capacity - start_address);

    if !yes && !target.is_emulated() {
        confirm_destructive(
            cli,
            &format!(
                "About to erase {} at {start_address:#x} on {} {}. \
                 Everything in that range will be lost.",
                format_size(requested),
                chip.manufacturer,
                chip.model
            ),
        )?;
    }

    if !cli.quiet {
        println!("Erasing {}...", format_size(requested).yellow());
    }

    let sectors = device.erase_range(start_address, requested)?;

    // An erase that claims success but left data behind is worse than a failure,
    // so read the range back and confirm it is blank.
    device.verify_erased(start_address, requested)?;

    if !cli.quiet {
        println!("{}", "Erase complete.".green().bold());
        println!("  Sectors erased: {sectors}");
        println!("  Confirmed blank by reading the range back.");
    }
    Ok(())
}

/// `openflash verify` — compare the chip against a file.
pub fn verify(cli: &Cli, file: PathBuf, start: &str) -> Result<()> {
    let start_address = parse_address(start).map_err(anyhow::Error::msg)?;
    let expected =
        std::fs::read(&file).with_context(|| format!("cannot read {}", file.display()))?;

    let (mut device, _) = connect(cli)?;
    device.set_read_only(true);

    if !cli.quiet {
        println!(
            "Verifying {} of chip contents against {}",
            format_size(expected.len() as u64).yellow(),
            file.display().to_string().cyan()
        );
    }

    let bar = progress_bar(cli, expected.len() as u64, "verifying");
    let outcome = {
        let mut on_progress = |done: u64, _total: u64| {
            if let Some(bar) = &bar {
                bar.set_position(done);
            }
        };
        device.verify(start_address, &expected, Some(&mut on_progress))
    };
    if let Some(bar) = bar {
        bar.finish_and_clear();
    }

    // A mismatch is a failure, and must set the exit status: this used to hold a
    // `let matches = true;` and print PASSED unconditionally.
    outcome?;

    if !cli.quiet {
        println!(
            "{} {} match byte for byte.",
            "Verification passed:".green().bold(),
            format_size(expected.len() as u64)
        );
    }
    Ok(())
}

/// `openflash interface` — select the active flash interface.
pub fn set_interface(cli: &Cli, interface: &str) -> Result<()> {
    let parsed = FlashInterface::parse(interface).ok_or_else(|| {
        anyhow::anyhow!(
            "unknown interface '{interface}'; valid values are {}",
            FlashInterface::ALL
                .iter()
                .map(|i| i.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        )
    })?;

    let (mut device, _) = connect(cli)?;
    device.set_interface(parsed)?;

    if !cli.quiet {
        println!("Interface set to {}.", parsed.as_str().cyan());
    }
    Ok(())
}

/// Ask before doing something irreversible.
fn confirm_destructive(cli: &Cli, what: &str) -> Result<()> {
    if cli.quiet {
        bail!("{what}\nRefusing to continue in --quiet mode without --yes");
    }

    eprintln!("{} {what}", "WARNING:".red().bold());
    eprint!("Type 'yes' to continue: ");
    std::io::stderr().flush().ok();

    let mut answer = String::new();
    std::io::stdin()
        .read_line(&mut answer)
        .context("cannot read the confirmation from stdin")?;

    if answer.trim() != "yes" {
        bail!("cancelled at the confirmation prompt");
    }
    Ok(())
}
