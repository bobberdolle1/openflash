//! GPIO-based NAND Flash interface for STM32F1
//! Uses bit-banging for NAND bus control

use embassy_stm32::gpio::{Level, Output, Input, Pull, Flex, Speed};
use embassy_time::{Duration, Timer};

/// NAND Flash timing parameters (in nanoseconds)
#[derive(Clone, Copy)]
pub struct NandTiming {
    pub t_wp: u32,   // WE# pulse width
    pub t_rp: u32,   // RE# pulse width
    pub t_cls: u32,  // CLE setup time
    pub t_als: u32,  // ALE setup time
    pub t_clh: u32,  // CLE hold time
    pub t_alh: u32,  // ALE hold time
}

impl Default for NandTiming {
    fn default() -> Self {
        Self {
            t_wp: 50,
            t_rp: 50,
            t_cls: 50,
            t_als: 50,
            t_clh: 20,
            t_alh: 20,
        }
    }
}

/// Pin configuration for NAND interface on STM32F103 "Blue Pill"
/// 
/// Control signals (directly controlled):
///   PA0 - CLE (Command Latch Enable)
///   PA1 - ALE (Address Latch Enable)
///   PA2 - WE# (Write Enable, active low)
///   PA3 - RE# (Read Enable, active low)
///   PA4 - CE# (Chip Enable, active low)
///   PA5 - R/B# (Ready/Busy input)
///
/// Data bus (directly controlled):
///   PB0-PB7 - D0-D7
///
/// Status LED:
///   PC13 - Onboard LED (active low on Blue Pill)

pub struct NandPins<'d> {
    pub cle: Output<'d>,
    pub ale: Output<'d>,
    pub we: Output<'d>,
    pub re: Output<'d>,
    pub ce: Output<'d>,
    pub rb: Input<'d>,
    
    pub d0: Flex<'d>,
    pub d1: Flex<'d>,
    pub d2: Flex<'d>,
    pub d3: Flex<'d>,
    pub d4: Flex<'d>,
    pub d5: Flex<'d>,
    pub d6: Flex<'d>,
    pub d7: Flex<'d>,
}

pub struct NandController<'d> {
    pins: NandPins<'d>,
    timing: NandTiming,
}

impl<'d> NandController<'d> {
    pub fn new(mut pins: NandPins<'d>) -> Self {
        // Initialize control signals
        pins.ce.set_high();
        pins.we.set_high();
        pins.re.set_high();
        pins.cle.set_low();
        pins.ale.set_low();
        
        // Data bus as input
        pins.d0.set_as_input(Pull::None);
        pins.d1.set_as_input(Pull::None);
        pins.d2.set_as_input(Pull::None);
        pins.d3.set_as_input(Pull::None);
        pins.d4.set_as_input(Pull::None);
        pins.d5.set_as_input(Pull::None);
        pins.d6.set_as_input(Pull::None);
        pins.d7.set_as_input(Pull::None);
        
        Self {
            pins,
            timing: NandTiming::default(),
        }
    }

    pub fn set_timing(&mut self, timing: NandTiming) {
        self.timing = timing;
    }

    pub async fn wait_ready(&mut self) {
        while self.pins.rb.is_low() {
            Timer::after(Duration::from_micros(1)).await;
        }
    }

    fn set_data_output(&mut self, data: u8) {
        self.pins.d0.set_as_output(Speed::High);
        self.pins.d1.set_as_output(Speed::High);
        self.pins.d2.set_as_output(Speed::High);
        self.pins.d3.set_as_output(Speed::High);
        self.pins.d4.set_as_output(Speed::High);
        self.pins.d5.set_as_output(Speed::High);
        self.pins.d6.set_as_output(Speed::High);
        self.pins.d7.set_as_output(Speed::High);
        
        self.pins.d0.set_level(if data & 0x01 != 0 { Level::High } else { Level::Low });
        self.pins.d1.set_level(if data & 0x02 != 0 { Level::High } else { Level::Low });
        self.pins.d2.set_level(if data & 0x04 != 0 { Level::High } else { Level::Low });
        self.pins.d3.set_level(if data & 0x08 != 0 { Level::High } else { Level::Low });
        self.pins.d4.set_level(if data & 0x10 != 0 { Level::High } else { Level::Low });
        self.pins.d5.set_level(if data & 0x20 != 0 { Level::High } else { Level::Low });
        self.pins.d6.set_level(if data & 0x40 != 0 { Level::High } else { Level::Low });
        self.pins.d7.set_level(if data & 0x80 != 0 { Level::High } else { Level::Low });
    }

