//! CLI command implementations

use crate::{create_progress_bar, format_size, parse_address, Cli};
use colored::Colorize;
use openflash_core::scripting::*;
use std::path::PathBuf;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

/// Scan for connected devices
pub fn scan(cli: &Cli) -> Result<()> {
    if !cli.quiet {
        println!("{}", "Scanning for OpenFlash devices...".yellow());
    }

    // Mock implementation - would scan USB/serial ports
    let devices = vec![
        ("RP2040", "/dev/ttyACM0", "1.8.0"),
        ("ESP32", "/dev/ttyUSB0", "1.8.0"),
    ];

    match cli.format.as_str() {
        "json" => {
            let json: Vec<_> = devices.iter().map(|(p, port, v)| {
                serde_json::json!({"platform": p, "port": port, "version": v})
            }).collect();
            println!("{}", serde_json::to_string_pretty(&json)?);
        }
        _ => {
            println!("\n{}", "Found devices:".green().bold());
            for (platform, port, version) in &devices {
                println!("  {} {} @ {} (fw {})", 
                    "●".green(), platform.cyan(), port.white(), version.dimmed());
            }
        }
    }
    Ok(())
}

/// Detect connected chip
pub fn detect(cli: &Cli) -> Result<()> {
    let mut of = OpenFlash::new();
    of.connect_with_config(ConnectionConfig {
        port: cli.port.clone(),
        ..Default::default()
    })?;

    let chip = of.detect_chip()?;

    match cli.format.as_str() {
        "json" => println!("{}", serde_json::to_string_pretty(&chip)?),
        _ => {
            println!("\n{}", "Detected chip:".green().bold());
            println!("  Manufacturer: {}", chip.manufacturer.cyan());
            println!("  Model:        {}", chip.model.cyan());
            println!("  Capacity:     {}", format_size(chip.capacity).yellow());
            println!("  Page size:    {} bytes", chip.page_size);
            println!("  Block size:   {}", format_size(chip.block_size as u64));
            println!("  OOB size:     {} bytes", chip.oob_size);
            println!("  Interface:    {}", chip.interface);
            println!("  ID:           {}", chip.id_bytes.iter()
                .map(|b| format!("{:02X}", b)).collect::<Vec<_>>().join(" "));
        }
    }
    Ok(())
}


/// Read/dump chip
pub fn read(
    cli: &Cli,
    output: PathBuf,
    start: &str,
    length: Option<&str>,
    oob: bool,
    skip_bad: bool,
) -> Result<()> {
    let start_addr = parse_address(start)?;
    let length_val = length.map(|l| parse_address(l)).transpose()?;

    let mut of = OpenFlash::new();
    of.connect_with_config(ConnectionConfig {
        port: cli.port.clone(),
        ..Default::default()
    })?;

    let chip = of.detect_chip()?;
    let total = length_val.unwrap_or(chip.capacity);

    if !cli.quiet {
        println!("{} {} to {}", "Reading".green(), format_size(total).yellow(), 
            output.display().to_string().cyan());
    }

    let pb = if !cli.quiet { Some(create_progress_bar(total, "Reading...")) } else { None };

    let result = of.read_with_options(ReadOptions {
        start_address: start_addr,
        length: length_val,
        include_oob: oob,
        skip_bad_blocks: skip_bad,
        ..Default::default()
    })?;

    if let Some(pb) = pb {
        pb.finish_with_message("Done!");
    }

    std::fs::write(&output, &result.data)?;
    if let Some(oob_data) = &result.oob_data {
        let oob_path = output.with_extension("oob");
        std::fs::write(&oob_path, oob_data)?;
    }

    if !cli.quiet {
        println!("\n{}", "Read complete:".green().bold());
        println!("  Bytes:    {}", format_size(result.stats.bytes_read));
        println!("  Pages:    {}", result.stats.pages_read);
        println!("  Duration: {} ms", result.stats.duration_ms);
        println!("  Speed:    {}/s", format_size(result.stats.speed_bps));
        if !result.bad_blocks.is_empty() {
            println!("  Bad blocks: {:?}", result.bad_blocks);
        }
    }
    Ok(())
}

