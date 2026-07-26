//! Device management for the GUI, on top of `openflash_core`.
//!
//! This used to hold its own USB code, its own copy of the protocol framing and
//! its own chip-info plumbing. That copy had never compiled — the `UsbDevice`
//! struct header was missing, leaving a dangling field — and its `read_page`
//! consumed the first 64 bytes of page data as an acknowledgement and then read
//! 64 bytes past the end of the page, desynchronising the link.
//!
//! Everything now goes through `openflash_core::transport` and
//! `openflash_core::device`, the same code the CLI uses, so there is one
//! implementation of talking to a device rather than two.

use serde::{Deserialize, Serialize};

use openflash_core::device::{Device, ProgramOptions};
use openflash_core::emulator::EmulatedDevice;
use openflash_core::protocol::{Command, Platform};
#[cfg(unix)]
use openflash_core::transport::UnixTransport;
#[cfg(feature = "usb")]
use openflash_core::transport::{list_devices, UsbTransport};
use openflash_core::transport::{TcpTransport, Transport};

pub use openflash_core::protocol::FlashInterface;

/// Size of the emulated chip offered by the GUI's demo mode.
const EMULATED_CHIP_SIZE: usize = 2 * 1024 * 1024;

/// Board the connected device runs on.
///
/// Mirrors `openflash_protocol::Platform`, kept as its own type because the
/// frontend deserialises these names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DevicePlatform {
    Unknown,
    Rp2040,
    Stm32f1,
    Stm32f4,
    Esp32,
    Rp2350,
    RaspberryPi,
    OrangePi,
    BananaPi,
    ArduinoGiga,
    Teensy40,
    Teensy41,
}

impl From<Option<Platform>> for DevicePlatform {
    fn from(platform: Option<Platform>) -> Self {
        match platform {
            Some(Platform::Rp2040) => Self::Rp2040,
            Some(Platform::Stm32f1) => Self::Stm32f1,
            Some(Platform::Stm32f4) => Self::Stm32f4,
            Some(Platform::Esp32) => Self::Esp32,
            Some(Platform::Rp2350) => Self::Rp2350,
            Some(Platform::RaspberryPi) => Self::RaspberryPi,
            Some(Platform::OrangePi) => Self::OrangePi,
            Some(Platform::BananaPi) => Self::BananaPi,
            Some(Platform::ArduinoGiga) => Self::ArduinoGiga,
            Some(Platform::Teensy40) => Self::Teensy40,
            Some(Platform::Teensy41) => Self::Teensy41,
            None => Self::Unknown,
        }
    }
}

impl DevicePlatform {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Unknown => "Unknown device",
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

    pub fn is_sbc(&self) -> bool {
        matches!(self, Self::RaspberryPi | Self::OrangePi | Self::BananaPi)
    }
}

/// Which interfaces the connected device implements.
///
/// Derived from the interface bitmap in the device's `GetVersion` reply, so it
/// reflects what the firmware actually supports rather than what the board could
/// support in principle.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub struct DeviceCapabilities {
    pub parallel_nand: bool,
    pub spi_nand: bool,
    pub spi_nor: bool,
    pub emmc: bool,
    pub ufs: bool,
}

impl DeviceCapabilities {
    fn from_version(version: &openflash_core::protocol::VersionInfo) -> Self {
        Self {
            parallel_nand: version.supports(FlashInterface::ParallelNand),
            spi_nand: version.supports(FlashInterface::SpiNand),
            spi_nor: version.supports(FlashInterface::SpiNor),
            emmc: version.supports(FlashInterface::Emmc),
            ufs: version.supports(FlashInterface::Ufs),
        }
    }
}

/// How a device is reached.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConnectionType {
    Usb,
    Tcp {
        host: String,
        port: u16,
    },
    #[cfg(unix)]
    UnixSocket {
        path: String,
    },
    /// The in-process emulator. Always surfaced to the user as emulated.
    Emulated,
}

/// A device the GUI knows about.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub id: String,
    pub name: String,
    pub serial: Option<String>,
    pub connected: bool,
    #[serde(default)]
    pub platform: Option<DevicePlatform>,
    #[serde(default)]
    pub capabilities: Option<DeviceCapabilities>,
    #[serde(default)]
    pub connection_type: Option<ConnectionType>,
    #[serde(default)]
    pub protocol_version: Option<u8>,
    #[serde(default)]
    pub firmware_version: Option<String>,
    /// True when this entry is the emulator rather than hardware.
    #[serde(default)]
    pub emulated: bool,
}

