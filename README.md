```
     ██████╗ ██████╗ ███████╗███╗   ██╗███████╗██╗      █████╗ ███████╗██╗  ██╗
    ██╔═══██╗██╔══██╗██╔════╝████╗  ██║██╔════╝██║     ██╔══██╗██╔════╝██║  ██║
    ██║   ██║██████╔╝█████╗  ██╔██╗ ██║█████╗  ██║     ███████║███████╗███████║
    ██║   ██║██╔═══╝ ██╔══╝  ██║╚██╗██║██╔══╝  ██║     ██╔══██║╚════██║██╔══██║
    ╚██████╔╝██║     ███████╗██║ ╚████║██║     ███████╗██║  ██║███████║██║  ██║
     ╚═════╝ ╚═╝     ╚══════╝╚═╝  ╚═══╝╚═╝     ╚══════╝╚═╝  ╚═╝╚══════╝╚═╝  ╚═╝
                                                                        v1.1.0
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
│                                           128MB to 8GB+                 │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## 📥 Download

**v1.0.0** — First stable release

| | | |
|:---:|:---:|:---:|
| [**Windows**](https://github.com/openflash/openflash/releases/download/v1.0.0/OpenFlash-1.0.0-x64.msi)<br>`OpenFlash-1.0.0-x64.msi` | [**macOS**](https://github.com/openflash/openflash/releases/download/v1.0.0/OpenFlash-1.0.0.dmg)<br>`OpenFlash-1.0.0.dmg` | [**Linux**](https://github.com/openflash/openflash/releases/download/v1.0.0/OpenFlash-1.0.0.AppImage)<br>`OpenFlash-1.0.0.AppImage` |

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
4. Explore: Hex View, Bitmap, Analysis
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

### Shows you everything

```
┌─────────────────────────────────────────────────────────────┐
│ [Operations] [Hex View] [Bitmap] [Analysis]                 │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  Hex Viewer         │  Bitmap View                          │
│  ─────────────────  │  ────────────                         │
│  00000000: 68 73 71 │  ████░░░░████████░░░░████             │
│  00000010: 73 00 00 │  ████████████████████████             │
│  00000020: 04 00 00 │  ░░░░░░░░░░░░░░░░░░░░░░░░             │
│  ...                │  ████████░░░░░░░░████████             │
│                     │                                       │
│  Virtual scroll     │  Click to jump to page                │
│  for GB-size dumps  │  Dark = data, Light = empty           │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

---

## 🏎️ Speed

| | Pico (RP2040) | Blue Pill (STM32) |
|---|:---:|:---:|
| **Chip ID** | 10ms | 50ms |
| **Page read** | 100μs | 500μs |
| **1GB dump** | 45 min | 3.5 hours |
| **Price** | $4 | $2 |
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
│  │  └──────────────┘  └──────────────┘  │   └── Analysis AI    │  │  │
│  │                                      └──────────────────────┘  │  │
│  └────────────────────────────────────────────────────────────────┘  │
└───────────────────────────────┬──────────────────────────────────────┘
                                │
                          USB Bulk Transfer
                          64-byte packets
                                │
┌───────────────────────────────▼──────────────────────────────────────┐
│  RASPBERRY PI PICO                                                   │
│  ┌────────────────────────────────────────────────────────────────┐  │
│  │  Firmware (~10KB)                                              │  │
│  │  ├── USB handler (embassy-usb)                                 │  │
│  │  └── GPIO bit-bang / PIO for NAND timing                       │  │
│  └────────────────────────────────────────────────────────────────┘  │
└───────────────────────────────┬──────────────────────────────────────┘
                                │
                          Parallel bus
                          8 data + 6 control
                                │
┌───────────────────────────────▼──────────────────────────────────────┐
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

v1.1  ✅  SPI NAND support ← YOU ARE HERE
          ├── 20+ SPI NAND chips
          ├── Quad SPI (QSPI) support
          ├── Internal ECC status
          └── Only 4 wires needed!

v1.2  📋  eMMC support
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

**OpenFlash v1.1.0**

*Your data wants to be free.*

[⭐ Star](https://github.com/openflash/openflash) · [🐛 Issues](https://github.com/openflash/openflash/issues) · [💬 Discuss](https://github.com/openflash/openflash/discussions)

</div>
