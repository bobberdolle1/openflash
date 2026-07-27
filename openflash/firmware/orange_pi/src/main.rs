//! OpenFlash agent for Orange Pi boards.
//!
//! A Linux daemon that drives the flash chip through the board's SPI controller
//! and serves hosts over a Unix socket or TCP. The protocol handling and the SPI
//! NOR sequencing come from `openflash-sbc-agent`, shared with the Raspberry Pi
//! and Banana Pi agents; this file is the board-specific part.
//!
//! Supported boards: Orange Pi Zero 3 (Allwinner H618), Zero 2W (H616),
//! Orange Pi 5 (Rockchip RK3588).
//!
//! # What this replaces
//!
//! The previous version declared its own opcode table in which `Ping` was `0x00`
//! rather than `0x01`, so a current host could not even ping it. It replied
//! without framing, and answered `0xFF` to everything except ping, version and
//! device info — the SPI and GPIO modules were never connected to the command
//! handler. Its SPI transfer also issued a `write` syscall followed by a separate
//! `read` on `/dev/spidev*`, which lets the kernel deassert chip select in
//! between, so a flash chip discards the command before the data arrives.

use log::{error, info, warn};

use openflash_sbc_agent::{listen_tcp, listen_unix, Agent, Platform, SpiNor};

mod gpio;
mod spi;

/// Agent version, reported in the `GetVersion` reply.
const VERSION: (u8, u8, u8) = (3, 1, 0);

/// Default Unix socket path.
const SOCKET_PATH: &str = "/tmp/openflash.sock";

fn main() {
    env_logger::init();

    info!(
        "OpenFlash Orange Pi agent v{}.{}.{}, protocol v{}",
        VERSION.0,
        VERSION.1,
        VERSION.2,
        openflash_sbc_agent::PROTOCOL_VERSION
    );

    match detect_board() {
        Some(board) => info!("Detected {board}"),
        None => warn!("Could not identify the board; continuing anyway"),
    }

    // An agent that cannot open the bus still starts and still answers: it
    // reports no interfaces, so a host learns that once instead of having every
    // operation fail separately.
    let bus = match spi::SpidevBus::open(spi::DEFAULT_DEVICE) {
        Ok(bus) => bus,
        Err(error) => {
            error!("Cannot open {}: {error}", spi::DEFAULT_DEVICE);
            error!("Enable the spidev overlay and check that the device node exists");
            spi::SpidevBus::unavailable(error.to_string())
        }
    };

    let mut agent = Agent::new(SpiNor::new(bus), Platform::OrangePi, VERSION);

    match std::env::var("OPENFLASH_TCP") {
        Ok(address) => listen_tcp(&address, &mut agent),
        Err(_) => listen_unix(SOCKET_PATH, &mut agent),
    }
}

/// Identify the board from the device tree model, falling back to `/proc/cpuinfo`.
fn detect_board() -> Option<&'static str> {
    if let Ok(model) = std::fs::read_to_string("/proc/device-tree/model") {
        if let Some(name) = board_from_model(&model) {
            return Some(name);
        }
    }
    let cpuinfo = std::fs::read_to_string("/proc/cpuinfo").ok()?;
    board_from_model(&cpuinfo)
}

fn board_from_model(text: &str) -> Option<&'static str> {
    // Most specific marker first, so H618 is not matched by a looser pattern.
    const BOARDS: &[(&str, &str)] = &[
        ("RK3588", "Orange Pi 5 (Rockchip RK3588)"),
        ("H618", "Orange Pi Zero 3 (Allwinner H618)"),
        ("H616", "Orange Pi Zero 2W (Allwinner H616)"),
        ("SUN50I", "Orange Pi (Allwinner sun50i)"),
    ];

    let upper = text.to_uppercase();
    BOARDS
        .iter()
        .find(|(marker, _)| upper.contains(marker))
        .map(|(_, name)| *name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn board_detection_recognises_the_supported_socs() {
        assert_eq!(
            board_from_model("Orange Pi 5 rk3588s\n"),
            Some("Orange Pi 5 (Rockchip RK3588)")
        );
        assert_eq!(
            board_from_model("Hardware\t: Allwinner H618\n"),
            Some("Orange Pi Zero 3 (Allwinner H618)")
        );
        assert_eq!(
            board_from_model("Hardware\t: Allwinner H616\n"),
            Some("Orange Pi Zero 2W (Allwinner H616)")
        );
    }

    #[test]
    fn an_unknown_board_is_not_guessed_at() {
        assert_eq!(board_from_model("Raspberry Pi 4 Model B\n"), None);
    }

    /// A host is told this as the firmware version, so it has to correspond to
    /// the build rather than to a number someone typed once.
    #[test]
    fn the_reported_version_matches_the_crate() {
        let reported = format!("{}.{}.{}", VERSION.0, VERSION.1, VERSION.2);
        assert_eq!(reported, env!("CARGO_PKG_VERSION"));
    }
}