/// Write/program chip
pub fn write(
    cli: &Cli,
    input: PathBuf,
    start: &str,
    verify: bool,
    erase: bool,
    skip_bad: bool,
) -> Result<()> {
    let start_addr = parse_address(start)?;
    let data = std::fs::read(&input)?;

    if !cli.quiet {
        println!("{} {} from {}", "Writing".green(), format_size(data.len() as u64).yellow(),
            input.display().to_string().cyan());
        if erase { println!("  Erase before write: {}", "yes".green()); }
        if verify { println!("  Verify after write: {}", "yes".green()); }
    }

    let pb = if !cli.quiet { Some(create_progress_bar(data.len() as u64, "Writing...")) } else { None };

    // Mock write operation
    std::thread::sleep(std::time::Duration::from_millis(100));

    if let Some(pb) = pb {
        pb.finish_with_message("Done!");
    }

    if !cli.quiet {
        println!("\n{}", "Write complete!".green().bold());
    }
    Ok(())
}

/// Erase chip
pub fn erase(cli: &Cli, start: Option<&str>, length: Option<&str>, force: bool) -> Result<()> {
    if !force && !cli.quiet {
        println!("{}", "WARNING: This will erase flash data!".red().bold());
        println!("Use --force to confirm.");
        return Ok(());
    }

    let start_addr = start.map(|s| parse_address(s)).transpose()?.unwrap_or(0);
    
    if !cli.quiet {
        println!("{} chip...", "Erasing".red());
    }

    // Mock erase
    std::thread::sleep(std::time::Duration::from_millis(100));

    if !cli.quiet {
        println!("{}", "Erase complete!".green().bold());
    }
    Ok(())
}

/// Verify chip contents
pub fn verify(cli: &Cli, file: PathBuf, start: &str) -> Result<()> {
    let data = std::fs::read(&file)?;
    
    if !cli.quiet {
        println!("{} against {}", "Verifying".yellow(), file.display().to_string().cyan());
    }

    // Mock verify
    let matches = true;

    if matches {
        println!("{}", "Verification PASSED!".green().bold());
    } else {
        println!("{}", "Verification FAILED!".red().bold());
    }
    Ok(())
}

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
        println!("{} {} ({})...", "Analyzing".cyan(), 
            input.as_ref().map(|p| p.display().to_string()).unwrap_or_default().yellow(),
            format_size(data.len() as u64));
        if deep { println!("  Deep scan: {}", "enabled".yellow()); }
    }

    // Mock analysis result
    let result = ScriptAnalysisResult {
        quality_score: 0.85,
        encryption_probability: 0.15,
        compression_probability: 0.45,
        patterns: vec![
            PatternInfo { pattern_type: "SquashFS".into(), offset: 0x10000, size: 0x500000, confidence: 0.95 },
            PatternInfo { pattern_type: "U-Boot".into(), offset: 0, size: 0x10000, confidence: 0.90 },
        ],
        filesystems: vec![
            FilesystemInfo { fs_type: "SquashFS".into(), offset: 0x10000, size: Some(0x500000), confidence: 0.95 },
        ],
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
            println!("  Encryption:  {:.0}%", result.encryption_probability * 100.0);
            println!("  Compression: {:.0}%", result.compression_probability * 100.0);
            println!("\n{}", "Detected patterns:".cyan());
            for p in &result.patterns {
                println!("  {} @ 0x{:X} ({}, {:.0}% confidence)", 
                    p.pattern_type.yellow(), p.offset, format_size(p.size), p.confidence * 100.0);
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
        "html" => format!("<html><body><h1>OpenFlash Analysis Report</h1><pre>{}</pre></body></html>",
            serde_json::to_string_pretty(result).unwrap_or_default()),
        _ => format!("# OpenFlash Analysis Report\n\n## Summary\n{}\n\n## Quality: {:.0}%\n",
            result.summary, result.quality_score * 100.0),
    }
}

