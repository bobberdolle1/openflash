# OpenFlash Wiki

## What actually works today

Put first, because the rest of this wiki describes the project's intent and the
two are not the same thing.

**Works end to end:** SPI NOR flash, over a Raspberry Pi, Orange Pi or Banana Pi
running the OpenFlash agent — identify, dump, erase, program, verify.

**Works without hardware:** the built-in emulator implements the device side of
the protocol with real flash semantics, so every command can be tried and seen.

**Does not work yet:** the microcontroller firmware. None of the RP2040, STM32,
ESP32, Teensy, RP2350 or Arduino GIGA firmware currently builds, so despite the
"cheap microcontroller" framing below, a $4 Pico cannot run this today. Parallel
NAND, SPI NAND, eMMC and UFS have chip databases and dump parsers, but no
firmware that can drive them.

[PLATFORMS.md](../PLATFORMS.md) gives the per-board, per-interface detail,
including what specifically is missing. [PROTOCOL.md](../PROTOCOL.md) is the wire
format.

## Quick Links

- [Getting Started](Getting-Started.md)
- [Hardware Setup](Hardware-Setup.md)
- [Supported Chips](Supported-Chips.md)
- [Troubleshooting](Troubleshooting.md)
- [FAQ](FAQ.md)

## What is OpenFlash?

An open-source toolkit for reading, writing and analysing flash memory. Four
parts:

1. **Command-line tool** — identify a chip, dump it, program it, verify it
2. **Desktop application** — Tauri + React, over the same core
3. **Device firmware** — code for the board wired to the chip
4. **Core library** — transport, chip databases, ECC, dump analysis

## Philosophy

> "Cheap hardware, premium software"

All the complex logic runs on your computer rather than on the device. The device
does the bus timing and nothing else. That keeps the firmware small enough to be
written once per board — and, on the three single-board computers, shared between
them rather than rewritten.

The intended payoff is inexpensive hardware, easy firmware updates and analysis
tools that are not constrained by a microcontroller's memory. The first of those
is not delivered yet: see the status section above.

## Use Cases

- **Firmware extraction** from routers, IoT devices, game consoles
- **Data recovery** from damaged flash storage
- **Security research** and reverse engineering
- **Embedded development** and debugging
- **Educational purposes** — learn how flash memory works

Analysing a dump you already have works for all five interfaces. *Taking* the
dump works for SPI NOR.

## Getting Help

- Check this wiki
- [GitHub Discussions](https://github.com/bobberdolle1/openflash/discussions)
- [Report bugs](https://github.com/bobberdolle1/openflash/issues)

## Contributing

See [CONTRIBUTING.md](https://github.com/bobberdolle1/openflash/blob/main/CONTRIBUTING.md).
The most useful contribution right now is getting one microcontroller firmware
building and talking, or testing against real hardware — nobody working on this
has had a chip to try it on.
