//! PIO-based NAND Flash interface for RP2040
//! Uses Programmable IO for precise timing and parallel data bus access

use embassy_rp::gpio::{Level, Output, Input, Pull, Flex, AnyPin};
use embassy_rp::pio::{Common, Config, Instance, Pio, PioPin, ShiftDirection, StateMachine};
use embassy_time::{Duration, Timer};
use fixed::traits::ToFixed;

/// NAND Flash timing parameters (in nanoseconds)
#[derive(Clone, Copy)]
pub struct NandTiming {
    pub t_wp: u32,   // WE# pulse width (min 12ns for fast NAND)
    pub t_rp: u32,   // RE# pulse width (min 12ns)
    pub t_cls: u32,  // CLE setup time (min 12ns)
    pub t_als: u32,  // ALE setup time (min 12ns)
    pub t_clh: u32,  // CLE hold time (min 5ns)
    pub t_alh: u32,  // ALE hold time (min 5ns)
    pub t_wh: u32,   // WE# high hold time (min 10ns)
    pub t_reh: u32,  // RE# high hold time (min 10ns)
}

impl Default for NandTiming {
    fn default() -> Self {
        // Conservative timing for most NAND chips (ONFI mode 0)
        Self {
            t_wp: 50,
            t_rp: 50,
            t_cls: 50,
            t_als: 50,
            t_clh: 20,
            t_alh: 20,
            t_wh: 30,
            t_reh: 30,
        }
    }
}

impl NandTiming {
    /// Fast timing for ONFI mode 4/5 chips
    pub fn fast() -> Self {
        Self {
            t_wp: 12,
            t_rp: 12,
            t_cls: 12,
            t_als: 12,
            t_clh: 5,
            t_alh: 5,
            t_wh: 10,
            t_reh: 10,
        }
    }
}

/// Pin configuration for NAND interface
pub struct NandPins<'d> {
    // Control signals
    pub cle: Output<'d>,  // Command Latch Enable (active high)
    pub ale: Output<'d>,  // Address Latch Enable (active high)
    pub we: Output<'d>,   // Write Enable (active low)
    pub re: Output<'d>,   // Read Enable (active low)
    pub ce: Output<'d>,   // Chip Enable (active low)
    pub rb: Input<'d>,    // Ready/Busy (low = busy)
    
    // Data bus D0-D7 as flexible pins
    pub d0: Flex<'d>,
    pub d1: Flex<'d>,
    pub d2: Flex<'d>,
    pub d3: Flex<'d>,
    pub d4: Flex<'d>,
    pub d5: Flex<'d>,
    pub d6: Flex<'d>,
    pub d7: Flex<'d>,
}

/// NAND Flash controller using GPIO bit-banging
/// For production, consider using PIO state machines for faster parallel access
pub struct NandController<'d> {
    pins: NandPins<'d>,
    timing: NandTiming,
}

impl<'d> NandController<'d> {
    /// Create a new NAND controller
    pub fn new(mut pins: NandPins<'d>) -> Self {
        // Initialize control signals to idle state
        pins.ce.set_high();   // Chip disabled
        pins.we.set_high();   // WE# idle high
        pins.re.set_high();   // RE# idle high
        pins.cle.set_low();   // CLE idle low
        pins.ale.set_low();   // ALE idle low
        
        // Set data bus as input initially
        pins.d0.set_as_input();
        pins.d1.set_as_input();
        pins.d2.set_as_input();
        pins.d3.set_as_input();
        pins.d4.set_as_input();
        pins.d5.set_as_input();
        pins.d6.set_as_input();
        pins.d7.set_as_input();
        
        Self {
            pins,
            timing: NandTiming::default(),
        }
    }

    /// Set timing parameters
    pub fn set_timing(&mut self, timing: NandTiming) {
        self.timing = timing;
    }

    /// Wait for NAND to become ready (R/B# goes high)
    pub async fn wait_ready(&mut self) {
        while self.pins.rb.is_low() {
            Timer::after(Duration::from_micros(1)).await;
        }
    }

