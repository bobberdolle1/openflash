//! CLI commands that work on files already on disk.
//!
//! Commands that talk to a device live in [`crate::device_commands`].

use crate::{format_size, Cli};
use colored::Colorize;
use openflash_core::scripting::*;
use std::path::PathBuf;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

/// AI analysis
pub fn analyze(
    cli: &Cli,
    input: Option<PathBuf>,
    output: Option<PathBuf>,
    deep: bool,
    report_format: &str,
) -> Result<()> {
    let data = if let Some(path) = &input {
        std::fs::read(path)?
    } else {
        return Err("No input file specified".into());
    };

    if !cli.quiet {
        println!(
            "{} {} ({})...",
            "Analyzing".cyan(),
            input
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_default()
                .yellow(),
            format_size(data.len() as u64)
        );
        if deep {
            println!("  Deep scan: {}", "enabled".yellow());
        }
    }

    if data.is_empty() {
        return Err("the input file is empty".into());
    }

    let result = analyse_bytes(&data, deep);

    match cli.format.as_str() {
        "json" => println!("{}", serde_json::to_string_pretty(&result)?),
        _ => {
            println!("\n{}", "Analysis Results:".green().bold());
            println!("  Quality:     {:.0}%", result.quality_score * 100.0);
            println!(
                "  Encryption:  {:.0}%",
                result.encryption_probability * 100.0
            );
            println!(
                "  Compression: {:.0}%",
                result.compression_probability * 100.0
            );
            println!("\n{}", "Detected patterns:".cyan());
            if result.patterns.is_empty() {
                println!("  {}", "none".dimmed());
            }
            for p in &result.patterns {
                println!(
                    "  {} @ 0x{:X} ({}, {:.0}% confidence)",
                    p.pattern_type.yellow(),
                    p.offset,
                    format_size(p.size),
                    p.confidence * 100.0
                );
            }

            // Filesystems and anomalies were computed and then dropped on the
            // floor by the text output, which is what `analyze` is mostly for.
            println!("\n{}", "Filesystems:".cyan());
            if result.filesystems.is_empty() {
                println!("  {}", "none found".dimmed());
            }
            for fs in &result.filesystems {
                let size = match fs.size {
                    Some(size) => format_size(size),
                    None => "size unknown".to_string(),
                };
                println!(
                    "  {} @ 0x{:X} ({}, {:.0}% confidence)",
                    fs.fs_type.yellow(),
                    fs.offset,
                    size,
                    fs.confidence * 100.0
                );
            }

            if !result.anomalies.is_empty() {
                println!("\n{}", "Anomalies:".cyan());
                for anomaly in &result.anomalies {
                    println!(
                        "  [{}] {} @ 0x{:X}",
                        anomaly.severity.red(),
                        anomaly.anomaly_type,
                        anomaly.offset
                    );
                    if !anomaly.description.is_empty() {
                        println!("      {}", anomaly.description.dimmed());
                    }
                }
            }

            if !result.key_candidates.is_empty() {
                println!("\n{}", "Possible key material:".cyan());
                for key in &result.key_candidates {
                    println!(
                        "  {} @ 0x{:X} ({:.0}% confidence)",
                        key.key_type.yellow(),
                        key.offset,
                        key.confidence * 100.0
                    );
                }
            }

            println!("\n{}", result.summary.dimmed());
        }
    }

    if let Some(out) = output {
        let report = generate_report(&result, report_format);
        std::fs::write(&out, report)?;
        if !cli.quiet {
            println!("\nReport saved to: {}", out.display().to_string().cyan());
        }
    }
    Ok(())
}

/// Page geometry assumed when analysing a file on disk.
///
/// A dump taken through `read` carries no geometry with it — it is just bytes —
/// and the analyser needs a page size to reason about per-page statistics.
/// 2048-byte pages in 64-page blocks is the common NAND layout. Signature and
/// entropy detection do not depend on it; only the per-page anomaly scan does.
const ASSUMED_PAGE_SIZE: usize = 2048;
const ASSUMED_PAGES_PER_BLOCK: usize = 64;

