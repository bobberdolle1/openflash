//! Tests for `openflash analyze`, which examines a dump already on disk.
//!
//! The command used to read the file, ignore it, and print a fixed result: a
//! SquashFS at 0x10000 and a U-Boot image at 0, with hardcoded confidences, for
//! any input at all. A user comparing two dumps got identical findings. These
//! tests pin the output to the input.

use std::io::Write;
use std::process::Command;

use assert_cmd::prelude::*;
use predicates::prelude::*;
use tempfile::NamedTempFile;

fn openflash() -> Command {
    let mut command = Command::cargo_bin("openflash").expect("the binary is built by `cargo test`");
    command.arg("--quiet");
    command
}

fn file_of(bytes: Vec<u8>) -> NamedTempFile {
    let mut file = NamedTempFile::new().unwrap();
    file.write_all(&bytes).unwrap();
    file.flush().unwrap();
    file
}

/// Deterministic high-entropy bytes, without pulling in a RNG dependency.
fn pseudo_random(len: usize) -> Vec<u8> {
    let mut state = 0x2545_F491_4F6C_DD1Du64;
    (0..len)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            (state >> 24) as u8
        })
        .collect()
}

fn analyse_json(bytes: Vec<u8>) -> serde_json::Value {
    let file = file_of(bytes);
    let output = openflash()
        .args(["analyze", file.path().to_str().unwrap(), "--format", "json"])
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    serde_json::from_slice(&output.stdout)
        .unwrap_or_else(|e| panic!("not JSON: {e}\n{}", String::from_utf8_lossy(&output.stdout)))
}

/// The test that matters: different inputs must produce different findings.
#[test]
fn the_result_depends_on_the_input() {
    let zeros = analyse_json(vec![0x00; 0x8000]);
    let erased = analyse_json(vec![0xFF; 0x8000]);
    let noise = analyse_json(pseudo_random(0x8000));

    let kind = |value: &serde_json::Value| {
        value["patterns"][0]["pattern_type"]
            .as_str()
            .unwrap_or("none")
            .to_string()
    };

    assert_eq!(kind(&zeros), "Zeroed");
    assert_eq!(kind(&erased), "Empty");
    assert_eq!(kind(&noise), "Encrypted");

    // And the summary is not one fixed sentence for every dump.
    assert_ne!(zeros["summary"], noise["summary"]);
}

#[test]
fn high_entropy_data_reads_as_probably_encrypted() {
    let result = analyse_json(pseudo_random(0x8000));
    let probability = result["encryption_probability"].as_f64().unwrap();
    assert!(
        probability > 0.9,
        "expected high encryption probability, got {probability}"
    );
}

#[test]
fn erased_flash_is_not_reported_as_encrypted() {
    let result = analyse_json(vec![0xFF; 0x8000]);
    let probability = result["encryption_probability"].as_f64().unwrap();
    assert!(
        probability < 0.1,
        "an erased chip is not encrypted, got {probability}"
    );
}

/// A real signature at a real offset, found rather than assumed.
#[test]
fn a_squashfs_superblock_is_found_where_it_actually_is() {
    let mut data = pseudo_random(0x8000);
    data[0..4].copy_from_slice(b"hsqs");

    let result = analyse_json(data);
    let filesystems = result["filesystems"].as_array().unwrap();

    assert_eq!(filesystems.len(), 1, "{result:#}");
    assert_eq!(filesystems[0]["fs_type"], serde_json::json!("SquashFS"));
    assert_eq!(filesystems[0]["offset"], serde_json::json!(0));
}