/// Chip details shown in the UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChipInfo {
    pub manufacturer: String,
    pub model: String,
    pub chip_id: Vec<u8>,
    pub size_mb: u32,
    pub page_size: u32,
    pub block_size: u32,
    pub interface: FlashInterface,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sector_size: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jedec_id: Option<Vec<u8>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_qspi: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_dual: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voltage: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_clock_mhz: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protected: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub luns: Option<Vec<UfsLunInfo>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ufs_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub serial_number: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub boot_lun_enabled: Option<bool>,
    /// Whether the chip id matched the database exactly, or the geometry was
    /// inferred from the capacity byte.
    #[serde(default)]
    pub exact_match: bool,
}

/// UFS logical unit details.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UfsLunInfo {
    #[serde(rename = "type")]
    pub lun_type: String,
    pub capacity_bytes: u64,
    pub block_size: u32,
    pub enabled: bool,
    pub write_protected: bool,
}

/// Holds the discovered devices and the open connection, if any.
pub struct DeviceManager {
    discovered: Vec<DeviceInfo>,
    active: Option<Device<Box<dyn Transport>>>,
    active_id: Option<String>,
    interface: FlashInterface,
}

impl DeviceManager {
    pub fn new() -> Self {
        Self {
            discovered: Vec::new(),
            active: None,
            active_id: None,
            interface: FlashInterface::SpiNor,
        }
    }

    /// Enumerate attached devices, plus the always-available emulator entry.
    pub fn scan_devices(&mut self) -> Vec<DeviceInfo> {
        let mut devices = Vec::new();

        #[cfg(feature = "usb")]
        match list_devices() {
            Ok(found) => {
                for device in found {
                    devices.push(DeviceInfo {
                        id: device.selector(),
                        name: device
                            .product
                            .clone()
                            .unwrap_or_else(|| "OpenFlash device".to_string()),
                        serial: device.serial_number.clone(),
                        connected: false,
                        platform: None,
                        capabilities: None,
                        connection_type: Some(ConnectionType::Usb),
                        protocol_version: None,
                        firmware_version: None,
                        emulated: false,
                    });
                }
            }
            // A host with no USB subsystem is not an error worth failing a scan
            // over; the list simply has no hardware in it.
            Err(error) => log_scan_failure(&error.to_string()),
        }

        devices.push(DeviceInfo {
            id: "emulated".to_string(),
            name: "Emulated SPI NOR chip (no hardware)".to_string(),
            serial: None,
            connected: false,
            platform: None,
            capabilities: None,
            connection_type: Some(ConnectionType::Emulated),
            protocol_version: None,
            firmware_version: None,
            emulated: true,
        });

        self.discovered = devices.clone();
        devices
    }

    pub fn list_devices(&self) -> Vec<DeviceInfo> {
        self.discovered
            .iter()
            .cloned()
            .map(|mut device| {
                device.connected = self.active_id.as_deref() == Some(device.id.as_str());
                device
            })
            .collect()
    }

    /// Connect to a device by the id `scan_devices` reported.
    ///
    /// `emulated` connects to the in-process emulator; `tcp:host:port` and
    /// `unix:/path` reach an SBC agent; anything else is treated as a USB serial
    /// number or bus address.
    pub fn connect(&mut self, id: &str) -> Result<(), String> {
        let transport: Box<dyn Transport> = if id == "emulated" {
            Box::new(EmulatedDevice::new(
                "EMULATED",
                [0xEF, 0x40, 0x15],
                EMULATED_CHIP_SIZE,
            ))
        } else if let Some(endpoint) = id.strip_prefix("tcp:") {
            Box::new(
                TcpTransport::connect(endpoint, openflash_core::transport::DEFAULT_TIMEOUT)
                    .map_err(|e| format!("cannot reach an agent at {endpoint}: {e}"))?,
            )
        } else if let Some(path) = id.strip_prefix("unix:") {
            #[cfg(unix)]
            {
                Box::new(
                    UnixTransport::connect(path)
                        .map_err(|e| format!("cannot reach an agent on {path}: {e}"))?,
                )
            }
            #[cfg(not(unix))]
            {
                let _ = path;
                return Err("Unix sockets are not available on this platform".to_string());
            }
        } else {
            #[cfg(feature = "usb")]
            {
                Box::new(UsbTransport::open(id).map_err(|e| e.to_string())?)
            }
            #[cfg(not(feature = "usb"))]
            {
                return Err("this build was compiled without USB support".to_string());
            }
        };

        let device = Device::connect(transport).map_err(|e| e.to_string())?;
        self.interface = FlashInterface::SpiNor;
        self.active_id = Some(id.to_string());

        // Fill in what the device reported so the UI can show it.
        let version = *device.version();
        if let Some(entry) = self.discovered.iter_mut().find(|d| d.id == id) {
            entry.platform = Some(DevicePlatform::from(version.platform));
            entry.capabilities = Some(DeviceCapabilities::from_version(&version));
            entry.protocol_version = Some(version.protocol);
            entry.firmware_version = Some(format!(
                "{}.{}.{}",
                version.firmware.0, version.firmware.1, version.firmware.2
            ));
        }

        self.active = Some(device);
        Ok(())
    }