/// Run the real analyser over a dump and convert its result for reporting.
///
/// This used to be a hardcoded literal: `analyze` read the file, ignored it, and
/// printed a SquashFS at 0x10000 and a U-Boot image at 0 with fixed confidences,
/// for any input at all. The engine it should have been calling lives in
/// `openflash_core::ai` and is what the Python bindings already use.
fn analyse_bytes(data: &[u8], deep: bool) -> ScriptAnalysisResult {
    use openflash_core::ai::AiAnalyzer;

    let analysis = AiAnalyzer::new(ASSUMED_PAGE_SIZE, ASSUMED_PAGES_PER_BLOCK)
        .with_deep_scan(deep)
        .analyze(data);

    ScriptAnalysisResult {
        quality_score: analysis.data_quality_score,
        encryption_probability: analysis.encryption_probability,
        compression_probability: analysis.compression_probability,
        patterns: analysis
            .patterns
            .iter()
            .map(|pattern| PatternInfo {
                pattern_type: format!("{:?}", pattern.pattern_type),
                offset: pattern.start_offset as u64,
                size: pattern.end_offset.saturating_sub(pattern.start_offset) as u64,
                confidence: pattern.confidence.to_score(),
            })
            .collect(),
        filesystems: analysis
            .filesystems
            .iter()
            .map(|fs| FilesystemInfo {
                fs_type: format!("{:?}", fs.fs_type),
                offset: fs.offset as u64,
                size: fs.size.map(|size| size as u64),
                confidence: fs.confidence.to_score(),
            })
            .collect(),
        anomalies: analysis
            .anomalies
            .iter()
            .map(|anomaly| AnomalyInfo {
                anomaly_type: anomaly.description.clone(),
                severity: format!("{:?}", anomaly.severity),
                offset: anomaly.location.unwrap_or(0) as u64,
                description: anomaly.recommendation.clone(),
            })
            .collect(),
        recovery_suggestions: analysis
            .recovery_suggestions
            .iter()
            .map(|suggestion| RecoverySuggestion {
                action: suggestion.action.clone(),
                description: suggestion.description.clone(),
                success_probability: suggestion.estimated_success,
                priority: suggestion.priority,
            })
            .collect(),
        key_candidates: analysis
            .key_candidates
            .iter()
            .map(|key| KeyCandidate {
                key_type: key.key_type.clone(),
                offset: key.offset as u64,
                // The engine reports where and how long, not the bytes; copying
                // key material into a report is not something to do by default.
                key_data: Vec::new(),
                confidence: key.confidence.to_score(),
            })
            .collect(),
        summary: analysis.summary,
    }
}

fn generate_report(result: &ScriptAnalysisResult, format: &str) -> String {
    match format {
        "json" => serde_json::to_string_pretty(result).unwrap_or_default(),
        "html" => format!(
            "<html><body><h1>OpenFlash Analysis Report</h1><pre>{}</pre></body></html>",
            serde_json::to_string_pretty(result).unwrap_or_default()
        ),
        _ => format!(
            "# OpenFlash Analysis Report\n\n## Summary\n{}\n\n## Quality: {:.0}%\n",
            result.summary,
            result.quality_score * 100.0
        ),
    }
}

