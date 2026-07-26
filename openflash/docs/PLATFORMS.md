# Platform status

What each board's firmware actually does today. The intent is that nobody buys a
board on the strength of a row in a table and then discovers the code for it is a
stub.

Every claim here is checkable: "builds" means `cargo build` succeeds,
"tested" means there are tests that run in CI, and "protocol" comes from
`protocol/tests/firmware_conformance.rs`, which reads each firmware's source and
compares its opcode table with the shared one.

Last verified: at the commit that introduced this file. If you change firmware,
update the row.

## Summary

| Platform | Builds in CI | Speaks the shared protocol | Working interfaces | Tests |
|---|---|---|---|---|
| Raspberry Pi (SBC) | yes | yes | SPI NOR | 21 |
| RP2040 (Pico) | no | no — legacy NAND opcodes, unframed v1 | none verified | none |
| STM32F103 (Blue Pill) | no | no — legacy NAND opcodes, unframed v1 | none verified | none |
| STM32F4 (Black Pill) | no | no — legacy NAND opcodes, unframed v1 | none verified | none |
| ESP32 | no | no — its own opcode base | none verified | none |
| Teensy 4.0 / 4.1 | no | almost — `GetVersion` differs | none verified | none |
| RP2350 (Pico 2) | no | no table at all | none — stubs | none |
| Arduino GIGA R1 | no | no table at all | none — stubs | none |
| Orange Pi (SBC) | no | no — its own opcode base | none verified | none |
| Banana Pi (SBC) | no | almost — `GetVersion` differs | none verified | none |

Only the Raspberry Pi agent is known to work end to end against the host tools.
It is the reference: read it before writing another.

## Detail

### Raspberry Pi (SBC) — working

A Linux daemon that drives the chip through the Pi's own SPI controller and
serves hosts over a Unix socket or TCP.

- SPI NOR: read, page program, sector/32K/64K/chip erase, status,
  write-enable/disable. Wired to `/dev/spidev0.0`.
- Uses `openflash-protocol`, framed revision 2, so it cannot drift from the host.
- Advertises SPI NOR only, and advertises nothing when the SPI device could not
  be opened, so a host can distinguish "no bus" from "no chip".
- Parallel NAND (`gpio_nand.rs`) is a scaffold whose operations return
  `NotImplemented`: the command sequences are there but the address cycles are
  not, and driving a NAND without an address would read or program an arbitrary
  page. Not advertised.
- Built, linted and tested by CI on every push.

Run it on the Pi, then from the host:

```bash
openflash --unix /tmp/openflash.sock detect
openflash --unix /tmp/openflash.sock read -o dump.bin
```

### RP2040, STM32F103, STM32F4 — substantial, unverified

These have the most complete drivers in the repository: `rp2040` is ~2700 lines
across PIO-based NAND, SPI NOR, SPI NAND and eMMC; `stm32f1` ~2300; `stm32f4`
~1600 with SPI NOR and its USB handler filled in but eMMC, parallel NAND and
SPI NAND left as stubs of a few dozen lines.

None of them builds:

- they pin `embassy-rp`/`embassy-stm32` 0.1 with `embassy-executor` 0.5 and the
  `nightly` feature, a combination that no longer resolves,
- there is no `memory.x`, no `build.rs` and no `.cargo/config.toml`, so there is
  no target, no linker script and no runner configured for any of them.

They also speak protocol revision 1 — unframed, no CRC — and use the deprecated
`0x03`–`0x07` parallel-NAND opcodes, so even once they build, a current host will
reject the handshake.

To bring one up: pin working embassy versions, add the target configuration and
the linker script, replace the local `Command` enum with `openflash-protocol`,
and move to framed responses. `firmware/raspberry_pi/src/main.rs` shows the frame
handling; the emulator in `core/src/emulator.rs` shows the semantics a device
side has to honour.

### ESP32 — substantial, unverified, incompatible numbering

~1250 lines, with SPI NOR reasonably complete and SPI NAND, eMMC and parallel
NAND as stubs. Its opcode table is its own: `Ping` is `0x00` rather than `0x01`,
so a current host cannot even ping it, and SPI NOR sits at `0x70` rather than
`0x60`. Does not build (`esp-hal` 0.22 with no target configuration).

### Teensy 4.0 / 4.1 — closest to compatible

~900 lines. Its opcode table matches the shared one except `GetVersion`, which it
puts at `0x03`. Migrating it is the smallest job of the microcontroller
firmwares. Does not build: no target configuration.

### RP2350 (Pico 2) — stubs

499 lines total, four of six files under 80. `spi_nor.rs` is 64 lines of opcode
constants and empty methods that log and return; `usb_handler.rs` is 35 lines.
There is no command table at all. Nothing here talks to a chip.

### Arduino GIGA R1 — stubs

378 lines, three of five files under 80, `usb_handler.rs` is 31. No command
table. Nothing here talks to a chip.

### Orange Pi, Banana Pi — early

Linux agents like the Raspberry Pi one but much smaller (337 and 746 lines).
Both declare their own opcode tables and reply unframed. The Raspberry Pi agent
is the model to follow; the SPI paths are close enough that porting is mostly
mechanical.

## Interfaces

Where each flash interface stands across the project, host side and device side.

| Interface | Host support | Device support |
|---|---|---|
| SPI NOR | identify, read, erase, program, verify | Raspberry Pi agent |
| Parallel NAND | chip database, ONFI parsing, ECC | none working |
| SPI NAND | chip database | none working |
| eMMC | chip database, CSD/EXT_CSD parsing | none working |
| UFS | descriptor parsing, SCSI CDB building | none working |

The host's chip databases and dump analysis cover far more than the device layer
can reach. That asymmetry is real: parsing an existing dump works for all five
interfaces, while *taking* a dump works for SPI NOR.

## Trying the tools without hardware

The emulator implements the device side of the protocol against a byte array,
with real flash semantics — programming only clears bits, a program wrapping a
page boundary wraps within the page, erase needs the write-enable latch:

```bash
openflash --emulate 2097152 --emulate-image chip.bin detect
openflash --emulate 2097152 --emulate-image chip.bin write -i firmware.bin
openflash --emulate 2097152 --emulate-image chip.bin read -o dump.bin
```

Every emulated run is labelled on stderr. It exercises the whole host stack —
argument parsing, framing, transport, device layer — so it is a real check that
the tools work, and it is what the integration tests use. It is not a substitute
for testing against a chip.
