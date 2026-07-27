//! OpenFlash agent for Banana Pi boards.
//!
//! A Linux daemon that drives the flash chip through the board's SPI controller
//! and serves hosts over a Unix socket or TCP. The protocol handling and the SPI
//! NOR sequencing come from `openflash-sbc-agent`, shared with the Raspberry Pi
//! and Orange Pi agents; this file is the board-specific part.
//!
//! Supported boards: Banana Pi M2 Zero (Allwinner H3), M4 Berry (Allwinner
//! H618), BPI-F3 (SpacemiT K1, RISC-V).
//!
//! # What this replaces
//!
//! The previous version declared its own opcode table, putting `GetVersion` at
//! `0x03` where the shared table has `0x0A`, and replied without framing — no
//! length, no checksum — so a dropped byte desynchronised the link silently. Its
//! SPI transfer issued a `write` syscall followed by a separate `read` on
//! `/dev/spidev*`; the kernel deasserts chip select between the two, so a flash
//! chip discards the command before the data arrives. Its own comment noted "for
//! full duplex, we need ioctl with spi_ioc_transfer", which is what the shared
//! bus now does.

use log::{error, info, warn};

use openflash_sbc_agent::{listen_tcp, listen_unix, Agent, Platform, SpiNor};

mod gpio;
mod spi;

/// Agent version, reported in the `GetVersion` reply.
const VERSION: (u8, u8, u8) = (3, 1, 0);

/// Default Unix socket path.
const SOCKET_PATH: &str = "/tmp/openflash.sock";

/// A recognised Banana Pi board.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoardInfo {
    /// Marketing name.
    pub name: &'static str,
    /// SoC the board is built around.
    pub soc: &'static str,
    /// spidev node the flash chip is wired to.
    pub spi_device: &'static str,
}

fn main() {
    env_logger::init();

    info!(
        "OpenFlash Banana Pi agent v{}.{}.{}, protocol v{}",
        VERSION.0,
        VERSION.1,
        VERSION.2,
        openflash_sbc_agent::PROTOCOL_VERSION
    );

    let board = detect_board();
    match board {
        Some(board) => info!("Detected {} ({})", board.name, board.soc),
        None => warn!(
            "Could not identify the board; assuming {}. Supported: M2 Zero (H3), \
             M4 Berry (H618), BPI-F3 (K1)",
            spi::DEFAULT_DEVICE
        ),
    }

    let device = board
        .map(|board| board.spi_device)
        .unwrap_or(spi::DEFAULT_DEVICE);

    // An agent that cannot open the bus still starts and still answers: it
    // reports no interfaces, so a host learns that once instead of having every
    // operation fail separately.
    let bus = match spi::SpidevBus::open(device) {
        Ok(bus) => bus,
        Err(error) => {
            error!("Cannot open {device}: {error}");
            error!("Enable the spidev overlay and check that the device node exists");
            spi::SpidevBus::unavailable(error.to_string())
        }
    };

    let mut agent = Agent::new(SpiNor::new(bus), Platform::BananaPi, VERSION);

    match std::env::var("OPENFLASH_TCP") {
        Ok(address) => listen_tcp(&address, &mut agent),
        Err(_) => listen_unix(SOCKET_PATH, &mut agent),
    }
}

/// Identify the board from the device tree.
fn detect_board() -> Option<BoardInfo> {
    let model = std::fs::read_to_string("/proc/device-tree/model").unwrap_or_default();
    let compatible = std::fs::read_to_string("/proc/device-tree/compatible").unwrap_or_default();
    board_from_device_tree(&model, &compatible)
}

fn board_from_device_tree(model: &str, compatible: &str) -> Option<BoardInfo> {
    // Checked most specific first: "F3" alone would otherwise match other models.
    if model.contains("M2 Zero") || model.contains("BPI-M2-Zero") || compatible.contains("sun8i-h3")
    {
        return Some(BoardInfo {
            name: "Banana Pi M2 Zero",
            soc: "Allwinner H3",
            spi_device: "/dev/spidev0.0",
        });
    }

    if model.contains("M4 Berry")
        || model.contains("BPI-M4-Berry")
        || compatible.contains("sun50i-h618")
    {
        return Some(BoardInfo {
            name: "Banana Pi M4 Berry",
            soc: "Allwinner H618",
            spi_device: "/dev/spidev0.0",
        });
    }

    if model.contains("BPI-F3") || compatible.contains("spacemit") {
        return Some(BoardInfo {
            name: "Banana Pi BPI-F3",
            soc: "SpacemiT K1 (RISC-V)",
            spi_device: "/dev/spidev0.0",
        });
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_supported_board_is_recognised() {
        assert_eq!(
            board_from_device_tree("Banana Pi BPI-M2-Zero", "").map(|b| b.soc),
            Some("Allwinner H3")
        );
        assert_eq!(
            board_from_device_tree("Banana Pi M4 Berry", "").map(|b| b.soc),
            Some("Allwinner H618")
        );
        assert_eq!(
            board_from_device_tree("Banana Pi BPI-F3", "").map(|b| b.soc),
            Some("SpacemiT K1 (RISC-V)")
        );
    }

    #[test]
    fn the_compatible_string_is_enough_on_its_own() {
        assert_eq!(
            board_from_device_tree("", "allwinner,sun50i-h618").map(|b| b.name),
            Some("Banana Pi M4 Berry")
        );
        assert_eq!(
            board_from_device_tree("", "spacemit,k1").map(|b| b.name),
            Some("Banana Pi BPI-F3")
        );
    }

    /// The old detection matched a bare "F3", which appears in unrelated model
    /// strings. An unrecognised board must fall through to the default rather
    /// than be misidentified.
    #[test]
    fn an_unrelated_model_is_not_matched() {
        assert_eq!(board_from_device_tree("Some Board F3000", ""), None);
        assert_eq!(board_from_device_tree("Raspberry Pi 4 Model B", ""), None);
    }

    /// A host is told this as the firmware version, so it has to correspond to
    /// the build rather than to a number someone typed once.
    #[test]
    fn the_reported_version_matches_the_crate() {
        let reported = format!("{}.{}.{}", VERSION.0, VERSION.1, VERSION.2);
        assert_eq!(reported, env!("CARGO_PKG_VERSION"));
    }
}