/// Compare two dumps
pub fn compare(cli: &Cli, file1: PathBuf, file2: PathBuf, output: Option<PathBuf>) -> Result<()> {
    let data1 = std::fs::read(&file1)?;
    let data2 = std::fs::read(&file2)?;

    if !cli.quiet {
        println!(
            "{} {} vs {}",
            "Comparing".cyan(),
            file1.display().to_string().yellow(),
            file2.display().to_string().yellow()
        );
    }

    let overlap = data1.len().min(data2.len());
    let mut differing_offsets = Vec::new();
    for index in 0..overlap {
        if data1[index] != data2[index] {
            differing_offsets.push(index);
        }
    }
    let length_difference = data1.len().abs_diff(data2.len());
    let diffs = differing_offsets.len() + length_difference;

    let longest = data1.len().max(data2.len());
    let similarity = if longest == 0 {
        1.0
    } else {
        1.0 - (diffs as f64 / longest as f64)
    };

    let mut report = String::new();
    report.push_str("# OpenFlash dump comparison\n\n");
    report.push_str(&format!(
        "- File 1: {} ({} bytes)\n",
        file1.display(),
        data1.len()
    ));
    report.push_str(&format!(
        "- File 2: {} ({} bytes)\n",
        file2.display(),
        data2.len()
    ));
    report.push_str(&format!(
        "- Differing bytes in the overlap: {}\n",
        differing_offsets.len()
    ));
    report.push_str(&format!("- Length difference: {length_difference} bytes\n"));
    report.push_str(&format!("- Similarity: {:.4}%\n", similarity * 100.0));
    if let Some(first) = differing_offsets.first() {
        report.push_str(&format!(
            "- First difference at {first:#x}: {:#04x} vs {:#04x}\n",
            data1[*first], data2[*first]
        ));
        report.push_str("\n## Differing offsets (first 1000)\n\n");
        for offset in differing_offsets.iter().take(1000) {
            report.push_str(&format!(
                "{offset:#010x}  {:#04x} -> {:#04x}\n",
                data1[*offset], data2[*offset]
            ));
        }
        if differing_offsets.len() > 1000 {
            report.push_str(&format!(
                "\n… and {} more\n",
                differing_offsets.len() - 1000
            ));
        }
    }

    if let Some(path) = &output {
        std::fs::write(path, &report)?;
    }

    if cli.format == "json" {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "file1": { "path": file1.display().to_string(), "size": data1.len() },
                "file2": { "path": file2.display().to_string(), "size": data2.len() },
                "differing_bytes": differing_offsets.len(),
                "length_difference": length_difference,
                "similarity": similarity,
                "first_difference": differing_offsets.first(),
            }))?
        );
        return Ok(());
    }

    println!("\n{}", "Comparison results:".green().bold());
    println!("  File 1 size: {}", format_size(data1.len() as u64));
    println!("  File 2 size: {}", format_size(data2.len() as u64));
    println!("  Differences: {diffs} bytes");
    println!("  Similarity:  {:.2}%", similarity * 100.0);
    match differing_offsets.first() {
        Some(first) => println!("  First diff:  {first:#x}"),
        None if length_difference == 0 => println!("  {}", "Files are identical.".green()),
        None => println!("  Common prefix is identical; the files differ in length only."),
    }
    if let Some(path) = &output {
        println!("  Report:      {}", path.display().to_string().cyan());
    }
    Ok(())
}

/// Parse a chip id given as hex, with or without separators.
///
/// `EF4018`, `ef 40 18` and `EF:40:18` all mean the same three bytes.
fn parse_chip_id(text: &str) -> Result<Vec<u8>> {
    let cleaned: String = text
        .chars()
        .filter(|c| !c.is_whitespace() && *c != ':' && *c != '-' && *c != ',')
        .collect();
    let cleaned = cleaned
        .strip_prefix("0x")
        .or_else(|| cleaned.strip_prefix("0X"))
        .unwrap_or(&cleaned);

    if cleaned.is_empty() || cleaned.len() % 2 != 0 {
        return Err(format!(
            "chip id must be an even number of hex digits, got {:?}",
            text
        )
        .into());
    }

    (0..cleaned.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&cleaned[i..i + 2], 16)
                .map_err(|_| format!("{:?} is not hex", &cleaned[i..i + 2]).into())
        })
        .collect()
}