    pub fn disconnect(&mut self) {
        self.active = None;
        self.active_id = None;
    }

    pub fn is_connected(&self) -> bool {
        self.active.is_some()
    }

    /// Whether the open connection is the emulator.
    pub fn is_emulated(&self) -> bool {
        self.active_id.as_deref() == Some("emulated")
    }

    pub fn get_interface(&self) -> FlashInterface {
        self.interface
    }

    /// Ask the device to switch interfaces.
    pub fn set_interface(&mut self, interface: FlashInterface) -> Result<(), String> {
        self.device_mut()?
            .set_interface(interface)
            .map_err(|e| e.to_string())?;
        self.interface = interface;
        Ok(())
    }

    fn device_mut(&mut self) -> Result<&mut Device<Box<dyn Transport>>, String> {
        self.active
            .as_mut()
            .ok_or_else(|| "No device connected".to_string())
    }

    /// Send a command and return the response payload.
    ///
    /// The payload is the data alone: a non-`Ok` status from the device becomes
    /// an error here, so callers no longer have to check a status byte and can
    /// no longer forget to.
    pub fn send_command(&mut self, command: Command, payload: &[u8]) -> Result<Vec<u8>, String> {
        let device = self.device_mut()?;
        device
            .transport_mut()
            .transact(command, payload, openflash_core::transport::DEFAULT_TIMEOUT)
            .map_err(|e| e.to_string())
    }

    /// Read the chip id and look it up.
    pub fn identify(&mut self) -> Result<ChipInfo, String> {
        let chip = self.device_mut()?.identify().map_err(|e| e.to_string())?;
        Ok(ChipInfo {
            manufacturer: chip.manufacturer,
            model: chip.model,
            chip_id: chip.jedec_id.to_vec(),
            size_mb: (chip.capacity / (1024 * 1024)) as u32,
            page_size: chip.page_size,
            block_size: chip.sector_size,
            interface: FlashInterface::SpiNor,
            sector_size: Some(chip.sector_size),
            jedec_id: Some(chip.jedec_id.to_vec()),
            has_qspi: None,
            has_dual: None,
            voltage: None,
            max_clock_mhz: None,
            protected: None,
            luns: None,
            ufs_version: None,
            serial_number: None,
            boot_lun_enabled: None,
            exact_match: chip.exact_match,
        })
    }

    /// Capacity of the connected chip in bytes.
    pub fn capacity(&mut self) -> Result<u64, String> {
        Ok(self.identify()?.size_mb as u64 * 1024 * 1024)
    }

    /// Read a range into memory.
    pub fn read_range(&mut self, start: u64, length: u64) -> Result<Vec<u8>, String> {
        self.device_mut()?
            .read_to_vec(start, length)
            .map_err(|e| e.to_string())
    }

    /// Read a range, reporting progress as `(bytes_done, bytes_total)`.
    pub fn read_range_with_progress(
        &mut self,
        start: u64,
        length: u64,
        progress: &mut dyn FnMut(u64, u64),
    ) -> Result<Vec<u8>, String> {
        let mut buffer = Vec::with_capacity(length as usize);
        self.device_mut()?
            .read_into(start, length, &mut buffer, Some(progress))
            .map_err(|e| e.to_string())?;
        Ok(buffer)
    }

    /// Program data, erasing first and verifying afterwards.
    pub fn program(&mut self, start: u64, data: &[u8]) -> Result<(), String> {
        self.device_mut()?
            .program(start, data, ProgramOptions::default(), None)
            .map(|_| ())
            .map_err(|e| e.to_string())
    }

    /// Erase whole sectors covering the range.
    pub fn erase_range(&mut self, start: u64, length: u64) -> Result<u64, String> {
        self.device_mut()?
            .erase_range(start, length)
            .map_err(|e| e.to_string())
    }

    /// Platform of the connected device.
    pub fn platform(&self) -> Option<DevicePlatform> {
        self.active
            .as_ref()
            .map(|device| DevicePlatform::from(device.version().platform))
    }

    /// Capabilities of the connected device.
    pub fn capabilities(&self) -> Option<DeviceCapabilities> {
        self.active
            .as_ref()
            .map(|device| DeviceCapabilities::from_version(device.version()))
    }
}

impl Default for DeviceManager {
    fn default() -> Self {
        Self::new()
    }
}