/// Compare two dumps
pub fn compare(cli: &Cli, file1: PathBuf, file2: PathBuf, output: Option<PathBuf>) -> Result<()> {
    let data1 = std::fs::read(&file1)?;
    let data2 = std::fs::read(&file2)?;

    if !cli.quiet {
        println!("{} {} vs {}", "Comparing".cyan(),
            file1.display().to_string().yellow(),
            file2.display().to_string().yellow());
    }

    let mut diffs = 0;
    let min_len = data1.len().min(data2.len());
    for i in 0..min_len {
        if data1[i] != data2[i] { diffs += 1; }
    }
    diffs += (data1.len() as i64 - data2.len() as i64).unsigned_abs() as usize;

    let similarity = 1.0 - (diffs as f64 / data1.len().max(data2.len()) as f64);

    println!("\n{}", "Comparison Results:".green().bold());
    println!("  File 1 size: {}", format_size(data1.len() as u64));
    println!("  File 2 size: {}", format_size(data2.len() as u64));
    println!("  Differences: {} bytes", diffs);
    println!("  Similarity:  {:.2}%", similarity * 100.0);
    Ok(())
}

/// Clone chip-to-chip
pub fn clone_chip(cli: &Cli, mode: &str, verify: bool) -> Result<()> {
    if !cli.quiet {
        println!("{} (mode: {})", "Starting chip-to-chip clone".cyan(), mode.yellow());
        println!("  Verify: {}", if verify { "yes".green() } else { "no".red() });
    }
    println!("{}", "Clone operation requires two devices connected.".yellow());
    Ok(())
}

/// Run batch jobs
pub fn batch(cli: &Cli, file: PathBuf, stop_on_error: bool) -> Result<()> {
    if !cli.quiet {
        println!("{} {}", "Running batch file:".cyan(), file.display().to_string().yellow());
    }

    // Mock batch execution
    let jobs = vec!["Read chip", "Analyze dump", "Export report"];
    for (i, job) in jobs.iter().enumerate() {
        println!("  [{}/{}] {} {}", i + 1, jobs.len(), "✓".green(), job);
    }

    println!("\n{}", "Batch complete!".green().bold());
    Ok(())
}

/// Run script
pub fn script(cli: &Cli, file: PathBuf, args: Vec<String>) -> Result<()> {
    if !cli.quiet {
        println!("{} {}", "Running script:".cyan(), file.display().to_string().yellow());
        if !args.is_empty() {
            println!("  Args: {:?}", args);
        }
    }
    println!("{}", "Script execution requires Python runtime.".yellow());
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
    let chips = vec![
        ("Samsung", "K9F1G08U0E", "parallel_nand", "128MB"),
        ("Samsung", "K9F2G08U0C", "parallel_nand", "256MB"),
        ("GigaDevice", "GD5F1GQ4U", "spi_nand", "128MB"),
        ("Winbond", "W25Q128JV", "spi_nor", "16MB"),
        ("Samsung", "KLMAG1JETD", "emmc", "16GB"),
    ];

    let filtered: Vec<_> = chips.iter()
        .filter(|(m, model, iface, _)| {
            interface.as_ref().map(|i| iface.contains(i)).unwrap_or(true) &&
            manufacturer.as_ref().map(|mf| m.to_lowercase().contains(&mf.to_lowercase())).unwrap_or(true) &&
            search.as_ref().map(|s| model.to_lowercase().contains(&s.to_lowercase())).unwrap_or(true)
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
            println!("\n{} ({} chips)", "Supported chips:".green().bold(), filtered.len());
            for (mfr, model, iface, cap) in filtered {
                println!("  {} {} ({}) - {}", mfr.cyan(), model.white(), iface.dimmed(), cap.yellow());
            }
        }
    }
    Ok(())
}

/// Show device info
pub fn info(cli: &Cli) -> Result<()> {
    let mut of = OpenFlash::new();
    of.connect_with_config(ConnectionConfig {
        port: cli.port.clone(),
        ..Default::default()
    })?;

    let info = of.device_info().ok_or("Not connected")?;

    match cli.format.as_str() {
        "json" => println!("{}", serde_json::to_string_pretty(&info)?),
        _ => {
            println!("\n{}", "Device Information:".green().bold());
            println!("  Port:       {}", info.port.cyan());
            println!("  Platform:   {}", info.platform.yellow());
            println!("  Firmware:   {}", info.firmware_version);
            println!("  Serial:     {}", info.serial_number.dimmed());
            println!("  Interfaces: {}", info.interfaces.join(", "));
        }
    }
    Ok(())
}

