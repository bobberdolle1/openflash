//! Tests for `openflash chips`, which queries the chip databases.
//!
//! The command used to print five hardcoded parts under the heading "Supported
//! chips" — a list unrelated to the 207 parts the rest of the tool matches
//! against. Anyone checking whether their chip was supported got an answer
//! invented for the occasion. These tests pin the replacement to the real
//! databases.

use std::process::Command;

use assert_cmd::prelude::*;
use predicates::prelude::*;

fn openflash() -> Command {
    let mut command = Command::cargo_bin("openflash").expect("the binary is built by `cargo test`");
    // Suppress the banner so assertions see only the command's own output.
    command.arg("--quiet");
    command
}

#[test]
fn a_catalogued_spi_nor_part_is_found_by_its_jedec_id() {
    openflash()
        .args(["chips", "--id", "EF4018"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Winbond"))
        .stdout(predicate::str::contains("W25Q128JV"))
        .stdout(predicate::str::contains("spi-nor"))
        // 16 MiB, read out of the database rather than guessed.
        .stdout(predicate::str::contains("16777216"));
}

#[test]
fn an_id_can_be_written_with_separators_or_without() {
    for spelling in ["EF4018", "ef4018", "EF 40 18", "EF:40:18", "0xEF4018"] {
        openflash()
            .args(["chips", "--id", spelling])
            .assert()
            .success()
            .stdout(predicate::str::contains("W25Q128JV"));
    }
}

#[test]
fn a_parallel_nand_id_finds_the_parallel_nand_part() {
    openflash()
        .args(["chips", "--id", "EC F1 00 95 40"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Samsung"))
        .stdout(predicate::str::contains("K9F1G08U0B"));
}

/// Every database has a fallback that derives geometry from the id, and those
/// answer for almost anything. Searching all four with fallbacks on reported a
/// "Generic SPI NAND" for a parallel NAND id — noise dressed as a result — so a
/// search without `--interface` returns catalogue entries only.
#[test]
fn a_search_across_databases_does_not_report_derived_guesses() {
    openflash()
        .args(["chips", "--id", "EC F1 00 95 40"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Generic").not())
        .stdout(predicate::str::contains("derived").not());
}

/// Naming the interface says which catalogue is the right one to ask, so the
/// derived answer becomes useful — but it still has to be labelled as derived.
#[test]
fn naming_an_interface_allows_a_derived_answer_and_labels_it() {
    openflash()
        .args(["chips", "--id", "EC F1 00 95 40", "-i", "spi-nand"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Generic SPI NAND"))
        .stdout(predicate::str::contains(
            "derived from the id, not a catalogue entry",
        ));
}

#[test]
fn json_output_says_whether_a_hit_was_exact() {
    let output = openflash()
        .args(["chips", "--id", "EF4018", "--format", "json"])
        .output()
        .unwrap();

    let parsed: serde_json::Value = serde_json::from_slice(&output.stdout)
        .unwrap_or_else(|e| panic!("not JSON: {e}\n{}", String::from_utf8_lossy(&output.stdout)));

    let entries = parsed.as_array().expect("a list of hits");
    assert_eq!(entries.len(), 1, "{parsed:#}");
    assert_eq!(entries[0]["exact"], serde_json::json!(true));
    assert_eq!(entries[0]["model"], serde_json::json!("W25Q128JV"));
    assert_eq!(entries[0]["interface"], serde_json::json!("spi-nor"));
}

#[test]
fn an_uncatalogued_id_says_so_rather_than_inventing_a_part() {
    openflash()
        .args(["chips", "--id", "AABBCC"])
        .assert()
        .success()
        .stdout(predicate::str::contains("matches no catalogued chip"));
}

#[test]
fn an_unparseable_id_is_rejected() {
    // Odd number of digits.
    openflash()
        .args(["chips", "--id", "EF401"])
        .assert()
        .failure();
    // Not hex.
    openflash()
        .args(["chips", "--id", "ZZZZ"])
        .assert()
        .failure();
}

#[test]
fn an_unknown_interface_is_reported_as_a_typo() {
    openflash()
        .args(["chips", "--id", "EF4018", "-i", "spinor"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("unknown interface"));
}

/// Enumeration is not available: the databases are `match` arms keyed by id, so
/// they can be queried and not iterated. The command has to say that rather than
/// print a stand-in list, which is what it used to do.
#[test]
fn listing_refuses_instead_of_printing_a_stand_in_list() {
    openflash()
        .args(["chips"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not implemented"))
        .stderr(predicate::str::contains("--id"));
}

#[test]
fn filtering_refuses_too_and_names_filtering_as_the_missing_part() {
    openflash()
        .args(["chips", "-m", "Winbond"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("filtering the chip database"));
}

/// A typo in `--interface` must be reported even on the path that cannot use it
/// yet, rather than being swallowed by the "not implemented" message.
#[test]
fn a_bad_interface_is_caught_even_when_listing_is_unavailable() {
    openflash()
        .args(["chips", "-i", "nonsense"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("unknown interface"));
}
