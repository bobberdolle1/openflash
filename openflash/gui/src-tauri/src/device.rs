//! USB device management for OpenFlash

use nusb::transfer::RequestBuffer;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex as TokioMutex;

use openflash_core::protocol::{Command, Packet};

const VENDOR_ID: u16 = 0xC0DE;
const PRODUCT_ID: u16 = 0xCAFE;
const EP_OUT: u8 = 0x01;
const EP_IN: u8 = 0x81;

/// Flash interface type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum FlashInterface {
    #[default]
    ParallelNand,
    SpiNand,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub id: String,
    pub name: String,
    pub serial: Option<String>,
    pub connected: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChipInfo {
    pub manufacturer: String,
    pub model: String,
    pub chip_id: Vec<u8>,
    pub size_mb: u32,
    pub page_size: u32,
    pub block_size: u32,
    pub interface: FlashInterface,
}

pub struct UsbDevice {
    interface: nusb::Interface,
}

impl UsbDevice {
    pub async fn send_command(&self, cmd: Command, args: &[u8]) -> Result<Vec<u8>, String> {
        let packet = Packet::new(cmd, args);
        let data = packet.to_bytes();

        // Send command
        self.interface
            .bulk_out(EP_OUT, data.to_vec())
            .await
            .status
            .map_err(|e| format!("USB write error: {:?}", e))?;

        // Receive response
        let buf = RequestBuffer::new(64);
        let result = self.interface.bulk_in(EP_IN, buf).await;
        
        result.status.map_err(|e| format!("USB read error: {:?}", e))?;
        Ok(result.data)
    }

    pub async fn read_page(&self, page_addr: u32, page_size: u16) -> Result<Vec<u8>, String> {
        let mut args = [0u8; 6];
        args[0..4].copy_from_slice(&page_addr.to_le_bytes());
        args[4..6].copy_from_slice(&page_size.to_le_bytes());

        self.send_command(Command::NandReadPage, &args).await?;

        let mut data = Vec::with_capacity(page_size as usize);
        while data.len() < page_size as usize {
            let buf = RequestBuffer::new(64);
            let result = self.interface.bulk_in(EP_IN, buf).await;
            result.status.map_err(|e| format!("USB read error: {:?}", e))?;
            
            let remaining = page_size as usize - data.len();
            let to_copy = remaining.min(result.data.len());
            data.extend_from_slice(&result.data[..to_copy]);
        }

        Ok(data)
    }
}

pub struct DeviceManager {
    devices: Vec<DeviceInfo>,
    active_device: Option<Arc<TokioMutex<UsbDevice>>>,
    interface: FlashInterface,
}

impl DeviceManager {
    pub fn new() -> Self {
        Self {
            devices: Vec::new(),
            active_device: None,
            interface: FlashInterface::ParallelNand,
        }
    }
    
    pub fn set_interface(&mut self, interface: FlashInterface) {
        self.interface = interface;
    }
    
    pub fn get_interface(&self) -> FlashInterface {
        self.interface
    }

    pub fn scan_devices(&mut self) -> Vec<DeviceInfo> {
        self.devices.clear();

        if let Ok(devices) = nusb::list_devices() {
            for dev_info in devices {
                if dev_info.vendor_id() == VENDOR_ID && dev_info.product_id() == PRODUCT_ID {
                    let id = format!("{:04x}:{:04x}:{}", 
                        dev_info.vendor_id(), 
                        dev_info.product_id(),
                        dev_info.bus_number()
                    );

                    let name = dev_info.product_string()
                        .unwrap_or("OpenFlash Device")
                        .to_string();

                    let serial = dev_info.serial_number().map(|s| s.to_string());

                    self.devices.push(DeviceInfo {
                        id,
                        name,
                        serial,
                        connected: false,
                    });
                }
            }
        }

        self.devices.clone()
    }

    pub fn list_devices(&self) -> Vec<DeviceInfo> {
        self.devices.clone()
    }

    pub fn connect(&mut self, device_id: &str) -> Result<(), String> {
        let devices = nusb::list_devices()
            .map_err(|e| format!("Failed to list devices: {}", e))?;

        for dev_info in devices {
            let id = format!("{:04x}:{:04x}:{}", 
                dev_info.vendor_id(), 
                dev_info.product_id(),
                dev_info.bus_number()
            );

            if id == device_id {
                let device = dev_info.open()
                    .map_err(|e| format!("Failed to open device: {}", e))?;

                let interface = device.claim_interface(0)
                    .map_err(|e| format!("Failed to claim interface: {}", e))?;

                self.active_device = Some(Arc::new(TokioMutex::new(UsbDevice { interface })));

                for dev in &mut self.devices {
                    if dev.id == device_id {
                        dev.connected = true;
                    }
                }

                return Ok(());
            }
        }

        Err("Device not found".to_string())
    }

    pub fn disconnect(&mut self) {
        self.active_device = None;
        for dev in &mut self.devices {
            dev.connected = false;
        }
    }

    pub fn get_active_device(&self) -> Option<Arc<TokioMutex<UsbDevice>>> {
        self.active_device.clone()
    }
}

impl Default for DeviceManager {
    fn default() -> Self {
        Self::new()
    }
}