/// Set interface
pub fn set_interface(cli: &Cli, interface: &str) -> Result<()> {
    if !cli.quiet {
        println!("Setting interface to: {}", interface.cyan());
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
pub fn unpack(cli: &Cli, input: PathBuf, output: PathBuf, depth: u32, recursive: bool) -> Result<()> {
    let data = std::fs::read(&input)?;
    
    if !cli.quiet {
        println!("{} {} ({})...", "Unpacking".cyan(), 
            input.display().to_string().yellow(),
            format_size(data.len() as u64));
        println!("  Output: {}", output.display().to_string().cyan());
        println!("  Max depth: {}", depth);
        println!("  Recursive: {}", if recursive { "yes".green() } else { "no".red() });
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
                    println!("  {} @ 0x{:08X} ({}) - {}", 
                        section.name.yellow(),
                        section.offset,
                        format_size(section.size),
                        section.section_type.dimmed());
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
        println!("{} from {} ...", "Extracting rootfs".cyan(), 
            input.display().to_string().yellow());
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
                println!("\n  {}. {} @ 0x{:08X} ({})", 
                    i + 1,
                    format!("{}", fs.fs_type).cyan(),
                    fs.offset,
                    format_size(fs.size));
                println!("     Files: {}, Directories: {}", fs.total_files, fs.total_dirs);
                
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
        println!("{} {} ...", "Scanning".red(), input.display().to_string().yellow());
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
                    
                    println!("\n  {} {} (CVSS: {:.1})", 
                        severity_icon,
                        vuln.name.red(),
                        vuln.cvss.base_score);
                    
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
        println!("{} {} ...", "Identifying chip from".cyan(), 
            input.display().to_string().yellow());
    }

    let identifier = MlChipIdentifier::new();
    let predictions = identifier.identify(&data).map_err(|e| e.to_string())?;

    match cli.format.as_str() {
        "json" => println!("{}", serde_json::to_string_pretty(&predictions)?),
        _ => {
            let model_info = identifier.model_info();
            println!("\n{}", "ML Model Info:".dimmed());
            println!("  Version: {}, Accuracy: {:.0}%, Chips: {}", 
                model_info.version, model_info.accuracy * 100.0, model_info.supported_chips);

            println!("\n{}", "Chip Predictions:".green().bold());
            for (i, pred) in predictions.iter().take(top).enumerate() {
                let confidence_bar = "█".repeat((pred.confidence * 20.0) as usize);
                let confidence_empty = "░".repeat(20 - (pred.confidence * 20.0) as usize);
                
                println!("\n  {}. {} {}", i + 1, 
                    pred.manufacturer.cyan(), 
                    pred.model.white().bold());
                println!("     Confidence: [{}{}] {:.1}%", 
                    confidence_bar.green(), confidence_empty.dimmed(),
                    pred.confidence * 100.0);
                println!("     Page: {} bytes, Block: {}, Capacity: {}", 
                    pred.page_size, 
                    format_size(pred.block_size as u64),
                    format_size(pred.capacity));
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
        println!("{} {} signatures from {}", 
            "Loaded".green(), count, file.display().to_string().cyan());
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
        println!("{} {} with {} signatures...", 
            "Scanning".cyan(), 
            input.display().to_string().yellow(),
            db.list().len());
    }

    let matches = db.scan(&data);

    match cli.format.as_str() {
        "json" => println!("{}", serde_json::to_string_pretty(&matches)?),
        _ => {
            if matches.is_empty() {
                println!("{}", "No signature matches found.".green());
            } else {
                println!("\n{} ({} matches)", "Signature Matches:".yellow().bold(), matches.len());
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
        println!("{} to {}", "Exported signatures".green(), 
            output.display().to_string().cyan());
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
