//! The Raspberry Pi's SPI controller, through rppal.
//!
//! One [`SpiBus`] call is one SPI transaction: rppal's `transfer` issues a single
//! `SPI_IOC_MESSAGE`, so chip select stays asserted across the opcode, the
//! address and the data. A flash chip that sees chip select go high in between
//! discards the command.

use rppal::spi::{Bus, Mode, SlaveSelect, Spi};

use openflash_sbc_agent::{BusError, BusResult, SpiBus};

/// Clock rate for flash access.
///
/// 10 MHz is comfortable for every SPI NOR part and for the jumper wiring people
/// actually use; the parts support more, the wiring often does not.
const CLOCK_HZ: u32 = 10_000_000;

/// The Pi's SPI bus, or a record of why it is unusable.
pub struct RppalBus {
    device: Option<Spi>,
    unavailable_reason: String,
}

impl RppalBus {
    /// Open SPI0 with chip select 0.
    pub fn open() -> Result<Self, rppal::spi::Error> {
        let device = Spi::new(Bus::Spi0, SlaveSelect::Ss0, CLOCK_HZ, Mode::Mode0)?;
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

    fn device(&self) -> BusResult<&Spi> {
        self.device
            .as_ref()
            .ok_or_else(|| BusError::Unavailable(self.unavailable_reason.clone()))
    }

    /// rppal's `write` takes `&mut self`, unlike its `transfer`.
    fn device_mut(&mut self) -> BusResult<&mut Spi> {
        let reason = self.unavailable_reason.clone();
        self.device.as_mut().ok_or(BusError::Unavailable(reason))
    }
}

impl SpiBus for RppalBus {
    fn transfer(&mut self, write: &[u8], read: &mut [u8]) -> BusResult<()> {
        debug_assert_eq!(write.len(), read.len(), "a transfer is symmetric");
        self.device()?
            .transfer(read, write)
            .map(|_| ())
            .map_err(|error| BusError::Transfer(error.to_string()))
    }

    fn write(&mut self, data: &[u8]) -> BusResult<()> {
        self.device_mut()?
            .write(data)
            .map(|_| ())
            .map_err(|error| BusError::Transfer(error.to_string()))
    }

    fn is_available(&self) -> bool {
        self.device.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// CI has no SPI device, so what is testable here is that a missing bus is
    /// reported rather than papered over — which is what the agent relies on to
    /// advertise no interfaces.
    #[test]
    fn a_missing_device_is_reported_as_unavailable() {
        let mut bus = RppalBus::unavailable("SPI is not enabled".to_string());
        assert!(!bus.is_available());

        let mut read = [0u8; 4];
        match bus.transfer(&[0u8; 4], &mut read) {
            Err(BusError::Unavailable(reason)) => assert_eq!(reason, "SPI is not enabled"),
            other => panic!("expected an unavailable error, got {other:?}"),
        }
        assert!(matches!(bus.write(&[0x06]), Err(BusError::Unavailable(_))));
    }
}