    /// Wait for NAND with timeout (returns false if timeout)
    pub async fn wait_ready_timeout(&mut self, timeout_us: u32) -> bool {
        let mut elapsed = 0u32;
        while self.pins.rb.is_low() {
            if elapsed >= timeout_us {
                return false;
            }
            Timer::after(Duration::from_micros(10)).await;
            elapsed += 10;
        }
        true
    }

    /// Set data bus as output and write a byte
    fn set_data_output(&mut self, data: u8) {
        self.pins.d0.set_as_output();
        self.pins.d1.set_as_output();
        self.pins.d2.set_as_output();
        self.pins.d3.set_as_output();
        self.pins.d4.set_as_output();
        self.pins.d5.set_as_output();
        self.pins.d6.set_as_output();
        self.pins.d7.set_as_output();
        
        self.pins.d0.set_level(if data & 0x01 != 0 { Level::High } else { Level::Low });
        self.pins.d1.set_level(if data & 0x02 != 0 { Level::High } else { Level::Low });
        self.pins.d2.set_level(if data & 0x04 != 0 { Level::High } else { Level::Low });
        self.pins.d3.set_level(if data & 0x08 != 0 { Level::High } else { Level::Low });
        self.pins.d4.set_level(if data & 0x10 != 0 { Level::High } else { Level::Low });
        self.pins.d5.set_level(if data & 0x20 != 0 { Level::High } else { Level::Low });
        self.pins.d6.set_level(if data & 0x40 != 0 { Level::High } else { Level::Low });
        self.pins.d7.set_level(if data & 0x80 != 0 { Level::High } else { Level::Low });
    }

    /// Set data bus as input and read a byte
    fn set_data_input(&mut self) -> u8 {
        self.pins.d0.set_as_input();
        self.pins.d1.set_as_input();
        self.pins.d2.set_as_input();
        self.pins.d3.set_as_input();
        self.pins.d4.set_as_input();
        self.pins.d5.set_as_input();
        self.pins.d6.set_as_input();
        self.pins.d7.set_as_input();
        
        let mut data = 0u8;
        if self.pins.d0.is_high() { data |= 0x01; }
        if self.pins.d1.is_high() { data |= 0x02; }
        if self.pins.d2.is_high() { data |= 0x04; }
        if self.pins.d3.is_high() { data |= 0x08; }
        if self.pins.d4.is_high() { data |= 0x10; }
        if self.pins.d5.is_high() { data |= 0x20; }
        if self.pins.d6.is_high() { data |= 0x40; }
        if self.pins.d7.is_high() { data |= 0x80; }
        data
    }

    /// Delay for timing (uses busy-wait for short delays)
    #[inline(always)]
    fn delay_ns(&self, ns: u32) {
        // At 125MHz, each cycle is 8ns
        // For short delays, use cortex_m::asm::nop()
        let cycles = ns / 8;
        for _ in 0..cycles {
            cortex_m::asm::nop();
        }
    }

    /// Send a command byte to NAND
    pub async fn send_command(&mut self, cmd: u8) {
        self.pins.ce.set_low();
        self.pins.cle.set_high();
        self.pins.ale.set_low();
        self.delay_ns(self.timing.t_cls);
        
        self.set_data_output(cmd);
        
        self.pins.we.set_low();
        self.delay_ns(self.timing.t_wp);
        self.pins.we.set_high();
        self.delay_ns(self.timing.t_wh);
        
        self.pins.cle.set_low();
        self.delay_ns(self.timing.t_clh);
    }

    /// Send an address byte to NAND
    pub async fn send_address(&mut self, addr: u8) {
        self.pins.ce.set_low();
        self.pins.cle.set_low();
        self.pins.ale.set_high();
        self.delay_ns(self.timing.t_als);
        
        self.set_data_output(addr);
        
        self.pins.we.set_low();
        self.delay_ns(self.timing.t_wp);
        self.pins.we.set_high();
        self.delay_ns(self.timing.t_wh);
        
        self.pins.ale.set_low();
        self.delay_ns(self.timing.t_alh);
    }

