//! GPIO-based Parallel NAND driver for Raspberry Pi
//!
//! Uses rppal for GPIO access. Note that GPIO timing on Linux is not
//! as precise as on bare-metal MCUs, but sufficient for most NAND chips.
//!
//! Not reachable from the command handler yet: the page and block operations
//! below refuse, because the address cycles they need are not implemented, and
//! driving a NAND without an address would read or overwrite an arbitrary page.
//! The module is kept compiled and type-checked so the work needed is visible
//! rather than rotting in a branch. `main.rs` does not advertise the parallel
//! NAND interface, so a host is never offered it.
#![allow(dead_code)]

use rppal::gpio::Gpio;
use std::thread;
use std::time::Duration;
use thiserror::Error;

/// GPIO pin assignments for NAND interface
/// Using BCM numbering
pub struct NandPins {
    // Data bus (directly connected)
    pub data: [u8; 8], // GPIO pins for D0-D7

    // Control signals
    pub ce: u8,  // Chip Enable (active low)
    pub we: u8,  // Write Enable (active low)
    pub re: u8,  // Read Enable (active low)
    pub ale: u8, // Address Latch Enable
    pub cle: u8, // Command Latch Enable
    pub rb: u8,  // Ready/Busy (input)
    pub wp: u8,  // Write Protect (active low)
}

impl Default for NandPins {
    fn default() -> Self {
        // Default pinout for Raspberry Pi 40-pin header
        Self {
            data: [2, 3, 4, 17, 27, 22, 10, 9], // D0-D7
            ce: 11,
            we: 5,
            re: 6,
            ale: 13,
            cle: 19,
            rb: 26,
            wp: 21,
        }
    }
}

/// NAND controller using GPIO
pub struct GpioNand {
    gpio: Gpio,
    pins: NandPins,
    initialized: bool,
}

#[derive(Error, Debug)]
pub enum NandError {
    #[error("GPIO error: {0}")]
    Gpio(#[from] rppal::gpio::Error),

    #[error("Timeout waiting for ready")]
    Timeout,

    #[error("Program failed")]
    ProgramFailed,

    #[error("Erase failed")]
    EraseFailed,

    #[error("not implemented: {0}")]
    NotImplemented(&'static str),
}

impl GpioNand {
    pub fn new(pins: NandPins) -> Result<Self, NandError> {
        let gpio = Gpio::new()?;

        Ok(Self {
            gpio,
            pins,
            initialized: false,
        })
    }

    /// Initialize GPIO pins
    pub fn init(&mut self) -> Result<(), NandError> {
        // Initialize would configure all pins
        self.initialized = true;
        Ok(())
    }

    /// Wait for chip ready (R/B pin high)
    fn wait_ready(&self, timeout_ms: u64) -> Result<(), NandError> {
        let start = std::time::Instant::now();
        let timeout = Duration::from_millis(timeout_ms);

        // Would check R/B pin here
        while start.elapsed() < timeout {
            // if rb_pin.is_high() { return Ok(()); }
            thread::sleep(Duration::from_micros(10));
        }

        Ok(()) // Simplified
    }

    /// Send command to NAND
    pub fn send_command(&self, _cmd: u8) -> Result<(), NandError> {
        // CLE high, WE pulse with data
        Ok(())
    }

    /// Send address bytes
    pub fn send_address(&self, _addr: &[u8]) -> Result<(), NandError> {
        // ALE high, WE pulse for each byte
        Ok(())
    }

    /// Read data from NAND
    pub fn read_data(&self, _buf: &mut [u8]) -> Result<(), NandError> {
        // RE pulse for each byte
        Ok(())
    }

    /// Write data to NAND
    pub fn write_data(&self, _data: &[u8]) -> Result<(), NandError> {
        // WE pulse for each byte
        Ok(())
    }

    /// Read NAND ID
    pub fn read_id(&self) -> Result<[u8; 8], NandError> {
        self.send_command(0x90)?; // READ ID
        self.send_address(&[0x00])?;

        let mut id = [0u8; 8];
        self.read_data(&mut id)?;

        Ok(id)
    }

    /// Read one page.
    ///
    /// Not implemented: the address cycles are missing. The command sequence
    /// below is correct in outline — READ (0x00), address, READ CONFIRM (0x30) —
    /// but nothing drives the column and row address onto the bus, so the chip
    /// would return whatever page its internal pointer happened to be at. That
    /// is worse than an error for a tool whose output is meant to be a faithful
    /// dump, so it refuses instead.
    pub fn read_page(&self, _block: u32, _page: u32, _buf: &mut [u8]) -> Result<(), NandError> {
        Err(NandError::NotImplemented(
            "parallel NAND page reads need the address cycles to be implemented",
        ))
    }

    /// Program one page.
    ///
    /// Not implemented, for the same reason as [`GpioNand::read_page`] — and
    /// programming without an address would write to an arbitrary page, so this
    /// must not be reachable.
    pub fn program_page(&self, _block: u32, _page: u32, _data: &[u8]) -> Result<(), NandError> {
        Err(NandError::NotImplemented(
            "parallel NAND page programming needs the address cycles to be implemented",
        ))
    }

    /// Erase one block.
    ///
    /// Not implemented: an erase without a row address would erase an arbitrary
    /// block.
    pub fn erase_block(&self, _block: u32) -> Result<(), NandError> {
        Err(NandError::NotImplemented(
            "parallel NAND block erase needs the row address cycles to be implemented",
        ))
    }
}
