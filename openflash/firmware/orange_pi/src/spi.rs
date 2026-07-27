//! The Orange Pi's SPI controller, through Linux spidev.
//!
//! One [`SpiBus`] call is one SPI transaction. That is the whole reason this uses
//! `SPI_IOC_MESSAGE` (via the `spidev` crate) rather than plain `read`/`write` on
//! the device node: separate syscalls let the kernel deassert chip select between
//! them, and a flash chip that sees chip select go high after the opcode discards
//! the command. The previous implementation did exactly that — `write_all(data)`
//! followed by `read_exact(data)`.

use spidev::{SpiModeFlags, Spidev, SpidevOptions, SpidevTransfer};

use openflash_sbc_agent::{BusError, BusResult, SpiBus};

/// Device node the agent opens by default.
pub const DEFAULT_DEVICE: &str = "/dev/spidev0.0";

/// Clock rate for flash access.
///
/// 10 MHz is comfortable for every SPI NOR part and for the jumper wiring people
/// actually use; the parts support more, the wiring often does not.
const CLOCK_HZ: u32 = 10_000_000;

/// The board's SPI bus, or a record of why it is unusable.
pub struct SpidevBus {
    device: Option<Spidev>,
    unavailable_reason: String,
}

impl SpidevBus {
    /// Open and configure a spidev device.
    pub fn open(path: &str) -> std::io::Result<Self> {
        let mut device = Spidev::open(path)?;
        device.configure(
            &SpidevOptions::new()
                .bits_per_word(8)
                .max_speed_hz(CLOCK_HZ)
                .mode(SpiModeFlags::SPI_MODE_0)
                .build(),
        )?;

        Ok(Self {
            device: Some(device),
            unavailable_reason: String::new(),
        })
    }

    /// A bus that is not there, carrying the reason so the host can be told.
    pub fn unavailable(reason: String) -> Self {
        Self {
            device: None,
            unavailable_reason: reason,
        }
    }

    fn device(&self) -> BusResult<&Spidev> {
        self.device
            .as_ref()
            .ok_or_else(|| BusError::Unavailable(self.unavailable_reason.clone()))
    }
}

impl SpiBus for SpidevBus {
    fn transfer(&mut self, write: &[u8], read: &mut [u8]) -> BusResult<()> {
        debug_assert_eq!(write.len(), read.len(), "a transfer is symmetric");
        let device = self.device()?;

        // One ioctl, so chip select stays asserted for the whole exchange.
        let mut transfer = SpidevTransfer::read_write(write, read);
        device
            .transfer(&mut transfer)
            .map_err(|error| BusError::Transfer(error.to_string()))
    }

    fn write(&mut self, data: &[u8]) -> BusResult<()> {
        let device = self.device()?;
        let mut transfer = SpidevTransfer::write(data);
        device
            .transfer(&mut transfer)
            .map_err(|error| BusError::Transfer(error.to_string()))
    }

    fn is_available(&self) -> bool {
        self.device.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// CI has no spidev node, so what is testable here is that a missing bus is
    /// reported rather than papered over — which is what the agent relies on to
    /// advertise no interfaces.
    #[test]
    fn a_missing_device_is_reported_as_unavailable() {
        let mut bus = SpidevBus::unavailable("no such device".to_string());
        assert!(!bus.is_available());

        let mut read = [0u8; 4];
        match bus.transfer(&[0u8; 4], &mut read) {
            Err(BusError::Unavailable(reason)) => assert_eq!(reason, "no such device"),
            other => panic!("expected an unavailable error, got {other:?}"),
        }
        assert!(matches!(bus.write(&[0x06]), Err(BusError::Unavailable(_))));
    }

    #[test]
    fn opening_a_nonexistent_device_fails_rather_than_panicking() {
        assert!(SpidevBus::open("/dev/definitely-not-a-spi-device").is_err());
    }
}