fn log_scan_failure(message: &str) {
    eprintln!("openflash: USB enumeration failed: {message}");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_scan_always_offers_the_emulator_and_labels_it() {
        let mut manager = DeviceManager::new();
        let devices = manager.scan_devices();

        let emulated = devices
            .iter()
            .find(|d| d.id == "emulated")
            .expect("the emulator is always offered");
        assert!(emulated.emulated);
        assert!(
            emulated.name.contains("no hardware"),
            "the entry must say it is not hardware: {}",
            emulated.name
        );
    }

    #[test]
    fn commands_without_a_connection_are_refused() {
        let mut manager = DeviceManager::new();
        assert!(!manager.is_connected());
        assert!(manager.send_command(Command::Ping, &[]).is_err());
        assert!(manager.identify().is_err());
        assert!(manager.read_range(0, 16).is_err());
        assert!(manager.program(0, &[0u8; 4]).is_err());
    }

    #[test]
    fn connecting_to_the_emulator_performs_the_handshake_and_fills_in_the_details() {
        let mut manager = DeviceManager::new();
        manager.scan_devices();
        manager.connect("emulated").expect("the emulator answers");

        assert!(manager.is_connected());
        assert!(manager.is_emulated());
        assert_eq!(manager.platform(), Some(DevicePlatform::Rp2040));

        let capabilities = manager.capabilities().unwrap();
        assert!(capabilities.spi_nor);
        assert!(!capabilities.emmc);

        let listed = manager.list_devices();
        let entry = listed.iter().find(|d| d.id == "emulated").unwrap();
        assert!(entry.connected);
        assert_eq!(
            entry.protocol_version,
            Some(openflash_core::protocol::PROTOCOL_VERSION)
        );
    }

    #[test]
    fn identify_names_the_emulated_chip() {
        let mut manager = DeviceManager::new();
        manager.scan_devices();
        manager.connect("emulated").unwrap();

        let chip = manager.identify().unwrap();
        assert_eq!(chip.model, "W25Q16JV");
        assert_eq!(chip.size_mb, 2);
        assert_eq!(chip.jedec_id, Some(vec![0xEF, 0x40, 0x15]));
        assert!(chip.exact_match);
    }

    /// The path that was broken in the old implementation: reading a page
    /// returned data that was offset by 64 bytes and left the link out of step.
    #[test]
    fn a_program_then_read_round_trip_returns_the_same_bytes() {
        let mut manager = DeviceManager::new();
        manager.scan_devices();
        manager.connect("emulated").unwrap();

        let payload: Vec<u8> = (0..3000u32).map(|i| (i % 251) as u8).collect();
        manager.program(0, &payload).unwrap();
        assert_eq!(
            manager.read_range(0, payload.len() as u64).unwrap(),
            payload
        );
    }

    #[test]
    fn progress_is_reported_from_zero_to_the_total() {
        let mut manager = DeviceManager::new();
        manager.scan_devices();
        manager.connect("emulated").unwrap();

        let mut seen = Vec::new();
        let data = manager
            .read_range_with_progress(0, 10_000, &mut |done, total| seen.push((done, total)))
            .unwrap();

        assert_eq!(data.len(), 10_000);
        assert_eq!(seen.first(), Some(&(0, 10_000)));
        assert_eq!(seen.last(), Some(&(10_000, 10_000)));
    }

    #[test]
    fn an_interface_the_device_lacks_is_refused() {
        let mut manager = DeviceManager::new();
        manager.scan_devices();
        manager.connect("emulated").unwrap();

        assert!(manager.set_interface(FlashInterface::Emmc).is_err());
        assert_eq!(manager.get_interface(), FlashInterface::SpiNor);

        manager.set_interface(FlashInterface::SpiNor).unwrap();
        assert_eq!(manager.get_interface(), FlashInterface::SpiNor);
    }

    #[test]
    fn erase_blanks_the_range() {
        let mut manager = DeviceManager::new();
        manager.scan_devices();
        manager.connect("emulated").unwrap();

        manager.program(0, &[0x00; 4096]).unwrap();
        assert_eq!(manager.erase_range(0, 4096).unwrap(), 1);
        assert!(manager
            .read_range(0, 4096)
            .unwrap()
            .iter()
            .all(|&b| b == 0xFF));
    }

    #[test]
    fn disconnecting_clears_the_connection() {
        let mut manager = DeviceManager::new();
        manager.scan_devices();
        manager.connect("emulated").unwrap();
        manager.disconnect();

        assert!(!manager.is_connected());
        assert!(manager.send_command(Command::Ping, &[]).is_err());
        assert!(!manager.list_devices().iter().any(|device| device.connected));
    }
}
