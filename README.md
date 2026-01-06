```
     ██████╗ ██████╗ ███████╗███╗   ██╗███████╗██╗      █████╗ ███████╗██╗  ██╗
    ██╔═══██╗██╔══██╗██╔════╝████╗  ██║██╔════╝██║     ██╔══██╗██╔════╝██║  ██║
    ██║   ██║██████╔╝█████╗  ██╔██╗ ██║█████╗  ██║     ███████║███████╗███████║
    ██║   ██║██╔═══╝ ██╔══╝  ██║╚██╗██║██╔══╝  ██║     ██╔══██║╚════██║██╔══██║
    ╚██████╔╝██║     ███████╗██║ ╚████║██║     ███████╗██║  ██║███████║██║  ██║
     ╚═════╝ ╚═╝     ╚══════╝╚═╝  ╚═══╝╚═╝     ╚══════╝╚═╝  ╚═╝╚══════╝╚═╝  ╚═╝
                                                                        v1.4.0
```

<div align="center">

**$4 hardware. $0 software. Infinite possibilities.**

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
│   Your $4 Raspberry Pi Pico    ──────►    Any NAND Flash Chip           │
│                                                                         │
│   + 20 jumper wires ($1)                  Parallel NAND:                │
│   + This software (free)                  Samsung, Hynix, Micron...     │
│   ─────────────────────────                                             │
│   = Full NAND programmer                  SPI NAND (v1.1+):             │
│                                           GigaDevice, Winbond...        │
│                                                                         │
│                                           eMMC (v1.2+):                 │
│                                           Samsung, Micron, SanDisk...   │
│                                                                         │
│                                           128MB to 8GB+                 │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## 📥 Download

**v1.4.0** — AI Analysis Major Upgrade

| | | |
|:---:|:---:|:---:|
| [**Windows**](https://github.com/openflash/openflash/releases/download/v1.4.0/OpenFlash-1.4.0-x64.msi)<br>`OpenFlash-1.4.0-x64.msi` | [**macOS**](https://github.com/openflash/openflash/releases/download/v1.4.0/OpenFlash-1.4.0.dmg)<br>`OpenFlash-1.4.0.dmg` | [**Linux**](https://github.com/openflash/openflash/releases/download/v1.4.0/OpenFlash-1.4.0.AppImage)<br>`OpenFlash-1.4.0.AppImage` |

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

| | Pico (RP2040) | Blue Pill (STM32F1) |
|---|:---:|:---:|
| **Chip ID** | 10ms | 50ms |
| **Page read** | 100μs | 500μs |
| **1GB dump** | 45 min | 3.5 hours |
| **Price** | $4 | $2 |
| **SPI NAND** | ✅ v1.1+ | ✅ v1.25+ |
| **eMMC** | ✅ v1.2+ | ✅ v1.25+ |
| **Verdict** | ✅ Get this | 💰 Ultra budget |

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
| **[Getting Started](openflash/docs/wiki/Getting-Started.md)** | First run guide |
| **[Hardware Guide](openflash/docs/HARDWARE_GUIDE.md)** | Wiring, BOM, PCB |
| **[Supported Chips](openflash/docs/wiki/Supported-Chips.md)** | Compatibility list |
| **[Troubleshooting](openflash/docs/wiki/Troubleshooting.md)** | When things break |

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
<summary><b>STM32F103 (Blue Pill)</b></summary>

```bash
rustup target add thumbv7m-none-eabi
cd openflash/firmware/stm32f1
cargo build --release --target thumbv7m-none-eabi
# Flash via ST-Link or serial bootloader
```

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

v1.4  ✅  AI Analysis v1.4 ← YOU ARE HERE
          ├── Filesystem detection (YAFFS2, UBIFS, ext4, FAT...)
          ├── OOB/spare area analysis with ECC detection
          ├── Encryption key search (AES-128/192/256)
          ├── Wear leveling analysis & life prediction
          ├── Memory map visualization
          ├── Dump comparison (diff)
          └── Report export (Markdown)

v2.0  🚀  Multi-device parallel dumping
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

PRs welcome. See [CONTRIBUTING.md](openflash/CONTRIBUTING.md).

---

## 📜 License

MIT. Do whatever. Don't sue us.

---

<div align="center">

**OpenFlash v1.4.0**

*Your data wants to be free.*

[⭐ Star](https://github.com/openflash/openflash) · [🐛 Issues](https://github.com/openflash/openflash/issues) · [💬 Discuss](https://github.com/openflash/openflash/discussions)

</div>
