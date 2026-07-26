//! The OpenFlash wire protocol.
//!
//! This crate is the single source of truth for how an OpenFlash host talks to
//! an OpenFlash device, and it is shared verbatim by both sides. It is
//! `no_std` and allocation-free by default so that bare-metal firmware can
//! depend on it:
//!
//! ```toml
//! openflash-protocol = { version = "3.1", default-features = false }
//! ```
//!
//! # Why it exists
//!
//! Each firmware used to declare its own copy of the command table, and the
//! copies had drifted: `Ping` was `0x01` on some boards and `0x00` on others,
//! parallel-NAND reads were `0x05`, `0x11` or `0x12` depending on the board,
//! and SPI NOR lived at either `0x60` or `0x70`. A host could therefore only
//! ever talk to a subset of the "supported" platforms, and there was nothing in
//! the build to catch the drift. Everything host- and device-side now reads its
//! opcodes from here; `tests/firmware_conformance.rs` fails the build if a
//! firmware source reintroduces its own table.
//!
//! # Layout
//!
//! - [`command`] — the opcode table, interfaces and status codes.
//! - [`frame`] — revision 2 framing: magic, length and CRC-16 on both header
//!   and payload, so a corrupt link is detected rather than silently returning
//!   wrong flash contents.
//! - [`crc`] — the CRC-16/CCITT-FALSE used by the framing.

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod command;
pub mod crc;
pub mod frame;

pub use command::{Command, FlashInterface, Status, LEGACY_NAND_ALIASES, PROTOCOL_VERSION};
pub use frame::{Frame, FrameError, MAGIC, MAX_FRAME, MAX_PAYLOAD};

/// Platform identifier a device reports in its [`Command::GetVersion`] reply.
///
/// The byte is assigned per board so the host can name the hardware it is
/// talking to, and so it can refuse operations a board cannot perform.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Platform {
    /// Raspberry Pi Pico / Pico W.
    Rp2040 = 0x01,
    /// STM32F103 "Blue Pill".
    Stm32f1 = 0x02,
    /// STM32F401/F411 "Black Pill".
    Stm32f4 = 0x03,
    /// ESP32.
    Esp32 = 0x04,
    /// Raspberry Pi Pico 2.
    Rp2350 = 0x05,
    /// Raspberry Pi single-board computer.
    RaspberryPi = 0x10,
    /// Orange Pi single-board computer.
    OrangePi = 0x11,
    /// Banana Pi single-board computer.
    BananaPi = 0x12,
    /// Arduino GIGA R1 WiFi.
    ArduinoGiga = 0x20,
    /// Teensy 4.0.
    Teensy40 = 0x30,
    /// Teensy 4.1.
    Teensy41 = 0x31,
}

impl Platform {
    /// Decode a platform byte.
    pub fn from_u8(value: u8) -> Option<Self> {
        Some(match value {
            0x01 => Self::Rp2040,
            0x02 => Self::Stm32f1,
            0x03 => Self::Stm32f4,
            0x04 => Self::Esp32,
            0x05 => Self::Rp2350,
            0x10 => Self::RaspberryPi,
            0x11 => Self::OrangePi,
            0x12 => Self::BananaPi,
            0x20 => Self::ArduinoGiga,
            0x30 => Self::Teensy40,
            0x31 => Self::Teensy41,
            _ => return None,
        })
    }

    /// Board name for display.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Rp2040 => "Raspberry Pi Pico (RP2040)",
            Self::Stm32f1 => "STM32F103 (Blue Pill)",
            Self::Stm32f4 => "STM32F4 (Black Pill)",
            Self::Esp32 => "ESP32",
            Self::Rp2350 => "Raspberry Pi Pico 2 (RP2350)",
            Self::RaspberryPi => "Raspberry Pi (SBC)",
            Self::OrangePi => "Orange Pi (SBC)",
            Self::BananaPi => "Banana Pi (SBC)",
            Self::ArduinoGiga => "Arduino GIGA R1 WiFi",
            Self::Teensy40 => "Teensy 4.0",
            Self::Teensy41 => "Teensy 4.1",
        }
    }

    /// Every platform, for exhaustive tests and documentation.
    pub const ALL: &'static [Platform] = &[
        Self::Rp2040,
        Self::Stm32f1,
        Self::Stm32f4,
        Self::Esp32,
        Self::Rp2350,
        Self::RaspberryPi,
        Self::OrangePi,
        Self::BananaPi,
        Self::ArduinoGiga,
        Self::Teensy40,
        Self::Teensy41,
    ];
}

