//! Guards against opcode drift between the host and the firmware.
//!
//! Every firmware used to declare its own copy of the command table, and the
//! copies disagreed. `Ping` was `0x01` on the RP2040, STM32 and Teensy boards
//! but `0x00` on the ESP32, Raspberry Pi and Orange Pi. A parallel-NAND page
//! read was `0x05`, `0x11` or `0x12` depending on which board you had. SPI NOR
//! sat at `0x60` on most, `0x70` on the ESP32. A host could therefore talk to
//! only a subset of the platforms the project claimed to support, and nothing in
//! the build noticed.
//!
//! Bare-metal firmware cannot be compiled in ordinary CI — it needs
//! cross-toolchains, linker scripts and per-board target configuration — so this
//! test reads the firmware sources instead. It parses every `= 0x..` opcode a
//! firmware declares and compares it with [`Command`], the shared table.
//!
//! Crates that have not been migrated yet are listed in [`NOT_YET_MIGRATED`],
//! together with what is wrong with each. The test checks the list is accurate
//! in both directions, so it fails when a crate is migrated but left in the list
//! and when a crate not in the list grows a table of its own.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use openflash_protocol::Command;

/// Firmware crates that still declare their own command table.
///
/// Each entry says which opcodes disagree with the shared table. Migrating a
/// crate means deleting its local table, depending on `openflash-protocol` with
/// `default-features = false`, and removing it from this list.
const NOT_YET_MIGRATED: &[(&str, &str)] = &[
    (
        "rp2040",
        "parallel NAND uses the deprecated v1 opcodes 0x03-0x07 instead of 0x10-0x16",
    ),
    (
        "stm32f1",
        "parallel NAND uses the deprecated v1 opcodes 0x03-0x07 instead of 0x10-0x16",
    ),
    (
        "stm32f4",
        "parallel NAND uses the deprecated v1 opcodes 0x03-0x07 instead of 0x10-0x16",
    ),
    (
        "esp32",
        "Ping is 0x00 not 0x01, NAND is 0x10-0x15 shifted by one, SPI NOR is at 0x70 not 0x60",
    ),
    (
        "teensy4",
        "GetVersion is 0x03 not 0x0A; the rest of the table already matches",
    ),
];

fn firmware_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../firmware")
}

/// Names of every firmware crate directory.
fn firmware_crates() -> Vec<String> {
    let mut crates: Vec<String> = fs::read_dir(firmware_dir())
        .expect("the firmware directory exists")
        .flatten()
        .filter(|entry| entry.path().is_dir())
        .filter(|entry| entry.path().join("Cargo.toml").is_file())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    crates.sort();
    crates
}

/// Opcodes a firmware's own `Command` enum declares, as `name -> byte`.
///
/// Only enum variants matter, so lines are taken from inside a block that starts
/// at `enum Command {` and ends at the matching close.
fn declared_opcodes(crate_name: &str) -> BTreeMap<String, u8> {
    let mut opcodes = BTreeMap::new();
    let source_dir = firmware_dir().join(crate_name).join("src");

    for path in rust_sources(&source_dir) {
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
        opcodes.extend(parse_command_enum(&source));
    }
    opcodes
}

fn rust_sources(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let mut stack = vec![dir.to_path_buf()];

    while let Some(current) = stack.pop() {
        let Ok(entries) = fs::read_dir(&current) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                files.push(path);
            }
        }
    }
    files.sort();
    files
}

/// Extract `Variant = 0xNN` pairs from any `enum Command { … }` block.
fn parse_command_enum(source: &str) -> BTreeMap<String, u8> {
    let mut opcodes = BTreeMap::new();
    let mut rest = source;

    while let Some(position) = rest.find("enum Command") {
        rest = &rest[position..];
        let Some(open) = rest.find('{') else { break };

        let mut depth = 0usize;
        let mut end = open;
        for (index, character) in rest[open..].char_indices() {
            match character {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        end = open + index;
                        break;
                    }
                }
                _ => {}
            }
        }

        for line in rest[open + 1..end].lines() {
            let line = line.trim();
            if line.starts_with("//") {
                continue;
            }
            let Some((name, value)) = line.split_once('=') else {
                continue;
            };
            let name = name.trim();
            if name.is_empty() || !name.chars().all(|c| c.is_alphanumeric() || c == '_') {
                continue;
            }
            // Strip a trailing comment before the comma: `0x12, // note` would
            // otherwise leave the comma attached and fail to parse.
            let value = value
                .split("//")
                .next()
                .unwrap_or("")
                .trim()
                .trim_end_matches(',')
                .trim();
            let Some(hex) = value
                .strip_prefix("0x")
                .or_else(|| value.strip_prefix("0X"))
            else {
                continue;
            };
            if let Ok(byte) = u8::from_str_radix(hex, 16) {
                opcodes.insert(name.to_string(), byte);
            }
        }

        rest = &rest[end..];
    }
    opcodes
}