/// Look an id up in the chip databases and report what matches.
///
/// The databases are keyed by id, so each one is asked directly rather than
/// scanned. A single id can match more than one of them — the leading byte is a
/// JEDEC manufacturer code shared across product lines — so every hit is
/// reported rather than the first.
///
/// Without `--interface` only *exact* catalogue entries count. Each database
/// also has a fallback that derives geometry from the capacity byte, and those
/// answer for almost any id: searching all four with fallbacks enabled reports a
/// "Generic SPI NAND" for a parallel NAND id, which is noise dressed as a
/// result. Naming an interface says which catalogue is the right one to ask, and
/// then the derived answer is useful — so it is shown, and labelled as derived.
fn lookup_chip_id(cli: &Cli, id: &[u8], interface: Option<&str>) -> Result<()> {
    use openflash_core::{emmc, onfi, spi_nand, spi_nor};

    let wanted = interface.map(normalise_interface).transpose()?;
    let allow_derived = wanted.is_some();
    let mut hits: Vec<serde_json::Value> = Vec::new();

    if wanted.is_none() || wanted == Some("spi-nor") {
        if let Ok(jedec) = <[u8; 3]>::try_from(id) {
            let exact = spi_nor::get_spi_nor_chip_info_exact(&jedec);
            let found = exact.is_some();
            let chip = exact.or_else(|| {
                allow_derived
                    .then(|| spi_nor::get_spi_nor_chip_info(&jedec))
                    .flatten()
            });
            if let Some(chip) = chip {
                hits.push(serde_json::json!({
                    "interface": "spi-nor",
                    "exact": found,
                    "manufacturer": chip.manufacturer,
                    "model": chip.model,
                    "size_bytes": chip.size_bytes,
                    "page_size": chip.page_size,
                    "sector_size": chip.sector_size,
                    "block_size": chip.block_size,
                    "voltage": chip.voltage,
                    "max_clock_mhz": chip.max_clock_mhz,
                    "address_bytes": chip.address_bytes,
                }));
            }
        }
    }

    if (wanted.is_none() || wanted == Some("spi-nand")) && id.len() >= 2 {
        let device = if id.len() >= 3 { &id[1..3] } else { &id[1..2] };
        let exact = spi_nand::get_spi_nand_chip_info_exact(id[0], device);
        let found = exact.is_some();
        let chip = exact.or_else(|| {
            allow_derived
                .then(|| spi_nand::get_spi_nand_chip_info(id))
                .flatten()
        });
        if let Some(chip) = chip {
            hits.push(serde_json::json!({
                "interface": "spi-nand",
                "exact": found,
                "manufacturer": chip.manufacturer,
                "model": chip.model,
                "size_mb": chip.size_mb,
                "page_size": chip.page_size,
                "pages_per_block": chip.block_size,
                "oob_size": chip.oob_size,
                "voltage": chip.voltage,
                "max_clock_mhz": chip.max_clock_mhz,
            }));
        }
    }

    if wanted.is_none() || wanted == Some("nand") {
        let exact = onfi::get_chip_info_exact(id);
        let found = exact.is_some();
        let chip = exact.or_else(|| allow_derived.then(|| onfi::get_chip_info(id)).flatten());
        if let Some(chip) = chip {
            hits.push(serde_json::json!({
                "interface": "nand",
                "exact": found,
                "manufacturer": chip.manufacturer,
                "model": chip.model,
                "size_mb": chip.size_mb,
                "page_size": chip.page_size,
                "pages_per_block": chip.block_size,
                "oob_size": chip.oob_size,
                "voltage": chip.voltage,
                "bus_width": chip.bus_width,
            }));
        }
    }

    // eMMC is keyed by the 16-byte CID rather than a short id, and its fallback
    // is recognisable by the model it builds, so exactness is read off that.
    if wanted.is_none() || wanted == Some("emmc") {
        if let Some(chip) = emmc::get_emmc_chip_info(id) {
            let found = !chip.model.starts_with("Generic eMMC");
            if found || allow_derived {
                hits.push(serde_json::json!({
                    "interface": "emmc",
                    "exact": found,
                    "manufacturer": chip.manufacturer,
                    "model": chip.model,
                    "size_gb": chip.size_gb,
                    "sector_size": chip.sector_size,
                    "voltage": chip.voltage,
                    "max_clock_mhz": chip.max_clock_mhz,
                }));
            }
        }
    }

    if cli.format == "json" {
        println!("{}", serde_json::to_string_pretty(&hits)?);
        return Ok(());
    }

    let printable: Vec<String> = id.iter().map(|b| format!("{b:02X}")).collect();
    if hits.is_empty() {
        println!(
            "\n{} matches no catalogued chip.",
            printable.join(" ").white()
        );
        println!(
            "  {}",
            "That is normal for a part that is not in the database. Naming an\n  \
             interface (--interface spi-nor) will also report geometry derived\n  \
             from the id where the format allows it."
                .dimmed()
        );
        return Ok(());
    }

    println!(
        "\n{}",
        format!("Chip id {}", printable.join(" ")).green().bold()
    );
    for hit in &hits {
        let object = hit.as_object().expect("built as an object above");
        let derived = object["exact"] == serde_json::json!(false);
        println!(
            "  {} {} ({}){}",
            object["manufacturer"].as_str().unwrap_or("?").cyan(),
            object["model"].as_str().unwrap_or("?").white(),
            object["interface"].as_str().unwrap_or("?").dimmed(),
            if derived {
                "  [derived from the id, not a catalogue entry]".yellow()
            } else {
                "".normal()
            },
        );
        for (key, value) in object {
            if matches!(
                key.as_str(),
                "manufacturer" | "model" | "interface" | "exact"
            ) {
                continue;
            }
            println!("      {}: {}", key.dimmed(), value);
        }
    }
    Ok(())
}

