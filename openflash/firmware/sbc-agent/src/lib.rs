//! The OpenFlash agent for single-board computers.
//!
//! Unlike the microcontroller firmware, an SBC agent is a Linux daemon running on
//! the board itself, driving the flash chip through the board's own SPI
//! controller and serving hosts over a Unix socket or TCP.
//!
//! Three boards run one of these — Raspberry Pi, Orange Pi, Banana Pi — and
//! before this crate existed each had its own copy of the protocol handling, its
//! own opcode table and its own SPI sequencing. The copies disagreed: `Ping` was
//! `0x00` on two of them and `0x01` on the third, and none of them framed its
//! replies, so a host could not talk to any of them properly.
//!
//! # Writing a board agent
//!
//! Implement [`bus::SpiBus`] for the board's SPI controller — two methods — and
//! hand it to [`agent::Agent`]:
//!
//! ```no_run
//! use openflash_sbc_agent::{Agent, Platform, SpiNor, listen_unix};
//! # use openflash_sbc_agent::bus::{SpiBus, BusResult};
//! # struct MyBus;
//! # impl SpiBus for MyBus {
//! #     fn transfer(&mut self, _: &[u8], _: &mut [u8]) -> BusResult<()> { Ok(()) }
//! #     fn write(&mut self, _: &[u8]) -> BusResult<()> { Ok(()) }
//! #     fn is_available(&self) -> bool { true }
//! # }
//! let flash = SpiNor::new(MyBus);
//! let mut agent = Agent::new(flash, Platform::OrangePi, (3, 1, 0));
//! listen_unix("/tmp/openflash.sock", &mut agent);
//! ```
//!
//! Everything else — the wire format, request validation, opcodes, page
//! boundaries, erase alignment, busy polling, error mapping — is here and is
//! tested here, against a chip that answers on the bus.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod agent;
pub mod bus;
pub mod nor;
mod serve;

pub use agent::Agent;
pub use bus::{BusError, BusResult, SpiBus};
pub use nor::{NorError, NorResult, SpiNor, PAGE_SIZE};
pub use serve::{listen_tcp, listen_unix};

pub use openflash_protocol::{Platform, PROTOCOL_VERSION};
