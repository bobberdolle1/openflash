//! USB Protocol definitions for OpenFlash
//! Defines command packets for communication between host and firmware

use serde::{Deserialize, Serialize};

/// USB Protocol Commands
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
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

/// Protocol packet structure (64 bytes total)
#[derive(Debug, Clone)]
pub struct Packet {
    pub cmd: Command,
    pub args: [u8; 63],
}

impl Packet {
    pub fn new(cmd: Command, args: &[u8]) -> Self {
        let mut packet_args = [0u8; 63];
        let copy_len = args.len().min(63);
        packet_args[..copy_len].copy_from_slice(&args[..copy_len]);
        
        Self {
            cmd,
            args: packet_args,
        }
    }

    pub fn to_bytes(&self) -> [u8; 64] {
        let mut bytes = [0u8; 64];
        bytes[0] = self.cmd as u8;
        bytes[1..].copy_from_slice(&self.args);
        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 64 {
            return None;
        }

        let cmd = Command::from_u8(bytes[0])?;
        let mut args = [0u8; 63];
        args.copy_from_slice(&bytes[1..64]);

        Some(Self { cmd, args })
    }
}

/// Common NAND commands
pub mod nand_commands {
    pub const READ1: u8 = 0x00;
    pub const READ2: u8 = 0x30;
    pub const READID: u8 = 0x90;
    pub const PAGEPROG: u8 = 0x80;
    pub const PROGSTART: u8 = 0x10;
    pub const BLOCKERASE: u8 = 0x60;
    pub const ERASESTART: u8 = 0xD0;
    pub const READSTATUS: u8 = 0x70;
    pub const RESET: u8 = 0xFF;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_packet_serialization() {
        let packet = Packet::new(Command::Ping, &[0x01, 0x02, 0x03]);
        let bytes = packet.to_bytes();
        let parsed = Packet::from_bytes(&bytes).unwrap();

        assert_eq!(parsed.cmd, Command::Ping);
        assert_eq!(parsed.args[0], 0x01);
        assert_eq!(parsed.args[1], 0x02);
        assert_eq!(parsed.args[2], 0x03);
    }

    #[test]
    fn test_command_from_u8() {
        assert_eq!(Command::from_u8(0x01), Some(Command::Ping));
        assert_eq!(Command::from_u8(0x07), Some(Command::ReadId));
        assert_eq!(Command::from_u8(0xFF), None);
    }
}