/// Map the interface names the flags accept onto one spelling.
fn normalise_interface(name: &str) -> Result<&'static str> {
    match name.to_lowercase().replace('_', "-").as_str() {
        "nand" | "parallel-nand" => Ok("nand"),
        "spi-nand" => Ok("spi-nand"),
        "spi-nor" | "nor" => Ok("spi-nor"),
        "emmc" => Ok("emmc"),
        other => Err(format!(
            "unknown interface {other:?}; expected nand, spi-nand, spi-nor or emmc"
        )
        .into()),
    }
}

/// Query the chip databases.
///
/// # Why there is no full listing yet
///
/// This used to print five hardcoded parts under the heading "Supported chips",
/// which had nothing to do with the databases the rest of the tool uses — those
/// hold 207 parts across four interfaces. Anyone checking whether their chip was
/// supported got an answer invented for the occasion.
///
/// Lookup by id is served properly, because the databases are written as
/// `match` arms keyed by id and that is exactly the question they can answer.
/// Listing and filtering need the same data as a table that can be iterated,
/// which is a refactor of all four modules rather than a change here, so those
/// flags report that instead of guessing.
pub fn list_chips(
    cli: &Cli,
    id: Option<String>,
    interface: Option<String>,
    manufacturer: Option<String>,
    search: Option<String>,
) -> Result<()> {
    if let Some(id) = id {
        let bytes = parse_chip_id(&id)?;
        return lookup_chip_id(cli, &bytes, interface.as_deref());
    }

    // Validate the interface even when it cannot be used yet, so a typo is
    // reported as a typo rather than swallowed by the message below.
    if let Some(name) = interface.as_deref() {
        normalise_interface(name)?;
    }

    let asked_to_filter = manufacturer.is_some() || search.is_some() || interface.is_some();
    not_implemented(
        if asked_to_filter {
            "filtering the chip database"
        } else {
            "listing the chip database"
        },
        "The databases hold 207 parts -- 70 SPI NOR, 65 parallel NAND, 45 SPI NAND\n\
         and 27 eMMC -- but they are written as `match` arms keyed by chip id, so\n\
         they can be queried and not yet enumerated.\n\n\
         Look a part up by its id instead:\n\
         \x20   openflash chips --id EF4018\n\
         \x20   openflash chips --id \"EC F1 00 95 40\"\n\n\
         Or read the id off the chip with `openflash detect`.",
    )
}

/// Show config
/// # Not implemented
///
/// There is no configuration file. This printed four fixed lines —
/// `default_port: auto`, `baud_rate: 115200`, `verify_writes: true`,
/// `skip_bad_blocks: true` — that were not read from anywhere and that no other
/// part of the tool consults. Two of them describe a serial link this protocol
/// does not use, and `skip_bad_blocks` names behaviour that does not exist.
pub fn config_show(_cli: &Cli) -> Result<()> {
    not_implemented(
        "configuration",
        "There is no configuration file: nothing persists between runs, and the\n\
         values this used to print were not read from anywhere.\n\n\
         Pass what you need on the command line instead -- `--device`, `--tcp`,\n\
         `--unix`, `--emulate` -- or set OPENFLASH_TCP on a board agent to make it\n\
         serve over the network.",
    )
}

/// # Not implemented
///
/// This printed `Set <key> = <value>` and stored nothing, so it reported success
/// for a write that never happened, including for keys that do not exist.
pub fn config_set(_cli: &Cli, key: &str, _value: &str) -> Result<()> {
    Err(format!(
        "cannot set {key:?}: there is no configuration file, and this command \
         previously reported success without storing anything. Pass options on \
         the command line instead."
    )
    .into())
}

/// # Not implemented
///
/// Reset what there is nothing of; it printed a success message unconditionally.
pub fn config_reset(_cli: &Cli) -> Result<()> {
    not_implemented(
        "configuration",
        "Nothing is stored, so there is nothing to reset.",
    )
}

// ============================================================================
// v1.9 - Advanced AI Features Commands
// ============================================================================

use openflash_core::ai_advanced::*;

