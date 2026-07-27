//! End-to-end tests for the commands that touch a device.
//!
//! These drive the real `openflash` binary against the in-process emulator, so
//! they exercise argument parsing, the transport, the protocol framing, the
//! device layer and the output formatting together. They are the tests that
//! would have caught what was wrong before: `write` and `erase` doing nothing,
//! `read` producing a file full of `0xFF`, and `verify` printing PASSED
//! unconditionally.
//!
//! The crate had `assert_cmd`, `predicates` and `tempfile` in dev-dependencies
//! but no `tests/` directory.

use std::fs;
use std::path::Path;
use std::process::Command;

use assert_cmd::prelude::*;
use predicates::prelude::*;
use tempfile::TempDir;

/// Emulated chip size used throughout: 64 KiB is 16 erase sectors, enough to
/// exercise sector and page boundaries without being slow.
const CHIP_SIZE: &str = "65536";

fn openflash() -> Command {
    Command::cargo_bin("openflash").expect("the binary is built by `cargo test`")
}

/// A session against one emulated chip whose contents persist across commands.
///
/// Each CLI invocation is a separate process, so without a backing file every
/// command would get its own freshly erased chip and a write could never be read
/// back — which is exactly the round trip these tests need to prove.
struct Session {
    _dir: TempDir,
    image: std::path::PathBuf,
}

impl Session {
    fn new() -> Self {
        let dir = TempDir::new().unwrap();
        let image = dir.path().join("chip.bin");
        Self { _dir: dir, image }
    }

    fn dir(&self) -> &Path {
        self.image
            .parent()
            .expect("the image lives in the temp dir")
    }

    /// A command against this session's chip, with confirmation disabled.
    fn command(&self) -> Command {
        let mut command = openflash();
        command
            .args(["--emulate", CHIP_SIZE, "--yes", "--emulate-image"])
            .arg(&self.image);
        command
    }
}

/// A one-shot command against an ephemeral emulated chip.
fn emulated() -> Command {
    let mut command = openflash();
    command.args(["--emulate", CHIP_SIZE, "--yes"]);
    command
}

/// Deterministic pseudo-random payload; a repeating pattern would hide
/// off-by-one errors in chunking.
fn payload(length: usize) -> Vec<u8> {
    let mut data = Vec::with_capacity(length);
    let mut state = 0x12345678u32;
    for _ in 0..length {
        state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        data.push((state >> 16) as u8);
    }
    data
}

fn write_file(dir: &Path, name: &str, data: &[u8]) -> std::path::PathBuf {
    let path = dir.join(name);
    fs::write(&path, data).expect("cannot write the fixture");
    path
}

#[test]
fn detect_reports_the_emulated_chip() {
    // 64 KiB is 2^16, so the capacity byte is 0x10 and no exact part matches;
    // the database falls back to a generic entry, and `detect` says so.
    emulated()
        .arg("detect")
        .assert()
        .success()
        .stdout(predicate::str::contains("Winbond"))
        .stdout(predicate::str::contains("EF 40 10"))
        .stdout(predicate::str::contains("64.00 KB"))
        .stdout(predicate::str::contains("not in the database"));
}

/// An id that is in the database must be named exactly, and without the
/// inferred-geometry warning.
#[test]
fn a_chip_in_the_database_is_named_exactly() {
    openflash()
        .args(["--emulate", "2097152", "detect"])
        .assert()
        .success()
        .stdout(predicate::str::contains("W25Q16JV"))
        .stdout(predicate::str::contains("EF 40 15"))
        .stdout(predicate::str::contains("2.00 MB"))
        .stdout(predicate::str::contains("not in the database").not());
}

/// Every emulated run must be labelled, so an emulated result can never be
/// mistaken for one taken off real hardware.
#[test]
fn emulated_runs_are_labelled_on_stderr() {
    emulated()
        .arg("detect")
        .assert()
        .success()
        .stderr(predicate::str::contains("EMULATED"))
        .stderr(predicate::str::contains("no hardware is involved"));
}