/// The old output claimed a SquashFS in every dump. A dump without one must
/// report none.
#[test]
fn a_dump_with_no_filesystem_reports_none() {
    let result = analyse_json(vec![0xFF; 0x8000]);
    assert!(
        result["filesystems"].as_array().unwrap().is_empty(),
        "{result:#}"
    );

    // And the text output says so rather than staying silent.
    let file = file_of(vec![0xFF; 0x8000]);
    openflash()
        .args(["analyze", file.path().to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("Filesystems:"))
        .stdout(predicate::str::contains("none found"));
}

#[test]
fn an_empty_file_is_rejected_rather_than_analysed() {
    let file = file_of(Vec::new());
    openflash()
        .args(["analyze", file.path().to_str().unwrap()])
        .assert()
        .failure()
        .stderr(predicate::str::contains("empty"));
}

#[test]
fn a_missing_file_is_reported() {
    openflash()
        .args(["analyze", "/nonexistent/dump.bin"])
        .assert()
        .failure();
}

/// `--output` writes the report the caller asked for, built from the same
/// analysis rather than from a template.
#[test]
fn the_written_report_reflects_the_analysis() {
    let mut data = pseudo_random(0x8000);
    data[0..4].copy_from_slice(b"hsqs");
    let input = file_of(data);
    let report = NamedTempFile::new().unwrap();

    openflash()
        .args([
            "analyze",
            input.path().to_str().unwrap(),
            "--output",
            report.path().to_str().unwrap(),
            "--report-format",
            "json",
        ])
        .assert()
        .success();

    let written: serde_json::Value =
        serde_json::from_slice(&std::fs::read(report.path()).unwrap()).unwrap();
    assert_eq!(
        written["filesystems"][0]["fs_type"],
        serde_json::json!("SquashFS")
    );
}

// ---------------------------------------------------------------------------
// rootfs
// ---------------------------------------------------------------------------

/// A SquashFS superblock with a known inode count and image size.
fn squashfs_image(inodes: u32, bytes_used: u64, total: usize) -> Vec<u8> {
    let mut image = vec![0u8; total];
    image[0..4].copy_from_slice(b"hsqs");
    image[4..8].copy_from_slice(&inodes.to_le_bytes());
    image[40..48].copy_from_slice(&bytes_used.to_le_bytes());
    image
}

/// The one that matters. `rootfs` used to return a fixed listing —
/// `/bin/busybox`, `/etc/passwd`, `/etc/shadow` — for every image, and write it
/// to `_files.txt`. For a tool used in recovery and security work, inventing
/// paths that look like real findings is the worst possible failure.
#[test]
fn rootfs_does_not_invent_a_file_listing() {
    let input = file_of(squashfs_image(1234, 4096, 4096));
    let out = tempfile::tempdir().unwrap();

    let output = openflash()
        .args([
            "rootfs",
            input.path().to_str().unwrap(),
            "-o",
            out.path().to_str().unwrap(),
            "--contents",
        ])
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");

    let stdout = String::from_utf8_lossy(&output.stdout);
    for invented in ["/bin/busybox", "/etc/passwd", "/etc/shadow", "/etc/init.d"] {
        assert!(
            !stdout.contains(invented),
            "invented path {invented} in output:\n{stdout}"
        );
    }

    let listing =
        std::fs::read_to_string(out.path().join("fs0_SquashFS").join("_files.txt")).unwrap();
    for invented in ["/bin/busybox", "/etc/passwd", "/etc/shadow"] {
        assert!(
            !listing.contains(invented),
            "invented path {invented} written to _files.txt:\n{listing}"
        );
    }
    assert!(
        listing.contains("No file listing"),
        "the listing file must explain itself:\n{listing}"
    );
}

/// What it does report has to come out of the image.
#[test]
fn rootfs_reports_what_the_superblock_actually_says() {
    let input = file_of(squashfs_image(4242, 8192, 8192));
    let out = tempfile::tempdir().unwrap();

    openflash()
        .args([
            "rootfs",
            input.path().to_str().unwrap(),
            "-o",
            out.path().to_str().unwrap(),
        ])
        .assert()
        .success()
        // The inode count is read from the bytes, not assumed.
        .stdout(predicate::str::contains("4242 inodes"))
        .stdout(predicate::str::contains("Contents not enumerated"));
}

/// Carving the image out is the useful thing it can honestly do, so another
/// tool can open what this one cannot parse.
#[test]
fn rootfs_carves_the_filesystem_image_out() {
    let image = squashfs_image(10, 2048, 2048);
    let input = file_of(image.clone());
    let out = tempfile::tempdir().unwrap();

    openflash()
        .args([
            "rootfs",
            input.path().to_str().unwrap(),
            "-o",
            out.path().to_str().unwrap(),
        ])
        .assert()
        .success();

    let carved = std::fs::read(out.path().join("fs0_SquashFS").join("image.squashfs")).unwrap();
    assert_eq!(&carved[..4], b"hsqs");
    assert_eq!(carved.len(), 2048);
}