/// Unpack firmware (binwalk-like)
pub fn unpack(
    cli: &Cli,
    input: PathBuf,
    output: PathBuf,
    depth: u32,
    recursive: bool,
) -> Result<()> {
    let data = std::fs::read(&input)?;

    if !cli.quiet {
        println!(
            "{} {} ({})...",
            "Unpacking".cyan(),
            input.display().to_string().yellow(),
            format_size(data.len() as u64)
        );
        println!("  Output: {}", output.display().to_string().cyan());
        println!("  Max depth: {}", depth);
        println!(
            "  Recursive: {}",
            if recursive { "yes".green() } else { "no".red() }
        );
    }

    let unpacker = FirmwareUnpacker::new()
        .with_max_depth(depth)
        .with_recursive(recursive);

    let result = unpacker.unpack(&data).map_err(|e| e.to_string())?;

    match cli.format.as_str() {
        "json" => println!("{}", serde_json::to_string_pretty(&result)?),
        _ => {
            println!("\n{}", "Unpack Results:".green().bold());
            println!("  Sections found: {}", result.total_sections);
            println!("  Extracted size: {}", format_size(result.extracted_size));

            if !result.sections.is_empty() {
                println!("\n{}", "Detected sections:".cyan());
                for section in &result.sections {
                    println!(
                        "  {} @ 0x{:08X} ({}) - {}",
                        section.name.yellow(),
                        section.offset,
                        format_size(section.size),
                        section.section_type.dimmed()
                    );
                }
            }

            if !result.warnings.is_empty() {
                println!("\n{}", "Warnings:".yellow());
                for warning in &result.warnings {
                    println!("  ⚠ {}", warning);
                }
            }
        }
    }

    // Create output directory and save sections
    std::fs::create_dir_all(&output)?;
    for (i, section) in result.sections.iter().enumerate() {
        if let Some(data) = &section.data {
            let filename = format!("{:02}_{}.bin", i, section.name.replace(['/', ' '], "_"));
            let path = output.join(&filename);
            std::fs::write(&path, data)?;
            if !cli.quiet {
                println!("  Saved: {}", path.display().to_string().dimmed());
            }
        }
    }

    Ok(())
}

/// Extract root filesystem
pub fn rootfs(cli: &Cli, input: PathBuf, output: PathBuf, contents: bool) -> Result<()> {
    let data = std::fs::read(&input)?;

    if !cli.quiet {
        println!(
            "{} from {} ...",
            "Extracting rootfs".cyan(),
            input.display().to_string().yellow()
        );
    }

    let extractor = RootfsExtractor::new().with_contents(contents);
    let results = extractor.extract(&data).map_err(|e| e.to_string())?;

    match cli.format.as_str() {
        "json" => println!("{}", serde_json::to_string_pretty(&results)?),
        _ => {
            if results.is_empty() {
                println!("{}", "No filesystems found.".yellow());
                return Ok(());
            }

            println!("\n{}", "Found filesystems:".green().bold());
            for (i, fs) in results.iter().enumerate() {
                println!(
                    "\n  {}. {} @ 0x{:08X} ({})",
                    i + 1,
                    format!("{}", fs.fs_type).cyan(),
                    fs.offset,
                    format_size(fs.size)
                );
                if fs.files.is_empty() {
                    // "Files: 0" on its own reads as an empty filesystem. It
                    // means the listing was not attempted, which is different.
                    println!("     {}", "Contents not enumerated".yellow());
                    for warning in &fs.warnings {
                        println!("       {}", warning.dimmed());
                    }
                } else {
                    println!(
                        "     Files: {}, Directories: {}",
                        fs.total_files, fs.total_dirs
                    );
                }

                if !fs.files.is_empty() && cli.verbose {
                    println!("     {}", "Contents:".dimmed());
                    for file in fs.files.iter().take(10) {
                        let icon = if file.is_dir { "📁" } else { "📄" };
                        println!("       {} {} ({:o})", icon, file.path, file.mode);
                    }
                    if fs.files.len() > 10 {
                        println!("       ... and {} more", fs.files.len() - 10);
                    }
                }
            }
        }
    }

    // Create output directory
    std::fs::create_dir_all(&output)?;

    for (i, fs) in results.iter().enumerate() {
        let fs_dir = output.join(format!("fs{}_{}", i, fs.fs_type));
        std::fs::create_dir_all(&fs_dir)?;

        // `_files.txt` used to be written from a fabricated listing, so a file
        // full of plausible paths landed on disk for any image. When there is no
        // listing, say so in the file rather than leaving an empty one that
        // reads as "this filesystem is empty".
        let listing_path = fs_dir.join("_files.txt");
        let listing = if fs.files.is_empty() {
            let mut text = format!(
                "# No file listing: {} contents cannot be enumerated yet.\n",
                fs.fs_type
            );
            for warning in &fs.warnings {
                text.push_str(&format!("# {warning}\n"));
            }
            text.push_str(&format!(
                "# The image itself is at offset {:#x}, {} bytes.\n",
                fs.offset, fs.size
            ));
            text
        } else {
            fs.files
                .iter()
                .map(|f| f.path.as_str())
                .collect::<Vec<_>>()
                .join("\n")
        };
        std::fs::write(&listing_path, listing)?;

        // Carve the filesystem image out so another tool can open it. That is
        // the useful thing this command can honestly do today.
        let end = (fs.offset as usize)
            .saturating_add(fs.size as usize)
            .min(data.len());
        let start = (fs.offset as usize).min(end);
        if end > start {
            let image_path = fs_dir.join(format!("image.{}", fs.fs_type).to_lowercase());
            std::fs::write(&image_path, &data[start..end])?;
            if !cli.quiet {
                println!(
                    "  Carved {} to {}",
                    format_size((end - start) as u64),
                    image_path.display().to_string().cyan()
                );
            }
        }

        if !cli.quiet {
            println!("Wrote: {}", fs_dir.display().to_string().cyan());
        }
    }

    Ok(())
}

