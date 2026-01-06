//! OpenFlash STM32F1 Firmware
//! Minimal firmware for NAND flash operations via USB

#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_stm32::bind_interrupts;
use embassy_stm32::peripherals::USB;
use embassy_stm32::usb::{Driver, InterruptHandler};
use embassy_usb::class::cdc_acm::{CdcAcmClass, State};
use embassy_usb::{Builder, Config};
use {defmt_rtt as _, panic_probe as _};

mod usb_handler;

use usb_handler::UsbHandler;

bind_interrupts!(struct Irqs {
    USB_LP_CAN1_RX0 => InterruptHandler<USB>;
});

static mut DEVICE_DESCRIPTOR: [u8; 256] = [0; 256];
static mut CONFIG_DESCRIPTOR: [u8; 256] = [0; 256];
static mut BOS_DESCRIPTOR: [u8; 256] = [0; 256];
static mut CONTROL_BUF: [u8; 64] = [0; 64];
static mut STATE: Option<State> = None;

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_stm32::init(Default::default());
    info!("OpenFlash STM32F1 Firmware v0.1.0");

    let driver = Driver::new(p.USB, Irqs, p.PA12, p.PA11);

    let mut config = Config::new(0xC0DE, 0xCAFE);
    config.manufacturer = Some("OpenFlash");
    config.product = Some("OpenFlash NAND Programmer");
    config.serial_number = Some("OF-STM32-001");
    config.max_power = 250;
    config.max_packet_size_0 = 64;
    config.composite_with_iads = true;

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

    let class = CdcAcmClass::new(&mut builder, state, 64);
    let usb = builder.build();

    spawner.spawn(usb_task(usb)).unwrap();

    info!("USB initialized");

    let mut handler = UsbHandler::new(class);

    loop {
        handler.class.wait_connection().await;
        info!("Host connected");
        handler.handle_commands().await;
        info!("Host disconnected");
    }
}

#[embassy_executor::task]
async fn usb_task(mut usb: embassy_usb::UsbDevice<'static, Driver<'static, USB>>) -> ! {
    usb.run().await
}
