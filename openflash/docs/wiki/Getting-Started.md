# Getting Started

## Before you start

There are no downloadable builds. The `v3.0.0` and `v1.5.0` releases are tags
with no attached files — no installers, no firmware images — so everything below
builds from source. An earlier version of this page told you to download
`OpenFlash-x.x.x-setup.exe` and `openflash-rp2040.uf2` from the Releases page;
neither has ever existed there.

What you can actually run today:

- the CLI and the desktop app, against **SPI NOR** flash, using a **Raspberry
  Pi, Orange Pi or Banana Pi** as the programmer
- everything, against a built-in **emulator**, with no hardware at all

The microcontroller firmware — Pico, Blue Pill, ESP32, Teensy — does not build.
See [PLATFORMS.md](../PLATFORMS.md).

## Build from source

Prerequisites:

- Rust 1.85 or newer (the desktop app needs 1.88)
- Node.js 18+, and Tauri's system packages, only if you want the desktop app

```bash
git clone https://github.com/bobberdolle1/openflash
cd openflash/openflash
cargo build --release -p openflash-cli
```

The binary lands in `target/release/openflash`.

For the desktop app, on Debian or Ubuntu:

```bash
sudo apt install libwebkit2gtk-4.1-dev libudev-dev
cd gui && npm ci && npm run tauri build
```

## First run, without hardware

The emulator implements the device side of the protocol against a byte array,
with real flash semantics — programming only clears bits, a program crossing a
page boundary wraps within the page, an erase needs the write-enable latch. A
mistake in the tooling shows up there rather than on your chip.

```bash
cd openflash/openflash
alias of='./target/release/openflash --emulate 2097152 --emulate-image /tmp/chip.bin'

of detect                       # identify the emulated part
of --yes write -i firmware.bin  # erase the affected sectors, program, verify
of read -o dump.bin             # dump it back
of verify --file firmware.bin   # compare the chip against the file
```

`--emulate-image` keeps the chip's contents in a file between commands. Without
it each invocation gets a freshly erased chip, so a write could not be read back.

Every emulated run says so on stderr, so an emulator result is never mistaken for
a hardware one.

In the desktop app, the emulator appears in the device list as
"Emulated SPI NOR chip (no hardware)".

## With hardware

You need a Raspberry Pi, Orange Pi or Banana Pi, wired to a SPI NOR chip. See
[Hardware Setup](Hardware-Setup.md) for the wiring.

On the board:

```bash
# Pick the crate for your board
cargo run --release -p openflash-firmware-raspberry-pi
cargo run --release -p openflash-firmware-orange-pi
cargo run --release -p openflash-firmware-banana-pi
```

The agent needs spidev enabled — on a Raspberry Pi that is `dtparam=spi=on` in
`/boot/firmware/config.txt` and a reboot. If it cannot open the bus it still
starts and reports no interfaces, so you can tell "no bus" from "no chip".

From your machine:

```bash
openflash --unix /tmp/openflash.sock detect
openflash --unix /tmp/openflash.sock read -o dump.bin
```

To work over the network instead, set `OPENFLASH_TCP=0.0.0.0:9999` on the board
and use `openflash --tcp board.local:9999`.

## Safety

`read` and `verify` open the device read-only, so a dump cannot alter the chip it
is reading. `write` and `erase` ask for confirmation against real hardware unless
you pass `--yes`. Programming erases first and preserves a partially covered
sector by reading it out and rewriting it.

Take a full dump before writing anything. It is the only way back.

## Next steps

- [Hardware Setup](Hardware-Setup.md) — wiring
- [Supported Chips](Supported-Chips.md) — what the database knows, and what can
  actually be read
- [Troubleshooting](Troubleshooting.md)
- [PLATFORMS.md](../PLATFORMS.md) — per-board status