/// Scan for vulnerabilities
pub fn vulnscan(
    cli: &Cli,
    input: PathBuf,
    output: Option<PathBuf>,
    credentials: bool,
    weak_crypto: bool,
) -> Result<()> {
    let data = std::fs::read(&input)?;

    if !cli.quiet {
        println!(
            "{} {} ...",
            "Scanning".red(),
            input.display().to_string().yellow()
        );
    }

    let scanner = VulnScanner::new()
        .with_credentials_check(credentials)
        .with_weak_crypto_check(weak_crypto);

    let result = scanner.scan(&data).map_err(|e| e.to_string())?;

    match cli.format.as_str() {
        "json" => println!("{}", serde_json::to_string_pretty(&result)?),
        _ => {
            println!("\n{}", "Vulnerability Scan Results:".green().bold());
            println!("  Total found:  {}", result.total);
            println!("  {} Critical:  {}", "🔴".red(), result.critical);
            println!("  {} High:      {}", "🟠".yellow(), result.high);
            println!("  {} Medium:    {}", "🟡".yellow(), result.medium);
            println!("  {} Low:       {}", "🟢".green(), result.low);
            println!("  Scan time:    {} ms", result.scan_duration_ms);
            println!("  Signatures:   {}", result.signatures_checked);

            if !result.vulnerabilities.is_empty() {
                println!("\n{}", "Vulnerabilities:".red().bold());
                for vuln in &result.vulnerabilities {
                    let severity_icon = match vuln.cvss.severity {
                        Severity::Critical => "🔴",
                        Severity::High => "🟠",
                        Severity::Medium => "🟡",
                        Severity::Low => "🟢",
                        Severity::Info => "🔵",
                    };

                    println!(
                        "\n  {} {} (CVSS: {:.1})",
                        severity_icon,
                        vuln.name.red(),
                        vuln.cvss.base_score
                    );

                    if let Some(cve) = &vuln.cve_id {
                        println!("     CVE: {}", cve.cyan());
                    }
                    println!("     Offset: 0x{:08X}", vuln.offset);
                    println!("     {}", vuln.description.dimmed());
                    println!("     Fix: {}", vuln.remediation.green());
                }
            }
        }
    }

    if let Some(out) = output {
        let report = serde_json::to_string_pretty(&result)?;
        std::fs::write(&out, report)?;
        if !cli.quiet {
            println!("\nReport saved to: {}", out.display().to_string().cyan());
        }
    }

    Ok(())
}

/// ML-based chip identification
pub fn identify(cli: &Cli, input: PathBuf, top: usize) -> Result<()> {
    let data = std::fs::read(&input)?;

    if !cli.quiet {
        println!(
            "{} {} ...",
            "Identifying chip from".cyan(),
            input.display().to_string().yellow()
        );
    }

    let identifier = MlChipIdentifier::new();
    let predictions = identifier.identify(&data).map_err(|e| e.to_string())?;

    match cli.format.as_str() {
        "json" => println!("{}", serde_json::to_string_pretty(&predictions)?),
        _ => {
            let model_info = identifier.model_info();
            println!("\n{}", "ML Model Info:".dimmed());
            println!(
                "  Version: {}, Accuracy: {:.0}%, Chips: {}",
                model_info.version,
                model_info.accuracy * 100.0,
                model_info.supported_chips
            );

            println!("\n{}", "Chip Predictions:".green().bold());
            for (i, pred) in predictions.iter().take(top).enumerate() {
                let confidence_bar = "█".repeat((pred.confidence * 20.0) as usize);
                let confidence_empty = "░".repeat(20 - (pred.confidence * 20.0) as usize);

                println!(
                    "\n  {}. {} {}",
                    i + 1,
                    pred.manufacturer.cyan(),
                    pred.model.white().bold()
                );
                println!(
                    "     Confidence: [{}{}] {:.1}%",
                    confidence_bar.green(),
                    confidence_empty.dimmed(),
                    pred.confidence * 100.0
                );
                println!(
                    "     Page: {} bytes, Block: {}, Capacity: {}",
                    pred.page_size,
                    format_size(pred.block_size as u64),
                    format_size(pred.capacity)
                );
                println!("     Interface: {}", pred.interface.yellow());
            }
        }
    }

    Ok(())
}