#[test]
fn json_output_is_parseable_and_not_preceded_by_the_banner() {
    let output = emulated()
        .args(["--format", "json", "detect"])
        .output()
        .expect("the command runs");
    assert!(output.status.success());

    let parsed: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("stdout must be pure JSON");
    assert_eq!(parsed["capacity"], 64 * 1024);
    assert_eq!(parsed["sector_size"], 4096);
    assert_eq!(parsed["exact_match"], false);
}

/// The headline case: what `write` puts on the chip is what `read` gets back.
#[test]
fn write_then_read_round_trips_byte_for_byte() {
    let session = Session::new();
    let original = payload(5000);
    let input = write_file(session.dir(), "firmware.bin", &original);
    let dump = session.dir().join("dump.bin");

    session
        .command()
        .args(["write", "-i"])
        .arg(&input)
        .assert()
        .success()
        .stdout(predicate::str::contains("Write complete"))
        .stdout(predicate::str::contains("read back and compared"));

    session
        .command()
        .args(["read", "-o"])
        .arg(&dump)
        .args(["--length", "5000"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Read complete"));

    let read_back = fs::read(&dump).unwrap();
    assert_eq!(
        read_back, original,
        "the dump must match what was written, byte for byte"
    );
}

/// `read` used to write `vec![0xFF; length]` and report a speed computed from a
/// hardcoded 5-second duration.
#[test]
fn read_does_not_produce_a_file_of_0xff_when_the_chip_holds_data() {
    let session = Session::new();
    let original = payload(4096);
    let input = write_file(session.dir(), "firmware.bin", &original);
    let dump = session.dir().join("dump.bin");

    session
        .command()
        .args(["write", "-i"])
        .arg(&input)
        .assert()
        .success();
    session
        .command()
        .args(["read", "-o"])
        .arg(&dump)
        .args(["--length", "4096"])
        .assert()
        .success();

    let read_back = fs::read(&dump).unwrap();
    assert_eq!(read_back.len(), 4096);
    assert!(
        !read_back.iter().all(|&b| b == 0xFF),
        "the dump is entirely 0xFF, which means nothing was actually read"
    );
    assert_eq!(read_back, original);
}

#[test]
fn a_dump_of_the_whole_chip_has_the_chip_s_length() {
    let dir = TempDir::new().unwrap();
    let dump = dir.path().join("full.bin");

    emulated()
        .args(["read", "-o"])
        .arg(&dump)
        .assert()
        .success();

    // The emulated chip's JEDEC id is derived from its array, so the capacity the
    // host computes from the id must equal what --emulate asked for.
    let metadata = fs::metadata(&dump).unwrap();
    assert_eq!(metadata.len(), CHIP_SIZE.parse::<u64>().unwrap());
}

#[test]
fn verify_passes_against_the_data_that_was_written() {
    let session = Session::new();
    let original = payload(2048);
    let input = write_file(session.dir(), "firmware.bin", &original);

    session
        .command()
        .args(["write", "-i"])
        .arg(&input)
        .assert()
        .success();

    session
        .command()
        .args(["verify", "--file"])
        .arg(&input)
        .assert()
        .success()
        .stdout(predicate::str::contains("Verification passed"));
}

/// `verify` held `let matches = true;` and printed PASSED whatever the chip
/// contained. A mismatch must fail and set a non-zero exit status.
#[test]
fn verify_fails_and_exits_non_zero_on_a_mismatch() {
    let session = Session::new();
    let written = payload(2048);
    let input = write_file(session.dir(), "firmware.bin", &written);

    let mut different = written.clone();
    different[1000] ^= 0xFF;
    let other = write_file(session.dir(), "different.bin", &different);

    session
        .command()
        .args(["write", "-i"])
        .arg(&input)
        .assert()
        .success();

    session
        .command()
        .args(["verify", "--file"])
        .arg(&other)
        .assert()
        .failure()
        .stderr(predicate::str::contains("verification failed"))
        .stderr(predicate::str::contains("0x3e8"));
}

/// `erase` slept 100 ms and printed "Erase complete!". It must now really
/// erase, and confirm by reading the range back.
#[test]
fn erase_really_blanks_the_range() {
    let session = Session::new();
    let original = payload(8192);
    let input = write_file(session.dir(), "firmware.bin", &original);
    let dump = session.dir().join("after-erase.bin");

    session
        .command()
        .args(["write", "-i"])
        .arg(&input)
        .assert()
        .success();

    session
        .command()
        .args(["erase", "--start", "0", "--length", "8192"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Confirmed blank"));

    session
        .command()
        .args(["read", "-o"])
        .arg(&dump)
        .args(["--length", "8192"])
        .assert()
        .success();

    let after = fs::read(&dump).unwrap();
    assert!(
        after.iter().all(|&b| b == 0xFF),
        "the erased range must read back as 0xFF"
    );
}

#[test]
fn an_unaligned_erase_is_refused() {
    emulated()
        .args(["erase", "--start", "100", "--length", "4096"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("does not fit"));
}

#[test]
fn writing_at_an_offset_leaves_the_surrounding_data_alone() {
    let session = Session::new();
    let background = payload(8192);
    let background_file = write_file(session.dir(), "background.bin", &background);
    let patch = vec![0x5Au8; 32];
    let patch_file = write_file(session.dir(), "patch.bin", &patch);
    let dump = session.dir().join("dump.bin");

    session
        .command()
        .args(["write", "-i"])
        .arg(&background_file)
        .assert()
        .success();
    session
        .command()
        .args(["write", "-i"])
        .arg(&patch_file)
        .args(["--start", "0x400"])
        .assert()
        .success();

    session
        .command()
        .args(["read", "-o"])
        .arg(&dump)
        .args(["--length", "8192"])
        .assert()
        .success();

    let mut expected = background.clone();
    expected[0x400..0x400 + patch.len()].copy_from_slice(&patch);
    assert_eq!(
        fs::read(&dump).unwrap(),
        expected,
        "patching 32 bytes must not disturb the rest of the sector"
    );
}

#[test]
fn writing_past_the_end_of_the_chip_is_refused() {
    let dir = TempDir::new().unwrap();
    let input = write_file(dir.path(), "big.bin", &payload(1024));

    emulated()
        .args(["write", "-i"])
        .arg(&input)
        // The emulated part is 64 KiB, so this range runs off the end.
        .args(["--start", "0xFFF0"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("does not fit"));
}

#[test]
fn info_reports_the_protocol_revision() {
    emulated()
        .arg("info")
        .assert()
        .success()
        .stdout(predicate::str::contains("Protocol:   v2"))
        .stdout(predicate::str::contains("spi-nor"));
}

#[test]
fn an_interface_the_device_lacks_is_refused() {
    emulated()
        .args(["interface", "emmc"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("does not implement"));
}

#[test]
fn an_unknown_interface_name_lists_the_valid_ones() {
    emulated()
        .args(["interface", "floppy"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("spi-nor"));
}

#[test]
fn pointing_at_two_devices_at_once_is_refused() {
    openflash()
        .args(["--device", "OF-1", "--tcp", "localhost:9999", "detect"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("mutually exclusive"));
}

#[test]
fn an_emulated_size_that_is_not_a_whole_sector_is_refused() {
    openflash()
        .args(["--emulate", "1000", "detect"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("sector"));
}

/// A command that cannot reach a device must fail, not fall back to something
/// that looks like success.
#[test]
fn an_unreachable_tcp_agent_fails() {
    openflash()
        // Port 1 on localhost: nothing listens there.
        .args(["--tcp", "127.0.0.1:1", "detect"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot reach"));
}

/// Subsystems that exist only as data types must say so and exit non-zero,
/// rather than printing a configuration summary that reads like a running
/// server.
#[test]
fn unimplemented_subsystems_fail_loudly() {
    for arguments in [vec!["server", "start"], vec!["clone"], vec!["job", "list"]] {
        let assertion = openflash()
            .args(&arguments)
            .assert()
            .failure()
            .stderr(predicate::str::contains("not implemented"));
        drop(assertion);
    }
}

#[test]
fn oob_is_refused_rather_than_silently_omitted() {
    let dir = TempDir::new().unwrap();
    let dump = dir.path().join("dump.bin");

    emulated()
        .args(["read", "-o"])
        .arg(&dump)
        .args(["--length", "256", "--oob"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("NAND"));
}

/// A read of more than one frame's payload must reassemble correctly; this is
/// where an off-by-one in chunking would show up.
#[test]
fn a_read_spanning_many_frames_is_reassembled_correctly() {
    let session = Session::new();
    // Deliberately not a multiple of the frame payload limit.
    let original = payload(20_001);
    let input = write_file(session.dir(), "firmware.bin", &original);
    let dump = session.dir().join("dump.bin");

    session
        .command()
        .args(["write", "-i"])
        .arg(&input)
        .assert()
        .success();
    session
        .command()
        .args(["read", "-o"])
        .arg(&dump)
        .args(["--length", "20001"])
        .assert()
        .success();

    assert_eq!(fs::read(&dump).unwrap(), original);
}

#[test]
fn compare_writes_the_report_that_dash_o_asks_for() {
    let dir = TempDir::new().unwrap();
    let first = payload(1024);
    let mut second = first.clone();
    second[500] ^= 0xFF;

    let a = write_file(dir.path(), "a.bin", &first);
    let b = write_file(dir.path(), "b.bin", &second);
    let report = dir.path().join("diff.md");

    openflash()
        .arg("compare")
        .arg(&a)
        .arg(&b)
        .arg("-o")
        .arg(&report)
        .assert()
        .success()
        .stdout(predicate::str::contains("First diff:  0x1f4"));

    let contents = fs::read_to_string(&report).expect("--output must produce a file");
    assert!(contents.contains("0x000001f4"), "{contents}");
}

#[test]
fn compare_reports_identical_files_as_identical() {
    let dir = TempDir::new().unwrap();
    let data = payload(512);
    let a = write_file(dir.path(), "a.bin", &data);
    let b = write_file(dir.path(), "b.bin", &data);

    openflash()
        .arg("compare")
        .arg(&a)
        .arg(&b)
        .assert()
        .success()
        .stdout(predicate::str::contains("identical"));
}

/// The persistence that makes the round-trip tests meaningful, asserted
/// directly: a write in one process must be visible to a read in the next.
#[test]
fn an_emulated_chip_image_survives_between_invocations() {
    let session = Session::new();
    let original = payload(1024);
    let input = write_file(session.dir(), "firmware.bin", &original);
    let dump = session.dir().join("dump.bin");

    session
        .command()
        .args(["write", "-i"])
        .arg(&input)
        .assert()
        .success();

    // A separate process, reading the same backing image.
    session
        .command()
        .args(["read", "-o"])
        .arg(&dump)
        .args(["--length", "1024"])
        .assert()
        .success();

    assert_eq!(fs::read(&dump).unwrap(), original);
}

/// Without a backing image each invocation gets its own blank chip. Asserting it
/// keeps the ephemeral mode honest: it must not appear to remember anything.
#[test]
fn an_ephemeral_emulated_chip_starts_blank_every_time() {
    let dir = TempDir::new().unwrap();
    let input = write_file(dir.path(), "firmware.bin", &payload(512));
    let dump = dir.path().join("dump.bin");

    emulated()
        .args(["write", "-i"])
        .arg(&input)
        .assert()
        .success();
    emulated()
        .args(["read", "-o"])
        .arg(&dump)
        .args(["--length", "512"])
        .assert()
        .success();

    let read_back = fs::read(&dump).unwrap();
    assert!(
        read_back.iter().all(|&b| b == 0xFF),
        "an ephemeral emulated chip must not carry data over from another process"
    );
}

/// `--quiet` suppresses progress and summaries, not failures: a script that
/// cannot flash a chip has to be told why, not just handed an exit status.
#[test]
fn quiet_mode_still_reports_the_reason_for_a_failure() {
    let session = Session::new();
    let expected = write_file(session.dir(), "expected.bin", &payload(256));

    // Nothing has been written, so the blank chip cannot match.
    session
        .command()
        .arg("--quiet")
        .args(["verify", "--file"])
        .arg(&expected)
        .assert()
        .failure()
        .stderr(predicate::str::contains("verification failed"))
        .stdout(predicate::str::is_empty());
}
