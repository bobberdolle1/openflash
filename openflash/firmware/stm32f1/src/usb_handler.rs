//! USB command handler for OpenFlash STM32F1 firmware

use defmt::*;
use embassy_time::Timer;
use embassy_usb::class::cdc_acm::CdcAcmClass;
use embassy_usb::driver::Driver;

const MAX_PAGE_SIZE: usize = 4352;
const PACKET_SIZE: usize = 64;

#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub enum Command {
    Ping = 0x01,
    BusConfig = 0x02,
    NandCmd = 0x03,
    NandAddr = 0x04,
    NandReadPage = 0x05,
    NandWritePage = 0x06,
    ReadId = 0x07,
    Reset = 0x08,
}

impl Command {
    fn from_u8(val: u8) -> Option<Self> {
        match val {
            0x01 => Some(Command::Ping),
            0x02 => Some(Command::BusConfig),
            0x03 => Some(Command::NandCmd),
            0x04 => Some(Command::NandAddr),
            0x05 => Some(Command::NandReadPage),
            0x06 => Some(Command::NandWritePage),
            0x07 => Some(Command::ReadId),
            0x08 => Some(Command::Reset),
            _ => None,
        }
    }
}

#[repr(u8)]
pub enum Status {
    Ok = 0x00,
    Error = 0x01,
    UnknownCommand = 0xFF,
}

pub struct UsbHandler<'d, D: Driver<'d>> {
    pub class: CdcAcmClass<'d, D>,
    page_buffer: [u8; MAX_PAGE_SIZE],
}

impl<'d, D: Driver<'d>> UsbHandler<'d, D> {
    pub fn new(class: CdcAcmClass<'d, D>) -> Self {
        Self {
            class,
            page_buffer: [0xFF; MAX_PAGE_SIZE],
        }
    }

    pub async fn handle_commands(&mut self) {
        let mut cmd_buf = [0u8; PACKET_SIZE];
        
        loop {
            match self.class.read_packet(&mut cmd_buf).await {
                Ok(n) if n > 0 => {
                    self.process_command(&cmd_buf[..n]).await;
                }
                Ok(_) => {}
                Err(_) => {
                    warn!("USB connection lost");
                    break;
                }
            }
        }
    }

    async fn process_command(&mut self, cmd_data: &[u8]) {
        if cmd_data.is_empty() {
            return;
        }

        let cmd_byte = cmd_data[0];
        let args = if cmd_data.len() > 1 { &cmd_data[1..] } else { &[] };

        match Command::from_u8(cmd_byte) {
            Some(Command::Ping) => self.handle_ping().await,
            Some(Command::BusConfig) => self.handle_bus_config(args).await,
            Some(Command::NandCmd) => self.handle_nand_cmd(args).await,
            Some(Command::NandAddr) => self.handle_nand_addr(args).await,
            Some(Command::NandReadPage) => self.handle_read_page(args).await,
            Some(Command::NandWritePage) => self.handle_write_page(args).await,
            Some(Command::ReadId) => self.handle_read_id().await,
            Some(Command::Reset) => self.handle_reset().await,
            None => {
                warn!("Unknown command: 0x{:02X}", cmd_byte);
                self.send_response(&[Status::UnknownCommand as u8]).await;
            }
        }
    }

    async fn handle_ping(&mut self) {
        info!("PING");
        self.send_response(&[Command::Ping as u8, Status::Ok as u8]).await;
    }

    async fn handle_bus_config(&mut self, args: &[u8]) {
        if args.len() >= 4 {
            info!("BUS_CONFIG");
            self.send_response(&[Command::BusConfig as u8, Status::Ok as u8]).await;
        } else {
            self.send_response(&[Command::BusConfig as u8, Status::Error as u8]).await;
        }
    }

    async fn handle_nand_cmd(&mut self, args: &[u8]) {
        if !args.is_empty() {
            info!("NAND_CMD: 0x{:02X}", args[0]);
            self.send_response(&[Command::NandCmd as u8, Status::Ok as u8]).await;
        } else {
            self.send_response(&[Command::NandCmd as u8, Status::Error as u8]).await;
        }
    }

    async fn handle_nand_addr(&mut self, args: &[u8]) {
        if !args.is_empty() {
            info!("NAND_ADDR: 0x{:02X}", args[0]);
            self.send_response(&[Command::NandAddr as u8, Status::Ok as u8]).await;
        } else {
            self.send_response(&[Command::NandAddr as u8, Status::Error as u8]).await;
        }
    }

    async fn handle_read_page(&mut self, args: &[u8]) {
        if args.len() >= 6 {
            let page_addr = u32::from_le_bytes([args[0], args[1], args[2], args[3]]);
            let page_size = u16::from_le_bytes([args[4], args[5]]) as usize;
            let size = page_size.min(MAX_PAGE_SIZE);
            
            info!("READ_PAGE: addr={}, size={}", page_addr, size);
            
            for i in 0..size {
                self.page_buffer[i] = 0xFF;
            }
            
            self.send_data_chunked(size).await;
        } else {
            self.send_response(&[Command::NandReadPage as u8, Status::Error as u8]).await;
        }
    }

    async fn handle_write_page(&mut self, args: &[u8]) {
        if args.len() >= 6 {
            let page_addr = u32::from_le_bytes([args[0], args[1], args[2], args[3]]);
            let page_size = u16::from_le_bytes([args[4], args[5]]) as usize;
            let size = page_size.min(MAX_PAGE_SIZE);
            
            info!("WRITE_PAGE: addr={}, size={}", page_addr, size);
            
            if self.receive_data_chunked(size).await {
                self.send_response(&[Command::NandWritePage as u8, Status::Ok as u8]).await;
            } else {
                self.send_response(&[Command::NandWritePage as u8, Status::Error as u8]).await;
            }
        } else {
            self.send_response(&[Command::NandWritePage as u8, Status::Error as u8]).await;
        }
    }

    async fn handle_read_id(&mut self) {
        info!("READ_ID");
        let response = [Command::ReadId as u8, Status::Ok as u8, 0xEC, 0xD7, 0x10, 0x95, 0x44];
        self.send_response(&response).await;
    }

    async fn handle_reset(&mut self) {
        info!("RESET");
        self.send_response(&[Command::Reset as u8, Status::Ok as u8]).await;
    }

    async fn send_response(&mut self, data: &[u8]) {
        let _ = self.class.write_packet(data).await;
    }

    async fn send_data_chunked(&mut self, size: usize) {
        let mut offset = 0;
        while offset < size {
            let chunk_size = (size - offset).min(PACKET_SIZE);
            let _ = self.class.write_packet(&self.page_buffer[offset..offset + chunk_size]).await;
            offset += chunk_size;
            Timer::after_micros(50).await;
        }
    }

    async fn receive_data_chunked(&mut self, size: usize) -> bool {
        let mut offset = 0;
        let mut chunk_buf = [0u8; PACKET_SIZE];
        
        while offset < size {
            match self.class.read_packet(&mut chunk_buf).await {
                Ok(n) => {
                    let copy_size = n.min(size - offset);
                    self.page_buffer[offset..offset + copy_size].copy_from_slice(&chunk_buf[..copy_size]);
                    offset += copy_size;
                }
                Err(_) => return false,
            }
        }
        true
    }
}