/// The opcode the shared table assigns to a variant name, allowing for the
/// spelling differences firmware authors used for the same operation.
fn canonical_opcode(name: &str) -> Option<u8> {
    let normalised = match name {
        // Different names, same operation.
        "ReadId" => "NandReadId",
        "NandProgramPage" => "NandWritePage",
        "NandEraseBlock" => "NandErase",
        "SpiReadJedecId" => "SpiNorReadJedecId",
        "SpiRead" => "SpiNorRead",
        "SpiProgram" => "SpiNorPageProgram",
        "SpiErase" => "SpiNorSectorErase",
        "SpiReadStatus" => "SpiNorReadStatus1",
        "SpiNandReadPage" => "SpiNandPageRead",
        "SpiNandWritePage" => "SpiNandProgramLoad",
        "SpiNandEraseBlock" => "SpiNandBlockErase",
        "SpiNandReadStatus" => "SpiNandGetFeature",
        "EmmcEraseBlocks" => "EmmcErase",
        other => other,
    };

    Command::ALL
        .iter()
        .find(|command| format!("{command:?}") == normalised)
        .map(|command| *command as u8)
}

/// Opcodes in a firmware crate that disagree with the shared table.
fn disagreements(crate_name: &str) -> Vec<(String, u8, u8)> {
    declared_opcodes(crate_name)
        .into_iter()
        .filter_map(|(name, declared)| {
            let expected = canonical_opcode(&name)?;
            (declared != expected).then_some((name, declared, expected))
        })
        .collect()
}

#[test]
fn the_not_yet_migrated_list_names_real_firmware_crates() {
    let crates = firmware_crates();
    assert!(
        !crates.is_empty(),
        "found no firmware crates; the path is probably wrong"
    );

    for (name, _) in NOT_YET_MIGRATED {
        assert!(
            crates.contains(&name.to_string()),
            "{name} is listed as not migrated but there is no such firmware crate; \
             the list is stale"
        );
    }
}

/// The gate that matters: a crate not on the list must agree with the shared
/// table, so a newly written or newly migrated firmware cannot reintroduce its
/// own numbering.
#[test]
fn migrated_firmware_agrees_with_the_shared_table() {
    let excluded: Vec<&str> = NOT_YET_MIGRATED.iter().map(|(name, _)| *name).collect();

    for crate_name in firmware_crates() {
        if excluded.contains(&crate_name.as_str()) {
            continue;
        }

        let mismatches = disagreements(&crate_name);
        assert!(
            mismatches.is_empty(),
            "firmware/{crate_name} declares opcodes that disagree with \
             openflash_protocol::Command: {}. Use the shared crate instead of a local table.",
            mismatches
                .iter()
                .map(|(name, declared, expected)| format!(
                    "{name} is 0x{declared:02X}, should be 0x{expected:02X}"
                ))
                .collect::<Vec<_>>()
                .join("; ")
        );
    }
}

/// Keeps the list from rotting the other way: a crate that has actually been
/// fixed must be removed from it, so the list always reflects real work left.
#[test]
fn crates_on_the_not_yet_migrated_list_really_do_disagree() {
    for (crate_name, reason) in NOT_YET_MIGRATED {
        let mismatches = disagreements(crate_name);
        assert!(
            !mismatches.is_empty(),
            "firmware/{crate_name} now agrees with the shared table (listed reason: \
             \"{reason}\"). Remove it from NOT_YET_MIGRATED."
        );
    }
}

#[test]
fn the_enum_parser_reads_a_command_table() {
    let source = r#"
        #[repr(u8)]
        pub enum Command {
            // A comment line
            Ping = 0x01,
            NandReadPage = 0x12,  // trailing comment
            SpiNorRead = 0x62,
        }
    "#;

    let parsed = parse_command_enum(source);
    assert_eq!(parsed.get("Ping"), Some(&0x01));
    assert_eq!(parsed.get("NandReadPage"), Some(&0x12));
    assert_eq!(parsed.get("SpiNorRead"), Some(&0x62));
    assert_eq!(parsed.len(), 3);
}

#[test]
fn the_parser_ignores_constants_outside_a_command_enum() {
    // Chip-level command bytes are a different layer and must not be compared
    // against the device command table.
    let source = r#"
        pub mod commands {
            pub const READ_JEDEC_ID: u8 = 0x9F;
        }

        pub enum Status {
            Ok = 0x00,
        }
    "#;

    assert!(parse_command_enum(source).is_empty());
}

#[test]
fn name_aliases_resolve_to_the_shared_table() {
    assert_eq!(canonical_opcode("ReadId"), Some(Command::NandReadId as u8));
    assert_eq!(canonical_opcode("Ping"), Some(Command::Ping as u8));
    assert_eq!(
        canonical_opcode("SpiReadJedecId"),
        Some(Command::SpiNorReadJedecId as u8)
    );
    // A name that means nothing to the shared table is not compared at all,
    // rather than being reported as a mismatch: firmware may have extra,
    // board-specific commands.
    assert_eq!(canonical_opcode("WifiScan"), None);
}
