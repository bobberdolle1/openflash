# Frequently Asked Questions

## General

### What is OpenFlash?
OpenFlash is an open-source tool for reading, writing, and analyzing NAND flash memory chips. It's designed for reverse engineers, hardware hackers, and data recovery specialists.

### Is it free?
Yes! OpenFlash is 100% free and open-source under the MIT license.

### What operating systems are supported?
- Windows 10/11
- macOS 10.15+
- Linux (Ubuntu 20.04+, Debian 11+, Arch, etc.)

### Do I need special hardware?
You need a cheap microcontroller (~$4-5):
- Raspberry Pi Pico (recommended)
- STM32F103 "Blue Pill"

Plus wires to connect to your NAND chip.

## Hardware

### Which microcontroller should I use?
**Raspberry Pi Pico (RP2040)** is recommended:
- Faster
- Better USB support
- Easier to flash
- More available

**STM32F103** works but is slower and harder to flash.

### Can I use Arduino?
Not currently. Arduino's USB stack isn't suitable for our protocol. We may add support in the future.

### What NAND chips are supported?
Any ONFI-compliant parallel NAND flash with 8-bit data bus. See [Supported Chips](Supported-Chips.md).

### Can I read eMMC/SD cards?
No, OpenFlash is for raw NAND flash only. eMMC and SD cards have built-in controllers.

### Can I read SPI NAND?
Not yet, but it's on the roadmap.

### Do I need to desolder the chip?
Usually yes, unless:
- The device has a NAND test point header
- You can access the chip in-circuit without interference

### What voltage does it use?
3.3V only. Never connect 5V to NAND chips!

## Software

### How do I test without hardware?
Click the "Mock" button to enable a simulated device. This lets you test all features.

### What file formats can I save?
Currently raw binary (.bin). More formats planned.

### Can I write/program chips?
Yes, but use with caution! Writing incorrect data can brick devices.

### Does it support bad block management?
OpenFlash detects bad blocks and shows them in analysis. It doesn't automatically skip them during dumps (you get raw data).

### What ECC algorithms are supported?
- Hamming (1-bit correction)
- BCH (multi-bit correction)

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
- White: Empty (0xFF)
- Blue: Low entropy (repetitive data)
- Green: Medium entropy
- Orange: High entropy
- Purple: Very high entropy (compressed/encrypted)
- Red: Potential bad block

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

See [CONTRIBUTING.md](https://github.com/openflash/openflash/blob/main/CONTRIBUTING.md)

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
