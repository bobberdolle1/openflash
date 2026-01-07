```
     ██████╗ ██████╗ ███████╗███╗   ██╗███████╗██╗      █████╗ ███████╗██╗  ██╗
    ██╔═══██╗██╔══██╗██╔════╝████╗  ██║██╔════╝██║     ██╔══██╗██╔════╝██║  ██║
    ██║   ██║██████╔╝█████╗  ██╔██╗ ██║█████╗  ██║     ███████║███████╗███████║
    ██║   ██║██╔═══╝ ██╔══╝  ██║╚██╗██║██╔══╝  ██║     ██╔══██║╚════██║██╔══██║
    ╚██████╔╝██║     ███████╗██║ ╚████║██║     ███████╗██║  ██║███████║██║  ██║
     ╚═════╝ ╚═╝     ╚══════╝╚═╝  ╚═══╝╚═╝     ╚══════╝╚═╝  ╚═╝╚══════╝╚═╝  ╚═╝
                                                                        v3.0.0
```

<div align="center">

**$4-60 hardware. $0 software. 11 platforms. Infinite possibilities.**

[Download](#-download) · [5-Minute Setup](#-5-minute-setup) · [Why This Exists](#-why-this-exists)

---

[![Release](https://img.shields.io/github/v/release/openflash/openflash?style=flat-square&color=00ff00)](https://github.com/openflash/openflash/releases)
[![License](https://img.shields.io/badge/license-MIT-blue?style=flat-square)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-win%20%7C%20mac%20%7C%20linux-lightgrey?style=flat-square)]()

</div>

---

## 💀 Why This Exists

You found a router. Or an old SSD. Or some sketchy IoT device from AliExpress.

You want to know what's inside. You want the firmware. The secrets. The data.

Commercial NAND programmers cost **$200-2000**. They run on Windows XP. They look like they were designed in 2003. Because they were.

**OpenFlash** is different:

```
┌─────────────────────────────────────────────────────────────────────────┐
│                                                                         │
│   Your $4-60 Hardware          ──────►    Any NAND Flash Chip           │
│                                                                         │
│   Microcontrollers:                       Parallel NAND:                │
│   • Raspberry Pi Pico ($4)                Samsung, Hynix, Micron...     │
│   • Raspberry Pi Pico 2 ($5)                                            │
│   • STM32F4 Black Pill ($5)               SPI NAND (v1.1+):             │
│   • Arduino GIGA R1 ($60)                 GigaDevice, Winbond...        │
│   • ESP32 ($4)                                                          │
│   • Teensy 4.0/4.1 ($20-30) ⚡ NEW        eMMC (v1.2+):                 │
│                                           Samsung, Micron, SanDisk...   │
│   Single Board Computers:                                               │
│   • Raspberry Pi 4/5 ($35-75)             128MB to 8GB+                 │
│   • Orange Pi ($15-50)                                                  │
│   • Banana Pi ($15-35) 🍌 NEW             11 platforms supported!       │
│                                                                         │
│   + jumper wires ($1)                                                   │
│   + This software (free)                                                │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## 📥 Download

**v3.0.0** — OpenFlash Pro (Cloud, Teams, Crowdsourcing!)

| | | |
|:---:|:---:|:---:|
| [**Windows**](https://github.com/openflash/openflash/releases/download/v3.0.0/OpenFlash-3.0.0-x64.msi)<br>`OpenFlash-3.0.0-x64.msi` | [**macOS**](https://github.com/openflash/openflash/releases/download/v3.0.0/OpenFlash-3.0.0.dmg)<br>`OpenFlash-3.0.0.dmg` | [**Linux**](https://github.com/openflash/openflash/releases/download/v3.0.0/OpenFlash-3.0.0.AppImage)<br>`OpenFlash-3.0.0.AppImage` |

<details>
<summary><b>Build from source</b></summary>

```bash
git clone https://github.com/openflash/openflash.git
cd openflash/openflash/gui && npm i && cargo tauri build
```
Requires: Rust 1.70+, Node 18+, [Tauri prerequisites](https://tauri.app/v1/guides/getting-started/prerequisites)

</details>

---

## ⚡ 5-Minute Setup

### No hardware? Try mock mode first

```
1. Open app
2. Click [Mock] → [Scan] → [Connect]
3. Click [Dump NAND]
4. Explore: Hex View, Bitmap, Analysis, AI
```

### Got a Raspberry Pi Pico?

**Wire it up** — choose your interface:

<details>
<summary><b>Parallel NAND (10 minutes with jumper wires)</b></summary>

```
PICO          NAND
────          ────
GP0   ───►    CLE
GP1   ───►    ALE
GP2   ───►    WE#
GP3   ───►    RE#
GP4   ───►    CE#
GP5   ───►    R/B#  (+ 10kΩ pull-up to 3.3V)
GP6   ───►    D0
GP7   ───►    D1
GP8   ───►    D2
GP9   ───►    D3
GP10  ───►    D4
GP11  ───►    D5
GP12  ───►    D6
GP13  ───►    D7
3V3   ───►    VCC   ⚠️  3.3V ONLY — 5V = dead chip
GND   ───►    GND
```

</details>

<details>
<summary><b>SPI NAND (v1.1+ — only 4 wires!)</b></summary>

```
PICO          SPI NAND
────          ────────
GP16  ───►    DO (MISO)
GP17  ───►    CS#
GP18  ───►    CLK
GP19  ───►    DI (MOSI)
3V3   ───►    VCC   ⚠️  3.3V ONLY
GND   ───►    GND
```

</details>

<details>
<summary><b>eMMC (v1.2+ — SPI mode, 4 wires)</b></summary>

```
PICO          eMMC
────          ────
GP12  ───►    DAT0 (MISO)
GP13  ───►    CS# (directly controlled)
GP14  ───►    CLK
GP15  ───►    CMD (MOSI)
3V3   ───►    VCC   ⚠️  3.3V ONLY — check your eMMC voltage!
GND   ───►    GND

Note: eMMC chips often require 1.8V or 3.3V — verify before connecting!
      Some eMMC modules have onboard voltage regulators.
```

</details>

**Flash firmware:**
1. Hold BOOTSEL on Pico
2. Plug USB
3. Drop `openflash-rp2040.uf2` onto the drive
4. Done

**Dump your chip:**
1. Open app → Scan → Connect
2. Chip auto-detected (30+ in database)
3. Click Dump → Wait → Analyze

---

## 🔥 What It Does

### Reads any NAND flash

```
Parallel NAND:
├── SLC, MLC, TLC
├── ONFI 1.0 → 4.0
├── 8-bit bus (16-bit coming)
└── 30+ chips: Samsung, Hynix, Micron, Toshiba, Macronix

SPI NAND (v1.1+):
├── Standard SPI + Quad SPI (QSPI)
├── Internal ECC support
├── 20+ chips: GigaDevice, Winbond, Macronix, Micron, Toshiba, XTX
└── Only 4 wires needed!

eMMC (v1.2+):
├── SPI mode communication
├── CID/CSD/EXT_CSD register access
├── Block read/write (512 bytes)
├── Boot partition support
└── Samsung, Micron, SanDisk, Toshiba, Kingston
```

### Fixes bit errors

```
ECC Engine
├── Hamming     →  1-bit correction   →  old SLC
├── BCH-4       →  4-bit correction   →  modern SLC
├── BCH-8       →  8-bit correction   →  MLC
└── BCH-16      →  16-bit correction  →  TLC
```

### Finds what's inside

```
Auto-detect:
├── Filesystems    SquashFS, UBIFS, JFFS2, CramFS
├── Compression    gzip, LZMA, XZ
├── Bootloaders    U-Boot headers
├── Bad blocks     Factory + runtime markers
└── Entropy map    Spot encrypted/compressed regions
```

### 🤖 AI Analysis v1.4 (NEW!)

```
AI-Powered Features:
├── Pattern Recognition
│   ├── Encrypted regions (high entropy detection)
│   ├── Compressed data (gzip, LZMA, XZ, zstd, LZ4)
│   ├── Executable code (ELF, U-Boot, kernels)
│   ├── Text/ASCII content
│   ├── Bootloader & device tree detection
│   └── Repeating patterns
│
├── Filesystem Detection (v1.4 NEW!)
│   ├── YAFFS2, UBIFS, JFFS2
│   ├── SquashFS, CramFS
│   ├── ext2/3/4, F2FS
│   ├── FAT16/32, NTFS
│   └── Auto-detect at any offset
│
├── OOB/Spare Analysis (v1.4 NEW!)
│   ├── Auto-detect ECC scheme
│   │   ├── Hamming, BCH4-40
│   │   ├── LDPC, Reed-Solomon
│   │   └── Visual OOB layout
│   ├── Bad block marker location
│   └── User data area mapping
│
├── Encryption Key Search (v1.4 NEW!)
│   ├── AES-128/192/256 key detection
│   ├── High-entropy region analysis
│   ├── Context-aware key identification
│   └── Deep scan mode
│
├── Wear Leveling Analysis (v1.4 NEW!)
│   ├── Erase count estimation
│   ├── Hot/cold block identification
│   ├── Remaining life prediction
│   └── Wear distribution stats
│
├── Memory Map (v1.4 NEW!)
│   ├── Visual memory layout
│   ├── Partition detection
│   ├── Interactive navigation
│   └── Color-coded regions
│
├── Dump Comparison (v1.4 NEW!)
│   ├── Diff analysis between dumps
│   ├── Bit-flip detection
│   ├── Similarity scoring
│   └── Changed block tracking
│
├── Anomaly Detection
│   ├── Bad block markers
│   ├── Bit rot / ECC errors
│   ├── Truncated dumps
│   └── Corrupted headers
│
├── Recovery Suggestions
│   ├── ECC correction recommendations
│   ├── Re-dump suggestions
│   └── Success probability estimates
│
└── Report Export (v1.4 NEW!)
    └── Markdown analysis reports
```

### Shows you everything

```
┌─────────────────────────────────────────────────────────────┐
│ [Operations] [Hex View] [Bitmap] [Analysis] [🤖 AI]         │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  AI Analysis v1.4                                           │
│  ─────────────────                                          │
│  📊 Patterns │ ⚠️ Issues │ 📁 FS │ 📋 OOB │ 🔐 Keys        │
│  📈 Wear │ 🗺️ Map │ 🔧 Recovery │ 💡 Tips                  │
│                                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │ Data Quality: ████████████░░░░ 78%                  │   │
│  │ Encryption:   ██░░░░░░░░░░░░░░ 12%                  │   │
│  │ Compression:  ██████░░░░░░░░░░ 35%                  │   │
│  │ Flash Life:   ████████████████ 95%                  │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                             │
│  Memory Map:                                                │
│  [Boot][Kernel████][RootFS██████████][Config][Empty░░░]    │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

---

## 🏎️ Speed

| | Pico | Pico 2 | STM32F4 | Arduino GIGA | Teensy 4.x | ESP32 | RPi 4/5 | Orange Pi | Banana Pi |
|---|:---:|:---:|:---:|:---:|:---:|:---:|:---:|:---:|:---:|
| **Chip ID** | 10ms | 5ms | 5ms | 2ms | 1ms | 15ms | 3ms | 5ms | 5ms |
| **Page read** | 100μs | 60μs | 50μs | 20μs | 10μs | 120μs | 30μs | 50μs | 50μs |
| **1GB dump** | 45 min | 30 min | 25 min | 10 min | 3-5 min | 50 min | 12 min | 20 min | 25 min |
| **Price** | ~$4 | ~$5 | ~$5 | ~$60 | ~$20-30 | ~$4 | ~$35-75 | ~$15-50 | ~$15-35 |
| **SPI NAND** | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| **eMMC** | ✅ | ✅ | ✅ | ✅ HS200 | ✅ | ✅ | ✅ | ✅ | ✅ |
| **NV-DDR** | ❌ | ✅ | ❌ | ✅ | ✅ | ❌ | ❌ | ❌ | ❌ |
| **WiFi** | ❌ | ❌ | ❌ | ✅ | ❌ | ✅ | ✅ | ✅ | ✅ |
| **USB** | CDC | CDC | OTG | HS 480M | HS 480M | UART* | N/A | N/A | N/A |
| **SD Card** | ❌ | ❌ | ❌ | ✅ | ✅ (4.1) | ❌ | ✅ | ✅ | ✅ |
| **Verdict** | ✅ Start | ⚡ Fast | 💪 MCU | 🏆 Pro | ⚡ Speed | 📶 WiFi | 🖥️ Server | 💰 Budget | 🍌 Alt |

*ESP32-S2/S3/C3 have native USB
**RPi/Orange Pi/Banana Pi connect via network (TCP/Unix socket)
***Teensy 4.x: USB High Speed (480 Mbit/s) = 10-20x faster transfers!

---

## 🧬 Architecture

```
┌──────────────────────────────────────────────────────────────────────┐
│  YOUR COMPUTER                                                       │
│  ┌────────────────────────────────────────────────────────────────┐  │
│  │  OpenFlash App                                                 │  │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────────────┐  │  │
│  │  │    React     │  │    Tauri     │  │    openflash-core    │  │  │
│  │  │   Frontend   │◄─┤    Rust      │◄─┤   ├── ONFI database  │  │  │
│  │  │  TypeScript  │  │   Backend    │  │   ├── ECC engine     │  │  │
│  │  └──────────────┘  └──────────────┘  │   └── AI Analysis    │  │  │
│  │                                      └──────────────────────┘  │  │
│  └────────────────────────────────────────────────────────────────┘  │
└───────────────────────────────────────┬──────────────────────────────┘
                                        │
                                  USB Bulk Transfer
                                  64-byte packets
                                        │
┌───────────────────────────────────────▼──────────────────────────────┐
│  RASPBERRY PI PICO                                                   │
│  ┌────────────────────────────────────────────────────────────────┐  │
│  │  Firmware (~10KB)                                              │  │
│  │  ├── USB handler (embassy-usb)                                 │  │
│  │  └── GPIO bit-bang / PIO for NAND timing                       │  │
│  └────────────────────────────────────────────────────────────────┘  │
└───────────────────────────────────────┬──────────────────────────────┘
                                        │
                                  Parallel bus
                                  8 data + 6 control
                                        │
┌───────────────────────────────────────▼──────────────────────────────┐
│  NAND FLASH CHIP                                                     │
│  Your secrets live here                                              │
└──────────────────────────────────────────────────────────────────────┘
```

**Philosophy:** All brains in the app. Firmware is dumb pipe. Cheap hardware, premium software.

---

## 📖 Docs

| | |
|---|---|
| **[Getting Started](openflash.wiki/Getting-Started.md)** | First run guide |
| **[Hardware Guide](openflash/docs/HARDWARE_GUIDE.md)** | Wiring, BOM, PCB |
| **[Supported Chips](openflash.wiki/Supported-Chips.md)** | Compatibility list |
| **[Troubleshooting](openflash.wiki/Troubleshooting.md)** | When things break |
| **[Changelog](CHANGELOG.md)** | Release history |
| **[Roadmap](ROADMAP.md)** | Future plans |

---

## 🛠️ Build Firmware

<details>
<summary><b>RP2040 (Raspberry Pi Pico)</b></summary>

```bash
rustup target add thumbv6m-none-eabi
cd openflash/firmware/rp2040
cargo build --release --target thumbv6m-none-eabi
# Hold BOOTSEL → plug USB → copy .uf2
```

</details>

<details>
<summary><b>RP2350 (Raspberry Pi Pico 2) — v2.3+</b></summary>

```bash
rustup target add thumbv8m.main-none-eabihf
cd openflash/firmware/rp2350
cargo build --release --target thumbv8m.main-none-eabihf
# Hold BOOTSEL → plug USB → copy .uf2
```

</details>

<details>
<summary><b>STM32F103 (Blue Pill)</b></summary>

```bash
rustup target add thumbv7m-none-eabi
cd openflash/firmware/stm32f1
cargo build --release --target thumbv7m-none-eabi
# Flash via ST-Link or serial bootloader
```

</details>

<details>
<summary><b>STM32F4 (Black Pill) — v1.5+</b></summary>

```bash
rustup target add thumbv7em-none-eabihf
cd openflash/firmware/stm32f4
cargo build --release --target thumbv7em-none-eabihf
# Flash via ST-Link, DFU, or probe-rs
```

</details>

<details>
<summary><b>Arduino GIGA R1 WiFi (STM32H747) — v2.3+</b></summary>

```bash
rustup target add thumbv7em-none-eabihf
cd openflash/firmware/arduino_giga
cargo build --release --target thumbv7em-none-eabihf
# Flash via DFU (double-tap reset) or probe-rs
```

</details>

<details>
<summary><b>ESP32 — v1.5+</b></summary>

```bash
# Install espup (ESP32 Rust toolchain)
cargo install espup
espup install

# Build
cd openflash/firmware/esp32
cargo build --release

# Flash
espflash flash target/xtensa-esp32-none-elf/release/openflash-firmware-esp32
```

</details>

<details>
<summary><b>Raspberry Pi SBC — v2.3+</b></summary>

```bash
# Build on the Pi itself or cross-compile
cd openflash/firmware/raspberry_pi
cargo build --release

# Run as daemon (requires root for GPIO)
sudo ./target/release/openflash-gpio --tcp 0.0.0.0:5000
```

</details>

<details>
<summary><b>Orange Pi — v2.3+</b></summary>

```bash
# Build on the Orange Pi itself
cd openflash/firmware/orange_pi
cargo build --release

# Run as daemon (requires root for /dev/mem)
sudo ./target/release/openflash-gpio --tcp 0.0.0.0:5000
```

</details>

<details>
<summary><b>Teensy 4.0/4.1 — v2.3.5+ ⚡ NEW</b></summary>

```bash
# Install Teensy toolchain
rustup target add thumbv7em-none-eabihf

# Build for Teensy 4.0
cd openflash/firmware/teensy4
cargo build --release --target thumbv7em-none-eabihf --features teensy40

# Build for Teensy 4.1 (with SD card support)
cargo build --release --target thumbv7em-none-eabihf --features teensy41

# Flash via Teensy Loader or teensy_loader_cli
teensy_loader_cli --mcu=TEENSY40 -w target/thumbv7em-none-eabihf/release/openflash-firmware-teensy4.hex
```

**Why Teensy 4.x?**
- USB High Speed (480 Mbit/s) = 10-20x faster than Pico/STM32
- 600 MHz ARM Cortex-M7 = soft ECC on-the-fly
- Teensy 4.1: SD card slot for autonomous operation
- FlexIO for precise NAND timing (NV-DDR support)

</details>

<details>
<summary><b>Banana Pi — v2.3.5+ 🍌 NEW</b></summary>

```bash
# Build on the Banana Pi itself
cd openflash/firmware/banana_pi
cargo build --release

# Run as daemon (requires root for /dev/mem or /dev/spidev)
sudo ./target/release/openflash-gpio --tcp 0.0.0.0:5000

# Supported boards:
# - Banana Pi M2 Zero (Allwinner H3) - RPi Zero form factor
# - Banana Pi M4 Berry (Allwinner H618) - RPi 4 alternative
# - Banana Pi BPI-F3 (SpacemiT K1) - RISC-V!
```

**Note:** Banana Pi is best for SPI NAND/NOR and eMMC. Parallel NAND is not recommended on Linux SBCs due to timing constraints.

</details>

---

## 🗺️ Roadmap

```
v1.0  ✅  Initial release
          ├── Parallel NAND read/write
          ├── 30+ chips in database
          ├── Hamming + BCH ECC
          └── SquashFS/UBIFS/JFFS2 detection

v1.1  ✅  SPI NAND support
          ├── 20+ SPI NAND chips
          ├── Quad SPI (QSPI) support
          ├── Internal ECC status
          └── Only 4 wires needed!

v1.2  ✅  eMMC support (RP2040)
          ├── eMMC/MMC card support via SPI mode
          ├── Read CID/CSD/EXT_CSD registers
          ├── Block read/write operations
          └── Boot partition access

v1.25 ✅  STM32F1 SPI NAND & eMMC
          ├── SPI NAND support for Blue Pill
          ├── eMMC support for Blue Pill
          └── Full feature parity with RP2040

v1.3  ✅  AI-Powered Analysis
          ├── Intelligent pattern recognition
          ├── Anomaly detection & recovery suggestions
          ├── Encryption/compression detection
          └── Chip-specific recommendations

v1.4  ✅  AI Analysis v1.4
          ├── Filesystem detection (YAFFS2, UBIFS, ext4, FAT...)
          ├── OOB/spare area analysis with ECC detection
          ├── Encryption key search (AES-128/192/256)
          ├── Wear leveling analysis & life prediction
          ├── Memory map visualization
          ├── Dump comparison (diff)
          └── Report export (Markdown)

v1.5  ✅  ESP32 & STM32F4 Support
          ├── ESP32 firmware (WiFi/BLE wireless operation!)
          ├── STM32F4 firmware (faster, USB OTG, FSMC)
          ├── Web interface for ESP32 (browser control)
          ├── 4 supported platforms: RP2040, STM32F1, STM32F4, ESP32
          └── Protocol v1.5 with WiFi commands

v1.6  ✅  NOR Flash & UFS Support
          ├── SPI NOR flash (W25Q, MX25L, IS25LP) — 30+ chips
          ├── UFS (Universal Flash Storage) — v2.0-4.0
          ├── ONFI 5.0 support with NV-DDR3
          ├── 16-bit parallel NAND bus
          └── 10 property-based tests

v1.7  ✅  Advanced Write Operations
          ├── Full chip programming with verification
          ├── Bad block management (auto-remap)
          ├── Wear leveling (erase count tracking)
          ├── Incremental backup/restore
          └── Clone chip-to-chip

v1.8  ✅  Scripting & Automation
          ├── Python API (pyopenflash)
          ├── CLI tool for headless operation
          ├── Batch processing
          ├── Custom analysis plugins
          └── CI/CD integration

v1.9  ✅  Advanced AI Features
          ├── ML-based chip identification
          ├── Firmware unpacking (binwalk)
          ├── Automatic rootfs extraction
          ├── Vulnerability scanning
          └── Custom signature database

v2.0  ✅  Multi-device & Enterprise
          ├── Multi-device parallel dumping
          ├── Device farm management
          ├── Remote operation (server mode)
          ├── Production line integration
          └── REST API

v2.1  ✅  Hardware Expansion
          ├── Official OpenFlash PCB (~$25 BOM)
          ├── TSOP-48 ZIF adapter board
          ├── BGA rework station integration
          ├── Logic analyzer mode (24 MHz)
          └── JTAG/SWD passthrough

v2.2  ✅  Expanded Chip Database
          ├── 150+ new chips across all flash types
          ├── Improved auto-detection
          └── Community chip submissions

v2.3  ✅  Platform Expansion
          ├── RP2350 (Raspberry Pi Pico 2) — NV-DDR, 150MHz
          ├── Arduino GIGA R1 WiFi (STM32H747) — FMC, HS USB, WiFi
          ├── Raspberry Pi SBC (3B+/4/5/Zero 2W) — Linux GPIO
          ├── Orange Pi (Zero 3/5) — Budget SBC option
          ├── Network device support (TCP/Unix socket)
          ├── GUI platform info & capabilities display
          └── 9 total platforms supported!

v2.3.5 ✅  Teensy & Banana Pi
          ├── Teensy 4.0/4.1 (NXP i.MX RT1062) — USB HS 480Mbps!
          │   ├── 10-20x faster transfers than USB Full Speed
          │   ├── 600 MHz Cortex-M7 for soft ECC on-the-fly
          │   ├── FlexIO for precise NV-DDR timing
          │   ├── SD card slot on 4.1 for autonomous operation
          │   └── Logic analyzer mode capability
          ├── Banana Pi (M2 Zero, M4 Berry, BPI-F3)
          │   ├── M2 Zero — RPi Zero form factor ($15)
          │   ├── M4 Berry — RPi 4 alternative ($25)
          │   ├── BPI-F3 — RISC-V (SpacemiT K1)!
          │   └── Best for SPI NAND/NOR/eMMC
          ├── Protocol version 0x25
          └── 11 total platforms supported!

v3.0  ✅  OpenFlash Pro ← YOU ARE HERE
          ├── Cloud sync & backup
          │   ├── Auto-sync on save
          │   ├── Conflict resolution
          │   └── Bandwidth limiting
          ├── Team collaboration
          │   ├── Organizations & teams
          │   ├── Role-based access (Owner/Admin/Member/Viewer)
          │   └── Shared projects
          ├── Chip database crowdsourcing
          │   ├── Community contributions
          │   ├── Verification workflow
          │   └── Reputation system
          ├── AI model updates OTA
          │   ├── 5 model types
          │   ├── Auto-update with notifications
          │   └── Version management
          ├── Enterprise support
          │   ├── Priority tickets
          │   └── Dedicated support
          ├── Subscription tiers (Free/Pro/Enterprise)
          └── Protocol version 0x30

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

v3.1  🔜  Mobile & Embedded
          ├── iOS app
          ├── Android app
          ├── Embedded Linux support
          └── WebAssembly core
```

---

## 🤝 Contributing

Found a bug? Got a chip we don't support? Want to add a feature?

```bash
# Dev mode with hot reload
cd openflash/gui && npm i && cargo tauri dev

# Run tests
cargo test -p openflash-core
```

PRs welcome. See [CONTRIBUTING.md](CONTRIBUTING.md).

---

## 📜 License

MIT. Do whatever. Don't sue us.

---

<div align="center">

**OpenFlash v3.0.0**

*Your data wants to be free.*

[⭐ Star](https://github.com/openflash/openflash) · [🐛 Issues](https://github.com/openflash/openflash/issues) · [💬 Discuss](https://github.com/openflash/openflash/discussions)

</div>
