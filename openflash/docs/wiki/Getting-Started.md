# Getting Started

This guide will help you get OpenFlash up and running.

## Prerequisites

### Software
- Windows 10/11, macOS 10.15+, or Linux
- No additional drivers needed (uses native USB)

### Hardware (choose one)
- **Raspberry Pi Pico** (RP2040) - Recommended, ~$4
- **Blue Pill** (STM32F103C8T6) - Budget option, ~$2

### NAND Flash
- Any ONFI-compliant parallel NAND flash
- 8-bit data bus (16-bit not yet supported)
- 3.3V operation

## Installation

### Option 1: Download Release (Recommended)

1. Go to [Releases](https://github.com/openflash/openflash/releases)
2. Download the installer for your OS:
   - Windows: `OpenFlash-x.x.x-setup.exe`
   - macOS: `OpenFlash-x.x.x.dmg`
   - Linux: `OpenFlash-x.x.x.AppImage` or `.deb`
3. Install and run

### Option 2: Build from Source

```bash
# Prerequisites
# - Rust 1.70+
# - Node.js 18+
# - Tauri prerequisites (see tauri.app)

git clone https://github.com/openflash/openflash.git
cd openflash/openflash/gui
npm install
cargo tauri build
```

## First Run (Without Hardware)

OpenFlash includes a mock device for testing:

1. Launch OpenFlash
2. Click **"Mock"** button
3. Click **"Scan"** - you'll see "OpenFlash Mock Device"
4. Click **"Connect"**
5. Click **"Dump NAND"**
6. Explore the tabs: Hex View, Bitmap, Analysis

## Flashing Firmware

### Raspberry Pi Pico (RP2040)

1. Download `openflash-rp2040.uf2` from Releases
2. Hold BOOTSEL button on Pico
3. Connect USB while holding button
4. Pico appears as USB drive
5. Copy `.uf2` file to the drive
6. Pico reboots automatically

### STM32F103 (Blue Pill)

1. Download `openflash-stm32f1.bin` from Releases
2. Use ST-Link or USB-Serial adapter
3. Flash using `st-flash` or STM32CubeProgrammer

## Wiring

See [Hardware Setup](Hardware-Setup.md) for detailed pinout diagrams.

## Next Steps

- [Hardware Setup](Hardware-Setup.md) - Wire up your NAND chip
- [Supported Chips](Supported-Chips.md) - Check if your chip is supported
- [Troubleshooting](Troubleshooting.md) - Common issues and solutions
