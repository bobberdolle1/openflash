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
| Raspberry Pi (SBC) | yes | yes | SPI NOR | 5 + 33 shared |
| Orange Pi (SBC) | yes | yes | SPI NOR | 6 + 33 shared |
| Banana Pi (SBC) | yes | yes | SPI NOR | 6 + 33 shared |
| RP2040 (Pico) | no | no — legacy NAND opcodes, unframed v1 | none verified | none |
| STM32F103 (Blue Pill) | no | no — legacy NAND opcodes, unframed v1 | none verified | none |
| STM32F4 (Black Pill) | no | no — legacy NAND opcodes, unframed v1 | none verified | none |
| ESP32 | no | no — its own opcode base | none verified | none |
| Teensy 4.0 / 4.1 | no | table almost matches, but is unused | none verified | none |
| RP2350 (Pico 2) | no | no table at all | none — stubs | none |
| Arduino GIGA R1 | no | no table at all | none — stubs | none |

The three single-board-computer agents work end to end against the host tools and
share one implementation, `firmware/sbc-agent`: the protocol handling, the SPI NOR
sequencing and their tests exist once. A board contributes two methods — a
full-duplex SPI transfer and a write — and nothing else. Read one of them before
writing another.

No microcontroller firmware currently builds.

## Detail

### The three SBC agents — working

Linux daemons that drive the chip through the board's own SPI controller and
serve hosts over a Unix socket or TCP. All three are built, linted and tested by
CI on every push.

| Board | Crate | SPI access |
|---|---|---|
| Raspberry Pi 3B+/4/5/Zero 2W | `firmware/raspberry_pi` | rppal |
| Orange Pi Zero 3 / Zero 2W / 5 | `firmware/orange_pi` | Linux spidev |
| Banana Pi M2 Zero / M4 Berry / BPI-F3 | `firmware/banana_pi` | Linux spidev |

Shared, in `firmware/sbc-agent`:

- SPI NOR: read, page program, sector/32K/64K/chip erase, status,
  write-enable/disable
- framed protocol revision 2, so no agent can drift from what the host speaks
- request validation before anything reaches the bus: a program that would cross
  a 256-byte page boundary is refused rather than wrapped by the chip and
  corrupting data, and an unaligned erase is refused rather than erasing the
  wrong unit
- one `SpiBus` call is one SPI transaction. This matters: the Orange Pi and
  Banana Pi agents used to issue a `write` syscall followed by a separate `read`
  on `/dev/spidev*`, which lets the kernel deassert chip select in between, so
  the chip discarded the command before the data arrived. Both now use
  `SPI_IOC_MESSAGE`.
- an agent that cannot open its bus still starts and reports no interfaces, so a
  host can tell "no bus" from "no chip"
- SPI NOR only is advertised, because that is all that is wired up

The sequencing is tested against a chip that answers on the bus, so a wrong
opcode, a reversed address or a missing write-enable shows up as wrong data in a
test rather than as a damaged chip.

Parallel NAND over Linux GPIO (`gpio_nand.rs`, `gpio.rs`) is scaffolding whose
operations refuse: the command sequences are there but the address cycles are
not, and driving a NAND without an address would read, or program, an arbitrary
page. Not advertised.

Run one on the board, then from the host:

```bash
openflash --unix /tmp/openflash.sock detect
openflash --unix /tmp/openflash.sock read -o dump.bin

# Or over the network, with OPENFLASH_TCP=0.0.0.0:9999 set on the board
openflash --tcp board.local:9999 detect
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

### Teensy 4.0 / 4.1 — the opcode table is close, the firmware is not

~900 lines. Its `Command` enum matches the shared table except `GetVersion`,
which it puts at `0x03`. An earlier version of this file concluded from that
alone that migrating it was the smallest job of the microcontroller firmwares.
That was wrong, and reading the rest of the crate is what shows it:

- `usb.rs` is a stub. `poll_command` returns `None` unconditionally and
  `send_response` discards its argument, both behind an `initialized` flag that
  nothing but `init()` touches. There is no USB implementation, so the firmware
  could never receive a command even if it built.
- the dispatcher in `main.rs` does not use that `Command` enum at all. It matches
  raw bytes on a different numbering — `0x02` is a version string, `0x03` a
  platform name, `0x04` a USB speed report — none of which is what the shared
  table assigns. The enum is dead code.
- that match also has an unreachable arm: `0x00 | 0x01` is handled first, then
  `0x01` again for "get device info", which can never run.
- replies are unframed, so protocol revision 2 is not spoken regardless.

So the work is a USB High Speed stack, a real dispatcher and framing — not a
one-line opcode change. Does not build either: no target configuration.

The `thumbv7em-none-eabihf` target it needs does install and work, so the
toolchain is not the obstacle.

### RP2350 (Pico 2) — stubs

499 lines total, four of six files under 80. `spi_nor.rs` is 64 lines of opcode
constants and empty methods that log and return; `usb_handler.rs` is 35 lines.
There is no command table at all. Nothing here talks to a chip.

### Arduino GIGA R1 — stubs

378 lines, three of five files under 80, `usb_handler.rs` is 31. No command
table. Nothing here talks to a chip.

## Interfaces

Where each flash interface stands across the project, host side and device side.

| Interface | Host support | Device support |
|---|---|---|
| SPI NOR | identify, read, erase, program, verify | all three SBC agents |
| Parallel NAND | chip database, ONFI parsing, Hamming ECC | none working |
| SPI NAND | chip database | none working |
| eMMC | chip database, CSD/EXT_CSD parsing | none working |
| UFS | descriptor parsing, SCSI CDB building | none working |

ECC: both codecs work. Hamming corrects single-bit errors and detects double-bit
ones, checked at every bit position of a sector. BCH corrects up to `t` bits over
a 512- or 1024-byte sector, in the 4-, 8-, 16- and 24-bit configurations NAND
uses.

BCH previously refused to run, because it repaired none of 4096 injected
single-bit errors and mis-corrected ten. The cause was the generator polynomial:
it was built as ∏(x − α^i) with coefficients in GF(2^13), but a binary BCH code
needs a generator over GF(2), the LCM of the minimal polynomials of the roots.
What is checked now:

- every primitive polynomial really is primitive — α is walked through the whole
  multiplicative group, and the log table is checked for gaps
- α^1 … α^2t are roots of the generator, evaluated directly
- the parity length is exactly *m·t*, giving 7, 13, 26 and 42 ECC bytes for the
  four configurations, which are the sizes NAND datasheets quote
- every one of the 4148 single-bit errors in a 512-byte codeword — data *and*
  parity — is repaired
- error patterns of weight 2…t are repaired, from a fixed seed
- patterns beyond `t` are detected, and never mis-corrected: after locating the
  bits the syndromes are recomputed and must vanish, otherwise the sector is
  reported uncorrectable and the caller's buffer is left untouched

One limit worth stating plainly: this codec is self-consistent, and it is not
automatically byte-compatible with any particular flash controller. A hardware
NAND controller chooses its own bit order, its own mapping of sectors into the
spare area, and sometimes scrambles the data, so ECC bytes taken from a dump made
by such a controller will not generally verify here. Matching a specific
controller is a separate job from having a correct BCH implementation. See
`core/src/ecc.rs`.

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