/// USB vendor id the device firmware presents.
pub const USB_VENDOR_ID: u16 = 0xC0DE;

/// USB product id the device firmware presents.
pub const USB_PRODUCT_ID: u16 = 0xCAFE;

/// Bulk OUT endpoint (host to device).
pub const USB_ENDPOINT_OUT: u8 = 0x01;

/// Bulk IN endpoint (device to host).
pub const USB_ENDPOINT_IN: u8 = 0x81;

/// Payload of a [`Command::GetVersion`] response.
///
/// Fixed six-byte layout: `[protocol, fw_major, fw_minor, fw_patch, platform,
/// interface_bitmap]`. The interface bitmap has bit `n` set when the device
/// implements the interface whose [`FlashInterface`] value is `n`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct VersionInfo {
    /// Protocol revision the device speaks.
    pub protocol: u8,
    /// Firmware version as `(major, minor, patch)`.
    pub firmware: (u8, u8, u8),
    /// Raw platform byte; `None` when the host does not recognise it.
    pub platform: Option<Platform>,
    /// Bitmap of implemented interfaces.
    pub interfaces: u8,
}

/// Size of a [`VersionInfo`] payload on the wire.
pub const VERSION_INFO_LEN: usize = 6;

impl VersionInfo {
    /// Encode into the fixed six-byte wire layout.
    pub fn to_bytes(&self) -> [u8; VERSION_INFO_LEN] {
        [
            self.protocol,
            self.firmware.0,
            self.firmware.1,
            self.firmware.2,
            self.platform.map(|p| p as u8).unwrap_or(0),
            self.interfaces,
        ]
    }

    /// Decode the fixed six-byte wire layout.
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < VERSION_INFO_LEN {
            return None;
        }
        Some(Self {
            protocol: bytes[0],
            firmware: (bytes[1], bytes[2], bytes[3]),
            platform: Platform::from_u8(bytes[4]),
            interfaces: bytes[5],
        })
    }

    /// Whether the device claims to implement `interface`.
    pub fn supports(&self, interface: FlashInterface) -> bool {
        self.interfaces & (1 << (interface as u8)) != 0
    }

    /// Build an interface bitmap from a list of interfaces.
    pub fn bitmap_of(interfaces: &[FlashInterface]) -> u8 {
        let mut bitmap = 0u8;
        let mut i = 0;
        while i < interfaces.len() {
            bitmap |= 1 << (interfaces[i] as u8);
            i += 1;
        }
        bitmap
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn platform_bytes_round_trip_and_are_unique() {
        let mut seen = [false; 256];
        for &platform in Platform::ALL {
            let byte = platform as u8;
            assert!(!seen[byte as usize], "0x{byte:02X} is assigned twice");
            seen[byte as usize] = true;
            assert_eq!(Platform::from_u8(byte), Some(platform));
            assert!(!platform.name().is_empty());
        }
        assert_eq!(Platform::from_u8(0x00), None);
        assert_eq!(Platform::from_u8(0xFF), None);
    }

    #[test]
    fn version_info_round_trips() {
        let info = VersionInfo {
            protocol: PROTOCOL_VERSION,
            firmware: (3, 1, 0),
            platform: Some(Platform::Rp2040),
            interfaces: VersionInfo::bitmap_of(&[
                FlashInterface::ParallelNand,
                FlashInterface::SpiNor,
            ]),
        };
        let bytes = info.to_bytes();
        assert_eq!(bytes.len(), VERSION_INFO_LEN);
        assert_eq!(VersionInfo::from_bytes(&bytes), Some(info));
    }

    #[test]
    fn version_info_reports_only_the_interfaces_in_its_bitmap() {
        let info = VersionInfo {
            protocol: PROTOCOL_VERSION,
            firmware: (1, 0, 0),
            platform: Some(Platform::Esp32),
            interfaces: VersionInfo::bitmap_of(&[FlashInterface::SpiNor, FlashInterface::SpiNand]),
        };
        assert!(info.supports(FlashInterface::SpiNor));
        assert!(info.supports(FlashInterface::SpiNand));
        assert!(!info.supports(FlashInterface::ParallelNand));
        assert!(!info.supports(FlashInterface::Emmc));
        assert!(!info.supports(FlashInterface::Ufs));
    }

    #[test]
    fn version_info_rejects_a_short_payload() {
        assert_eq!(VersionInfo::from_bytes(&[1, 2, 3]), None);
    }
}
