//! OpenFlash agent for the Raspberry Pi.
//!
//! A Linux daemon that drives the flash chip through the Pi's own SPI controller
//! and serves hosts over a Unix socket or TCP:
//!
//! ```text
//! openflash --unix /tmp/openflash.sock detect
//! openflash --unix /tmp/openflash.sock read -o dump.bin
//! ```
//!
//! The protocol handling and the SPI NOR sequencing come from
//! `openflash-sbc-agent`, shared with the Orange Pi and Banana Pi agents, so
//! there is one implementation of each rather than three that disagree — which is
//! how `Ping` ended up as `0x00` on some boards and `0x01` on others.

use log::{error, info, warn};

use openflash_sbc_agent::{listen_tcp, listen_unix, Agent, Platform, SpiNor};

mod gpio_nand;
mod spi;

/// Agent version, reported in the `GetVersion` reply.
const VERSION: (u8, u8, u8) = (3, 1, 0);

/// Default Unix socket path.
const SOCKET_PATH: &str = "/tmp/openflash.sock";

fn main() {
    env_logger::init();

    info!(
        "OpenFlash Raspberry Pi agent v{}.{}.{}, protocol v{}",
        VERSION.0,
        VERSION.1,
        VERSION.2,
        openflash_sbc_agent::PROTOCOL_VERSION
    );

    match detect_pi_model() {
        Some(model) => info!("Detected {model}"),
        // Not fatal: the agent works on any board with a usable spidev, and
        // refusing to start on an unrecognised revision would be unhelpful.
        None => warn!("Could not identify the board from /proc/cpuinfo; continuing anyway"),
    }

    // An agent that cannot open the bus still starts and still answers: it
    // reports no interfaces, so a host learns that once instead of having every
    // operation fail separately.
    let bus = match spi::RppalBus::open() {
        Ok(bus) => bus,
        Err(error) => {
            error!("Cannot open the SPI device: {error}");
            error!("Enable SPI with `raspi-config` and check that /dev/spidev0.0 exists");
            spi::RppalBus::unavailable(error.to_string())
        }
    };

    let mut agent = Agent::new(SpiNor::new(bus), Platform::RaspberryPi, VERSION);

    match std::env::var("OPENFLASH_TCP") {
        Ok(address) => listen_tcp(&address, &mut agent),
        Err(_) => listen_unix(SOCKET_PATH, &mut agent),
    }
}

/// Identify the board from `/proc/cpuinfo`.
fn detect_pi_model() -> Option<&'static str> {
    let cpuinfo = std::fs::read_to_string("/proc/cpuinfo").ok()?;
    model_from_cpuinfo(&cpuinfo)
}

fn model_from_cpuinfo(cpuinfo: &str) -> Option<&'static str> {
    // Ordered so BCM2710 does not match before BCM2711.
    const MODELS: &[(&str, &str)] = &[
        ("BCM2712", "Raspberry Pi 5"),
        ("BCM2711", "Raspberry Pi 4"),
        ("BCM2837", "Raspberry Pi 3B+"),
        ("BCM2710", "Raspberry Pi Zero 2W"),
        ("BCM2835", "Raspberry Pi Zero / 1"),
    ];

    MODELS
        .iter()
        .find(|(soc, _)| cpuinfo.contains(soc))
        .map(|(_, name)| *name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn board_detection_prefers_the_longer_soc_match() {
        assert_eq!(
            model_from_cpuinfo("Hardware\t: BCM2711\n"),
            Some("Raspberry Pi 4")
        );
        assert_eq!(
            model_from_cpuinfo("Hardware\t: BCM2710A1\n"),
            Some("Raspberry Pi Zero 2W")
        );
        assert_eq!(model_from_cpuinfo("Hardware\t: SomethingElse\n"), None);
    }

    /// A host is told this as the firmware version, so it has to correspond to
    /// the build rather than to a number someone typed once.
    #[test]
    fn the_reported_version_matches_the_crate() {
        let reported = format!("{}.{}.{}", VERSION.0, VERSION.1, VERSION.2);
        assert_eq!(reported, env!("CARGO_PKG_VERSION"));
    }
}