    fn set_data_input(&mut self) -> u8 {
        self.pins.d0.set_as_input(Pull::None);
        self.pins.d1.set_as_input(Pull::None);
        self.pins.d2.set_as_input(Pull::None);
        self.pins.d3.set_as_input(Pull::None);
        self.pins.d4.set_as_input(Pull::None);
        self.pins.d5.set_as_input(Pull::None);
        self.pins.d6.set_as_input(Pull::None);
        self.pins.d7.set_as_input(Pull::None);
        
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

    #[inline(always)]
    fn delay_ns(&self, ns: u32) {
        // At 72MHz, each cycle is ~14ns
        let cycles = ns / 14;
        for _ in 0..cycles {
            cortex_m::asm::nop();
        }
    }

    pub async fn send_command(&mut self, cmd: u8) {
        self.pins.ce.set_low();
        self.pins.cle.set_high();
        self.pins.ale.set_low();
        self.delay_ns(self.timing.t_cls);
        
        self.set_data_output(cmd);
        
        self.pins.we.set_low();
        self.delay_ns(self.timing.t_wp);
        self.pins.we.set_high();
        
        self.pins.cle.set_low();
        self.delay_ns(self.timing.t_clh);
    }

    pub async fn send_address(&mut self, addr: u8) {
        self.pins.ce.set_low();
        self.pins.cle.set_low();
        self.pins.ale.set_high();
        self.delay_ns(self.timing.t_als);
        
        self.set_data_output(addr);
        
        self.pins.we.set_low();
        self.delay_ns(self.timing.t_wp);
        self.pins.we.set_high();
        
        self.pins.ale.set_low();
        self.delay_ns(self.timing.t_alh);
    }

    pub fn read_byte(&mut self) -> u8 {
        self.pins.ce.set_low();
        self.pins.cle.set_low();
        self.pins.ale.set_low();
        
        self.pins.re.set_low();
        self.delay_ns(self.timing.t_rp);
        let data = self.set_data_input();
        self.pins.re.set_high();
        
        data
    }

    pub fn read_data(&mut self, buffer: &mut [u8]) {
        for byte in buffer.iter_mut() {
            *byte = self.read_byte();
        }
    }

    pub fn write_byte(&mut self, data: u8) {
        self.pins.ce.set_low();
        self.pins.cle.set_low();
        self.pins.ale.set_low();
        
        self.set_data_output(data);
        
        self.pins.we.set_low();
        self.delay_ns(self.timing.t_wp);
        self.pins.we.set_high();
    }

    pub fn write_data(&mut self, data: &[u8]) {
        for &byte in data {
            self.write_byte(byte);
        }
    }

    pub async fn read_id(&mut self) -> [u8; 5] {
        let mut id = [0u8; 5];
        
        self.send_command(0x90).await;
        self.send_address(0x00).await;
        self.delay_ns(100);
        self.read_data(&mut id);
        
        self.pins.ce.set_high();
        id
    }

    pub async fn reset(&mut self) {
        self.send_command(0xFF).await;
        self.pins.ce.set_high();
        Timer::after(Duration::from_millis(5)).await;
        self.wait_ready().await;
    }

    pub async fn read_page(&mut self, page_addr: u32, buffer: &mut [u8]) {
        self.send_command(0x00).await;
        
        self.send_address(0x00).await;
        self.send_address(0x00).await;
        self.send_address((page_addr & 0xFF) as u8).await;
        self.send_address(((page_addr >> 8) & 0xFF) as u8).await;
        self.send_address(((page_addr >> 16) & 0xFF) as u8).await;
        
        self.send_command(0x30).await;
        self.wait_ready().await;
        self.read_data(buffer);
        
        self.pins.ce.set_high();
    }

    pub async fn program_page(&mut self, page_addr: u32, data: &[u8]) -> bool {
        self.send_command(0x80).await;
        
        self.send_address(0x00).await;
        self.send_address(0x00).await;
        self.send_address((page_addr & 0xFF) as u8).await;
        self.send_address(((page_addr >> 8) & 0xFF) as u8).await;
        self.send_address(((page_addr >> 16) & 0xFF) as u8).await;
        
        self.write_data(data);
        self.send_command(0x10).await;
        self.wait_ready().await;
        
        // Read status
        self.send_command(0x70).await;
        let status = self.read_byte();
        self.pins.ce.set_high();
        
        (status & 0x01) == 0
    }
}