    /// Read a single byte from data bus
    pub fn read_byte(&mut self) -> u8 {
        self.pins.ce.set_low();
        self.pins.cle.set_low();
        self.pins.ale.set_low();
        
        self.pins.re.set_low();
        self.delay_ns(self.timing.t_rp);
        let data = self.set_data_input();
        self.pins.re.set_high();
        self.delay_ns(self.timing.t_reh);
        
        data
    }

    /// Read multiple bytes into a buffer
    pub fn read_data(&mut self, buffer: &mut [u8]) {
        for byte in buffer.iter_mut() {
            *byte = self.read_byte();
        }
    }

    /// Write a single byte to data bus
    pub fn write_byte(&mut self, data: u8) {
        self.pins.ce.set_low();
        self.pins.cle.set_low();
        self.pins.ale.set_low();
        
        self.set_data_output(data);
        
        self.pins.we.set_low();
        self.delay_ns(self.timing.t_wp);
        self.pins.we.set_high();
        self.delay_ns(self.timing.t_wh);
    }

    /// Write multiple bytes from a buffer
    pub fn write_data(&mut self, data: &[u8]) {
        for &byte in data {
            self.write_byte(byte);
        }
    }

    /// Read NAND chip ID (5 bytes typically)
    pub async fn read_id(&mut self) -> [u8; 5] {
        let mut id = [0u8; 5];
        
        self.send_command(commands::READ_ID).await;
        self.send_address(0x00).await;
        
        // Small delay before reading
        self.delay_ns(100);
        
        self.read_data(&mut id);
        
        self.pins.ce.set_high();
        id
    }

    /// Reset the NAND chip
    pub async fn reset(&mut self) {
        self.send_command(commands::RESET).await;
        self.pins.ce.set_high();
        
        // Wait for reset to complete (typically < 5ms)
        Timer::after(Duration::from_millis(5)).await;
        self.wait_ready().await;
    }

    /// Read status register
    pub async fn read_status(&mut self) -> u8 {
        self.send_command(commands::READ_STATUS).await;
        self.delay_ns(100);
        let status = self.read_byte();
        self.pins.ce.set_high();
        status
    }

    /// Read a page of data (large page NAND: 2KB+ pages)
    pub async fn read_page(&mut self, page_addr: u32, buffer: &mut [u8]) {
        // Send READ command (0x00)
        self.send_command(commands::READ_1ST).await;
        
        // Send column address (2 bytes for large page)
        self.send_address(0x00).await;
        self.send_address(0x00).await;
        
        // Send row/page address (3 bytes for large capacity)
        self.send_address((page_addr & 0xFF) as u8).await;
        self.send_address(((page_addr >> 8) & 0xFF) as u8).await;
        self.send_address(((page_addr >> 16) & 0xFF) as u8).await;
        
        // Send READ confirm command (0x30)
        self.send_command(commands::READ_2ND).await;
        
        // Wait for data to be ready (tR typically 25-100us)
        self.wait_ready().await;
        
        // Read page data
        self.read_data(buffer);
        
        self.pins.ce.set_high();
    }

    /// Read a page with OOB/spare area
    pub async fn read_page_with_oob(
        &mut self, 
        page_addr: u32, 
        data: &mut [u8], 
        oob: &mut [u8]
    ) {
        self.send_command(commands::READ_1ST).await;
        
        // Column 0
        self.send_address(0x00).await;
        self.send_address(0x00).await;
        
        // Row address
        self.send_address((page_addr & 0xFF) as u8).await;
        self.send_address(((page_addr >> 8) & 0xFF) as u8).await;
        self.send_address(((page_addr >> 16) & 0xFF) as u8).await;
        
        self.send_command(commands::READ_2ND).await;
        self.wait_ready().await;
        
        // Read data area
        self.read_data(data);
        
        // Read OOB area
        self.read_data(oob);
        
        self.pins.ce.set_high();
    }