/// Load custom signatures
pub fn signatures_load(cli: &Cli, file: PathBuf) -> Result<()> {
    let yaml = std::fs::read_to_string(&file)?;
    let mut db = SignatureDatabase::new("custom");
    let count = db.load_yaml(&yaml).map_err(|e| e.to_string())?;

    if !cli.quiet {
        println!(
            "{} {} signatures from {}",
            "Loaded".green(),
            count,
            file.display().to_string().cyan()
        );
    }
    Ok(())
}

/// Scan with custom signatures
pub fn signatures_scan(cli: &Cli, input: PathBuf, signatures: Option<PathBuf>) -> Result<()> {
    let data = std::fs::read(&input)?;

    let mut db = SignatureDatabase::new("scan");

    // Load signatures if provided
    if let Some(sig_file) = signatures {
        let yaml = std::fs::read_to_string(&sig_file)?;
        db.load_yaml(&yaml).map_err(|e| e.to_string())?;
    } else {
        // Add some default signatures
        db.add(CustomSignature {
            id: "backdoor_shell".to_string(),
            name: "Reverse Shell".to_string(),
            description: "Potential reverse shell pattern".to_string(),
            category: SignatureCategory::Backdoor,
            pattern: PatternType::Hex(b"/bin/sh -i".to_vec()),
            severity: Severity::Critical,
            author: Some("OpenFlash".to_string()),
            created: None,
            tags: vec!["backdoor".to_string()],
        });
    }

    if !cli.quiet {
        println!(
            "{} {} with {} signatures...",
            "Scanning".cyan(),
            input.display().to_string().yellow(),
            db.list().len()
        );
    }

    let matches = db.scan(&data);

    match cli.format.as_str() {
        "json" => println!("{}", serde_json::to_string_pretty(&matches)?),
        _ => {
            if matches.is_empty() {
                println!("{}", "No signature matches found.".green());
            } else {
                println!(
                    "\n{} ({} matches)",
                    "Signature Matches:".yellow().bold(),
                    matches.len()
                );
                for m in &matches {
                    println!("\n  {} @ 0x{:08X}", m.signature.name.red(), m.offset);
                    println!("     Category: {:?}", m.signature.category);
                    println!("     Severity: {}", m.signature.severity);
                }
            }
        }
    }

    Ok(())
}

/// Export signature database
pub fn signatures_export(cli: &Cli, output: PathBuf) -> Result<()> {
    let db = SignatureDatabase::new("export");
    let yaml = db.export_yaml();
    std::fs::write(&output, yaml)?;

    if !cli.quiet {
        println!(
            "{} to {}",
            "Exported signatures".green(),
            output.display().to_string().cyan()
        );
    }
    Ok(())
}

/// List loaded signatures
pub fn signatures_list(cli: &Cli) -> Result<()> {
    let db = SignatureDatabase::new("default");
    let info = db.info();

    match cli.format.as_str() {
        "json" => println!("{}", serde_json::to_string_pretty(&info)?),
        _ => {
            println!("\n{}", "Signature Database:".green().bold());
            println!("  Name: {}", info.name);
            println!("  Version: {}", info.version);
            println!("  Total: {} signatures", info.total_signatures);
        }
    }
    Ok(())
}

/// Report that a feature is declared but not implemented.
///
/// Returns an error so the process exits non-zero. Several commands used to
/// print a plausible success summary and exit 0 instead, which is worse than
/// not having them at all: a script could not tell that nothing had happened.
pub fn not_implemented(feature: &str, detail: &str) -> Result<()> {
    Err(format!("{feature} is not implemented in this build.\n{detail}").into())
}
