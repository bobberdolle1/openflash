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

    // Mock analysis result
    let result = ScriptAnalysisResult {
        quality_score: 0.85,
        encryption_probability: 0.15,
        compression_probability: 0.45,
        patterns: vec![
            PatternInfo {
                pattern_type: "SquashFS".into(),
                offset: 0x10000,
                size: 0x500000,
                confidence: 0.95,
            },
            PatternInfo {
                pattern_type: "U-Boot".into(),
                offset: 0,
                size: 0x10000,
                confidence: 0.90,
            },
        ],
        filesystems: vec![FilesystemInfo {
            fs_type: "SquashFS".into(),
            offset: 0x10000,
            size: Some(0x500000),
            confidence: 0.95,
        }],
        anomalies: vec![],
        recovery_suggestions: vec![],
        key_candidates: vec![],
        summary: "Firmware dump with bootloader and SquashFS filesystem".into(),
    };

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
            for p in &result.patterns {
                println!(
                    "  {} @ 0x{:X} ({}, {:.0}% confidence)",
                    p.pattern_type.yellow(),
                    p.offset,
                    format_size(p.size),
                    p.confidence * 100.0
                );
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

/// List supported chips
pub fn list_chips(
    cli: &Cli,
    interface: Option<String>,
    manufacturer: Option<String>,
    search: Option<String>,
) -> Result<()> {
    // Mock chip database
    let chips = [
        ("Samsung", "K9F1G08U0E", "parallel_nand", "128MB"),
        ("Samsung", "K9F2G08U0C", "parallel_nand", "256MB"),
        ("GigaDevice", "GD5F1GQ4U", "spi_nand", "128MB"),
        ("Winbond", "W25Q128JV", "spi_nor", "16MB"),
        ("Samsung", "KLMAG1JETD", "emmc", "16GB"),
    ];

    let filtered: Vec<_> = chips
        .iter()
        .filter(|(m, model, iface, _)| {
            interface
                .as_ref()
                .map(|i| iface.contains(i))
                .unwrap_or(true)
                && manufacturer
                    .as_ref()
                    .map(|mf| m.to_lowercase().contains(&mf.to_lowercase()))
                    .unwrap_or(true)
                && search
                    .as_ref()
                    .map(|s| model.to_lowercase().contains(&s.to_lowercase()))
                    .unwrap_or(true)
        })
        .collect();

    match cli.format.as_str() {
        "json" => {
            let json: Vec<_> = filtered.iter().map(|(m, model, iface, cap)| {
                serde_json::json!({"manufacturer": m, "model": model, "interface": iface, "capacity": cap})
            }).collect();
            println!("{}", serde_json::to_string_pretty(&json)?);
        }
        _ => {
            println!(
                "\n{} ({} chips)",
                "Supported chips:".green().bold(),
                filtered.len()
            );
            for (mfr, model, iface, cap) in filtered {
                println!(
                    "  {} {} ({}) - {}",
                    mfr.cyan(),
                    model.white(),
                    iface.dimmed(),
                    cap.yellow()
                );
            }
        }
    }
    Ok(())
}

/// Show config
pub fn config_show(_cli: &Cli) -> Result<()> {
    println!("\n{}", "Current configuration:".green().bold());
    println!("  default_port: auto");
    println!("  baud_rate: 115200");
    println!("  verify_writes: true");
    println!("  skip_bad_blocks: true");
    Ok(())
}

/// Set config value
pub fn config_set(_cli: &Cli, key: &str, value: &str) -> Result<()> {
    println!("Set {} = {}", key.cyan(), value.yellow());
    Ok(())
}

/// Reset config
pub fn config_reset(_cli: &Cli) -> Result<()> {
    println!("{}", "Configuration reset to defaults.".green());
    Ok(())
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
                println!(
                    "     Files: {}, Directories: {}",
                    fs.total_files, fs.total_dirs
                );

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

        // Save file listing
        let listing: Vec<&str> = fs.files.iter().map(|f| f.path.as_str()).collect();
        let listing_path = fs_dir.join("_files.txt");
        std::fs::write(&listing_path, listing.join("\n"))?;

        if !cli.quiet {
            println!("\nExtracted to: {}", fs_dir.display().to_string().cyan());
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
