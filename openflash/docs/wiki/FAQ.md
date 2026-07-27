# Frequently Asked Questions

## General

### What is OpenFlash?
OpenFlash is an open-source tool for reading, writing, and analyzing flash memory chips. It's designed for reverse engineers, hardware hackers, and data recovery specialists.

### What works right now?
Reading, erasing, programming and verifying **SPI NOR** flash, using a Raspberry
Pi, Orange Pi or Banana Pi as the programmer. Plus offline analysis of dumps you
already have, for all five interfaces.

Nothing else can take a dump yet. [PLATFORMS.md](../PLATFORMS.md) has the detail,
and it is worth reading before buying a board.

### Is it free?
Yes! OpenFlash is 100% free and open-source under the MIT license.

### What operating systems are supported?
- Windows 10/11
- macOS 10.15+
- Linux (Ubuntu 20.04+, Debian 11+, Arch, etc.)

### Do I need special hardware?
For real chips, yes: a Raspberry Pi, Orange Pi or Banana Pi, plus wires to your
SPI NOR chip. Any of the three works — they run the same agent.

To try the software without any hardware, use the built-in emulator. See
[Getting Started](Getting-Started.md).

## Hardware

### Which board should I use?
Whichever of the three you already have. The Raspberry Pi, Orange Pi and Banana
Pi agents share one implementation, so they behave identically; they differ only
in how the board's SPI controller is reached.

### Can I use a Raspberry Pi Pico, a Blue Pill or an ESP32?
Not yet. The project's own README and earlier versions of this page recommended a
$4 Pico, but none of the microcontroller firmware currently builds — they pin
dependency versions that no longer resolve and have no target configuration or
linker script. [PLATFORMS.md](../PLATFORMS.md) lists what each one needs.

This is the gap the project most wants closed; see the contributing list in the
README.

### Can I use Arduino?
No. The Arduino GIGA directory is a stub with no command table and nothing that
talks to a chip.

### What chips are supported?
The database holds 207 parts across SPI NOR, parallel NAND, SPI NAND and eMMC,
but only SPI NOR can actually be read today. See
[Supported Chips](Supported-Chips.md), which explains the difference between "in
the database" and "readable".

### Can I read parallel NAND?
Not yet. The chip database, the ONFI parameter-page parser and the ECC codecs are
all there, and they work on a dump you already have — but no firmware in this
repository can drive a parallel NAND bus. The single-board-computer agents have
scaffolding for it whose operations deliberately refuse, because the address
cycles are missing and driving a NAND without an address would read, or program,
an arbitrary page.

### Can I read eMMC/SD cards?
Not yet. eMMC parts are in the database and their CSD/EXT_CSD registers can be
parsed from a dump, but there is no firmware that can talk to one. SD cards are
not a target.

### Can I read SPI NAND?
Not yet — same position as parallel NAND: database and parsers, no firmware.

### Do I need to desolder the chip?
Usually yes, unless:
- The device has a NAND test point header
- You can access the chip in-circuit without interference

### What voltage does it use?
3.3V only. Never connect 5V to NAND chips!

## Software

### How do I test without hardware?
Connect to the "Emulated SPI NOR chip" entry in the device list, or run the CLI
with `--emulate 2097152`. The emulator implements the device side of the real
protocol against a byte array, with real flash semantics, so it exercises the
same code path as hardware. Every emulated run is labelled as such.

### What file formats can I save?
Currently raw binary (.bin). More formats planned.

### Can I write/program chips?
Yes, but use with caution! Writing incorrect data can brick devices.

### Does it support bad block management?
No, and the analysis output is careful about saying so. Manufacturers mark bad
blocks in the spare area, which the dump statistics do not read; what they report
is a count of blocks whose first page is entirely zero, which is the pattern a
worn or failed block usually leaves. That is a heuristic and it is named for what
it measures — `all_zero_blocks` — rather than presented as a bad block table.

Dumps are raw: nothing is skipped or remapped.

### What ECC algorithms are supported?
- **Hamming** — corrects one bit per sector, detects two. Checked at every bit
  position of a sector.
- **BCH** — corrects up to `t` bits over a 512- or 1024-byte sector, in the 4-,
  8-, 16- and 24-bit configurations NAND uses.

BCH previously refused to run because it mis-corrected data; that is fixed. One
limit is worth knowing: the codec is self-consistent, but a hardware NAND
controller picks its own bit order and spare-area layout, and sometimes scrambles
the data, so ECC bytes from a dump made by such a controller will not generally
verify here.

## Analysis

### What filesystems can it detect?
- SquashFS
- UBIFS
- JFFS2
- YAFFS2 (partial)
- U-Boot images
- Compressed data (gzip, LZMA, XZ)

### What does the bitmap view show?
Each pixel represents one page:
- White: empty (all `0xFF`)
- Blue: low entropy (repetitive data)
- Green: medium entropy
- Orange: high entropy
- Purple: very high entropy (compressed or encrypted)
- Red: all zeros

Red means the page read back as all zeros, which is the pattern a worn or failed
block usually leaves — not a bad block marker read from the spare area. The
legend in the app says "Bad/Zero" for the same reason.

### Can it decrypt encrypted data?
No, OpenFlash only reads raw data. Decryption is up to you.

## Troubleshooting

### Why is my chip not detected?
See [Troubleshooting](Troubleshooting.md#chip-detection-issues)

### Why is the dump all 0xFF?
The chip might be empty, or there's a wiring issue. See [Troubleshooting](Troubleshooting.md#dump-issues)

### The app won't start
See [Troubleshooting](Troubleshooting.md#gui-issues)

## Contributing

### How can I help?
- Test with different NAND chips
- Report bugs
- Improve documentation
- Submit code improvements
- Translate the UI

See [CONTRIBUTING.md](https://github.com/bobberdolle1/openflash/blob/main/CONTRIBUTING.md)

### How do I add support for a new chip?
1. Get the chip ID
2. Find the datasheet
3. Submit an issue or PR with specifications

### Can I use OpenFlash in my commercial product?
Yes, the MIT license allows commercial use. Attribution appreciated but not required.

## Safety & Legal

### Is this legal?
Reading your own devices is legal. Reading devices you don't own may not be. Always ensure you have the right to access the data.

### Can this damage my chip?
Reading is safe. Writing can potentially damage data if done incorrectly. Always backup first!

### Is my data safe?
OpenFlash runs locally on your computer. No data is sent anywhere. The app doesn't require internet access.
