//! OpenFlash RP2040 Firmware
//! Minimal firmware for NAND flash operations via USB
//! Supports both parallel NAND and SPI NAND interfaces

#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_rp::bind_interrupts;
use embassy_rp::gpio::{Input, Output, Level, Pull, Flex};
use embassy_rp::peripherals::USB;
use embassy_rp::usb::{Driver, InterruptHandler};
use embassy_rp::spi::{Spi, Config as SpiConfig};
use embassy_usb::class::cdc_acm::{CdcAcmClass, State};
use embassy_usb::{Builder, Config};
use {defmt_rtt as _, panic_probe as _};

mod pio_nand;
mod spi_nand;
mod usb_handler;

use pio_nand::{NandController, NandPins};
use spi_nand::SpiNandController;
use usb_handler::UsbHandler;

bind_interrupts!(struct Irqs {
    USBCTRL_IRQ => InterruptHandler<USB>;
});

// USB descriptors storage
static mut DEVICE_DESCRIPTOR: [u8; 256] = [0; 256];
static mut CONFIG_DESCRIPTOR: [u8; 256] = [0; 256];
static mut BOS_DESCRIPTOR: [u8; 256] = [0; 256];
static mut CONTROL_BUF: [u8; 64] = [0; 64];
static mut STATE: Option<State> = None;

/// Pin assignments for NAND interface on Raspberry Pi Pico
/// 
/// === Parallel NAND Mode ===
/// Control signals:
///   GP0  - CLE (Command Latch Enable)
///   GP1  - ALE (Address Latch Enable)
///   GP2  - WE# (Write Enable, active low)
///   GP3  - RE# (Read Enable, active low)
///   GP4  - CE# (Chip Enable, active low)
///   GP5  - R/B# (Ready/Busy, active low = busy)
///
/// Data bus:
///   GP6  - D0
///   GP7  - D1
///   GP8  - D2
///   GP9  - D3
///   GP10 - D4
///   GP11 - D5
///   GP12 - D6
///   GP13 - D7
///
/// === SPI NAND Mode ===
/// SPI signals (directly on SPI0):
///   GP16 - SPI0 RX (MISO)
///   GP17 - SPI0 CS# (directly controlled)
///   GP18 - SPI0 SCK
///   GP19 - SPI0 TX (MOSI)
///
/// Optional (shared):
///   GP14 - WP# (Write Protect, active low)
///   GP25 - LED (onboard)

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());
    info!("OpenFlash RP2040 Firmware v0.1.0");

    // Initialize LED for status indication
    let mut led = Output::new(p.PIN_25, Level::Low);
    led.set_high(); // LED on during init

    // Initialize NAND control pins
    let nand_pins = NandPins {
        cle: Output::new(p.PIN_0, Level::Low),
        ale: Output::new(p.PIN_1, Level::Low),
        we: Output::new(p.PIN_2, Level::High),   // Active low, idle high
        re: Output::new(p.PIN_3, Level::High),   // Active low, idle high
        ce: Output::new(p.PIN_4, Level::High),   // Active low, idle high (disabled)
        rb: Input::new(p.PIN_5, Pull::Up),       // Ready/Busy with pull-up
        
        // Data bus as flexible I/O
        d0: Flex::new(p.PIN_6),
        d1: Flex::new(p.PIN_7),
        d2: Flex::new(p.PIN_8),
        d3: Flex::new(p.PIN_9),
        d4: Flex::new(p.PIN_10),
        d5: Flex::new(p.PIN_11),
        d6: Flex::new(p.PIN_12),
        d7: Flex::new(p.PIN_13),
    };

    // Create NAND controller
    let nand = NandController::new(nand_pins);
    info!("Parallel NAND controller initialized");

    // Initialize SPI NAND controller
    // SPI0 pins: GP16=RX, GP17=CS, GP18=SCK, GP19=TX
    let mut spi_config = SpiConfig::default();
    spi_config.frequency = 40_000_000; // 40 MHz initial speed
    
    let spi = Spi::new_blocking(
        p.SPI0,
        p.PIN_18, // SCK
        p.PIN_19, // MOSI
        p.PIN_16, // MISO
        spi_config,
    );
    let spi_cs = Output::new(p.PIN_17, Level::High); // CS# idle high
    let spi_nand = SpiNandController::new(spi, spi_cs);
    info!("SPI NAND controller initialized");

    // Create USB driver
    let driver = Driver::new(p.USB, Irqs);

    // USB configuration
    let mut config = Config::new(0xC0DE, 0xCAFE);
    config.manufacturer = Some("OpenFlash");
    config.product = Some("OpenFlash NAND Programmer");
    config.serial_number = Some("OF-RP2040-001");
    config.max_power = 250; // 500mA
    config.max_packet_size_0 = 64;
    config.composite_with_iads = true;

    // Safety: These statics are only accessed from this task
    let (device_descriptor, config_descriptor, bos_descriptor, control_buf, state) = unsafe {
        STATE = Some(State::new());
        (
            &mut DEVICE_DESCRIPTOR,
            &mut CONFIG_DESCRIPTOR,
            &mut BOS_DESCRIPTOR,
            &mut CONTROL_BUF,
            STATE.as_mut().unwrap(),
        )
    };

    let mut builder = Builder::new(
        driver,
        config,
        device_descriptor,
        config_descriptor,
        bos_descriptor,
        control_buf,
    );

    // Create CDC ACM class for serial communication
    let class = CdcAcmClass::new(&mut builder, state, 64);

    // Build USB device
    let usb = builder.build();

    // Spawn USB task
    spawner.spawn(usb_task(usb)).unwrap();

    info!("USB initialized, waiting for connection...");
    led.set_low(); // LED off, ready

    // Create command handler with NAND controller
    let mut handler = UsbHandler::new(class, nand);

    // Main loop
    loop {
        // Wait for USB connection
        handler.class.wait_connection().await;
        info!("Host connected");
        led.set_high();

        // Handle commands until disconnection
        handler.handle_commands().await;
        
        info!("Host disconnected");
        led.set_low();
    }
}

#[embassy_executor::task]
async fn usb_task(mut usb: embassy_usb::UsbDevice<'static, Driver<'static, USB>>) -> ! {
    usb.run().await
}