    /// Program (write) a page
    pub async fn program_page(&mut self, page_addr: u32, data: &[u8]) -> bool {
        // Send PROGRAM command
        self.send_command(commands::PAGE_PROGRAM_1ST).await;
        
        // Column address
        self.send_address(0x00).await;
        self.send_address(0x00).await;
        
        // Row address
        self.send_address((page_addr & 0xFF) as u8).await;
        self.send_address(((page_addr >> 8) & 0xFF) as u8).await;
        self.send_address(((page_addr >> 16) & 0xFF) as u8).await;
        
        // Write data
        self.write_data(data);
        
        // Confirm program
        self.send_command(commands::PAGE_PROGRAM_2ND).await;
        
        // Wait for program to complete (tPROG typically 200-700us)
        self.wait_ready().await;
        
        // Check status
        let status = self.read_status().await;
        
        // Bit 0 = 0 means success
        (status & 0x01) == 0
    }

    /// Erase a block
    pub async fn erase_block(&mut self, block_addr: u32) -> bool {
        // Calculate page address from block (assuming 64 pages/block)
        let page_addr = block_addr * 64;
        
        // Send ERASE command
        self.send_command(commands::BLOCK_ERASE_1ST).await;
        
        // Row address only (3 bytes)
        self.send_address((page_addr & 0xFF) as u8).await;
        self.send_address(((page_addr >> 8) & 0xFF) as u8).await;
        self.send_address(((page_addr >> 16) & 0xFF) as u8).await;
        
        // Confirm erase
        self.send_command(commands::BLOCK_ERASE_2ND).await;
        
        // Wait for erase to complete (tBERS typically 2-5ms)
        self.wait_ready().await;
        
        // Check status
        let status = self.read_status().await;
        
        (status & 0x01) == 0
    }

    /// Check if block is bad (reads spare area marker)
    pub async fn is_bad_block(&mut self, block_addr: u32) -> bool {
        let page_addr = block_addr * 64; // First page of block
        
        // Read first byte of spare area
        self.send_command(commands::READ_1ST).await;
        
        // Column = page_size (start of spare area, typically 2048)
        self.send_address(0x00).await;
        self.send_address(0x08).await; // 0x0800 = 2048
        
        // Row address
        self.send_address((page_addr & 0xFF) as u8).await;
        self.send_address(((page_addr >> 8) & 0xFF) as u8).await;
        self.send_address(((page_addr >> 16) & 0xFF) as u8).await;
        
        self.send_command(commands::READ_2ND).await;
        self.wait_ready().await;
        
        let marker = self.read_byte();
        self.pins.ce.set_high();
        
        // 0xFF = good block, anything else = bad
        marker != 0xFF
    }
}

/// NAND command constants
pub mod commands {
    pub const READ_1ST: u8 = 0x00;
    pub const READ_2ND: u8 = 0x30;
    pub const READ_ID: u8 = 0x90;
    pub const RESET: u8 = 0xFF;
    pub const PAGE_PROGRAM_1ST: u8 = 0x80;
    pub const PAGE_PROGRAM_2ND: u8 = 0x10;
    pub const BLOCK_ERASE_1ST: u8 = 0x60;
    pub const BLOCK_ERASE_2ND: u8 = 0xD0;
    pub const READ_STATUS: u8 = 0x70;
    pub const READ_PARAMETER_PAGE: u8 = 0xEC;
    pub const SET_FEATURES: u8 = 0xEF;
    pub const GET_FEATURES: u8 = 0xEE;
}

/// Status register bits
pub mod status {
    pub const FAIL: u8 = 0x01;        // Operation failed
    pub const READY: u8 = 0x40;       // Device ready
    pub const WRITE_PROTECT: u8 = 0x80; // Write protected
}
