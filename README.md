<p align="center">
  
</p>

<h11>

<p align="center">
  <strong>The Ultimate Open-Source NAND Flash Toolkit</strong><br>
  <e
</p>

<p align="center">
  <a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/License-MIT-blue?style=for-the-badge" alt="License"></a>
  <a h
  <a href="https://tauri.app/"><img src="https://img.shields.io/badge/Tauri-2.0-purple?style=for-the-badge&logo=tauri" alt="Tauri"></a>


<">
  <a href="#-quick-st</a> •
  <a href=
  <a href="#-supported-hardware">Ha</a> •
  <a href="#-documentation">Docs</a> •
  <a href="#-contributing">Contributing</a>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/Windows-0078D6?dows">
  <img src="https://img.shields.io/badge/macOS-000000?style=flat&logo=apple&log
  <img src="https://img.shields.io/badge/Linux-FCC624?style=flat&l
/p>

---

## What is OpenFlash?

**OpenFlash** is a complete hardwaered.


```
┌──────────────────────────────────────────────────────────────────┐
│  🖥️  Desktop App (Tauri + React)                                 │
│  ├── Hex Viewer with virtual scrolling                          │
│  ├── Bitmap visualization for data density analysis             │
│  ├── AI-powered filesystem detection                            │
│  └── ECC correction (Hamming + BCH)                             │
├──────────────────────────────────────────────────────────────────┤
│  📚  Core Library (Rust)                                         │
│  ├── 30+ NAND chips in ONFI database                            │
│  ├── Protocol definitions for USB communication                 │
│  └── Signature detection (SquashFS, UBIFS, JFFS2, U-Boot...)    │
├──────────────────────────────────────────────────────────────────┤
│  🔌  Firmware (Embassy Rust)                                     │
│  ├── Raspberry Pi Pico (RP2040) — Recommended                   │
│  └── Blue Pill (STM32F103) — Budget option                      │
└──────────────────────────────────────────────────────────────────┘
```

### Why OpenFlash?

| Feature | OpenFlash | Commercial Tools |
|---------|-----------|------------------|
| **Price** | ~$5 (Pico + wires) | $200-2000+ |
| **Open Source** | ✅ MIT License | ❌ Proprietary |
| **Cross-Platform** | ✅ Win/Mac/Linux | ⚠️ Often Windows-only |
| **Extensible** | ✅ Add your own chips | ❌ Vendor lock-in |
| **Modern Stack** | ✅ Rust + React | ❌ Legacy codebases |

---

## 🚀 Quick Start

### Option 1: Download Release (Recommended)

<table>
<tr>
<td align="center"><b>Windows</b></td>
<td align="center"><b>macOS</b></td>
<td align="center"><b>Linux</b></td>
</tr>
<tr>
<td align="center">
<a href="https://github.com/openflash/openflash/releases/latest/download/OpenFlash-1.0.0-x64-setup.exe">
<img src="https://img.shields.io/badge/Download-.exe-0078D6?style=for-the-badge&logo=windows" alt="Windows Download">
</a>
</td>
<td align="center">
<a href="https://github.com/openflash/openflash/releases/latest/download/OpenFlash-1.0.0-universal.dmg">
<img src="https://img.shields.io/badge/Download-.dmg-000000?style=for-the-badge&logo=apple" alt="macOS Download">
</a>
</td>
<td align="center">
<a href="https://github.com/openflash/openflash/releases/latest/download/OpenFlash-1.0.0-amd64.AppImage">
<img src="https://img.shields.io/badge/Download-.AppImage-FCC624?style=for-the-badge&logo=linux" alt="Linux Download">
</a>
</td>
</tr>
</table>

### Option 2: Build from Source

```bash
# Prerequisites: Rust 1.70+, Node.js 18+
git clone https://github.com/openflash/openflash.git
cd openflash/openflash/gui
npm install
cargo tauri build
```

### Try Without Hardware (Mock Mode)

No hardware? No problem! OpenFlash includes a mock device for testing:

1. Launch OpenFlash
2. Click **🧪 Mock** → **🔄 Scan** → **Connect**
3. Click **📥 Dump NAND** and explore the results

---

## ✨ Features

### 🔍 Smart Chip Detection

OpenFlash automatically identifies your NAND chip from its ID bytes and configures optimal timing parameters.

**Supported Manufacturers:**
Samsung • SK Hynix • Micron • Toshiba/Kioxia • Macronix • Winbond • GigaDevice

**30+ chips in database** with automatic fallback to generic ONFI detection for unknown chips.

### 🛡️ Error Correction

Built-in ECC algorithms to recover data from degraded flash:

| Algorithm | Capability | Use Case |
|-----------|------------|----------|
| **Hamming** | 1-bit correction | Legacy SLC NAND |
| **BCH-4** | 4-bit correction | Modern SLC |
| **BCH-8** | 8-bit correction | MLC NAND |
| **BCH-16** | 16-bit correction | TLC NAND |

### 🔬 Analysis Engine

Automatic detection of:
- **Filesystems:** SquashFS, UBIFS, JFFS2, CramFS
- **Compression:** gzip, LZMA, XZ
- **Bootloaders:** U-Boot signatures
- **Bad blocks:** Factory and runtime markers
- **Entropy analysis:** Identify encrypted/compressed regions

### 🎨 Visual Tools

- **Hex Viewer** — Virtual scrolling for multi-GB dumps, search, highlights
- **Bitmap View** — See data density patterns at a glance, spot empty regions
- **Signature Highlights** — Jump directly to detected filesystems

---

## 🔌 Supported Hardware

### Microcontrollers

| Board | Price | Speed | Recommendation |
|-------|-------|-------|----------------|
| **Raspberry Pi Pico** | ~$4 | ⚡ Fast | ✅ Best choice |
| **Blue Pill (STM32F103)** | ~$2 | 🐢 Slower | 💰 Budget option |

### NAND Flash Types

- ✅ **SLC** — Single-Level Cell (most reliable)
- ✅ **MLC** — Multi-Level Cell
- ✅ **TLC** — Triple-Level Cell
- ✅ **ONFI 1.0 - 4.0** compliant devices
- ✅ **8-bit parallel** interface
- 🔜 **16-bit** and **SPI NAND** coming soon

### Wiring (Raspberry Pi Pico)

```
NAND Signal    Pico GPIO    Description
───────────    ─────────    ───────────
CLE            GP0          Command Latch Enable
ALE            GP1          Address Latch Enable
WE#            GP2          Write Enable (active low)
RE#            GP3          Read Enable (active low)
CE#            GP4          Chip Enable (active low)
R/B#           GP5          Ready/Busy (needs 10kΩ pull-up)
D0-D7          GP6-GP13     8-bit Data Bus
VCC            3V3          ⚠️ 3.3V ONLY!
GND            GND          Ground
```

> 📖 **Full wiring guide:** [docs/HARDWARE_GUIDE.md](openflash/docs/HARDWARE_GUIDE.md)

---

## 📊 Performance

| Operation | RP2040 | STM32F1 |
|-----------|--------|---------|
| Chip ID Read | < 10ms | < 50ms |
| Page Read (4KB) | ~100μs | ~500μs |
| Full Dump (1GB) | ~45 min | ~3.5 hours |

*Times include USB transfer and optional ECC processing.*

---

## 🏗️ Architecture

OpenFlash follows the **"cheap hardware, premium software"** philosophy:

```
┌─────────────────────────────────────────────────────────────┐
│                     Desktop Application                     │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
│  │   React     │  │   Tauri     │  │    Core Library     │  │
│  │  Frontend   │◄─┤   Bridge    │◄─┤  (openflash-core)   │  │
│  │  TypeScript │  │    Rust     │  │   ONFI • ECC • AI   │  │
│  └─────────────┘  └─────────────┘  └─────────────────────┘  │
└────────────────────────────┬────────────────────────────────┘
                             │ USB Bulk Transfer
                             ▼
┌─────────────────────────────────────────────────────────────┐
│                    Firmware (Embassy Rust)                  │
│  ┌─────────────────────┐  ┌─────────────────────────────┐   │
│  │   USB Handler       │  │      NAND Interface         │   │
│  │   64-byte packets   │◄─┤  GPIO bit-bang / PIO        │   │
│  └─────────────────────┘  └─────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

**Key Design Decisions:**
- 🧠 **All intelligence in desktop app** — Firmware is minimal (~10KB)
- 🔄 **Async everywhere** — Tokio on desktop, Embassy on MCU
- 🦀 **100% Rust** — Memory safety from firmware to GUI backend
- 📦 **Single binary** — No runtime dependencies

---

## 📚 Documentation

| Document | Description |
|----------|-------------|
| [Getting Started](openflash/docs/wiki/Getting-Started.md) | First-time setup guide |
| [Hardware Guide](openflash/docs/HARDWARE_GUIDE.md) | Wiring diagrams and BOM |
| [Supported Chips](openflash/docs/wiki/Supported-Chips.md) | Full chip compatibility list |
| [Troubleshooting](openflash/docs/wiki/Troubleshooting.md) | Common issues and fixes |
| [FAQ](openflash/docs/wiki/FAQ.md) | Frequently asked questions |

---

## 🛠️ Building Firmware

### Raspberry Pi Pico (RP2040)

```bash
rustup target add thumbv6m-none-eabi
cd openflash/firmware/rp2040
cargo build --release --target thumbv6m-none-eabi

# Flash: Hold BOOTSEL, connect USB, copy .uf2 to drive
```

### Blue Pill (STM32F103)

```bash
rustup target add thumbv7m-none-eabi
cd openflash/firmware/stm32f1
cargo build --release --target thumbv7m-none-eabi

# Flash with ST-Link or USB-Serial bootloader
```

---

## 🤝 Contributing

We welcome contributions! Whether it's:

- 🐛 **Bug reports** — Found an issue? Let us know
- 💡 **Feature requests** — Ideas for improvements
- 🔧 **Pull requests** — Code contributions
- 📝 **Documentation** — Help improve our docs
- 🧪 **Testing** — Try with different NAND chips

See [CONTRIBUTING.md](openflash/CONTRIBUTING.md) for guidelines.

### Development Setup

```bash
# Run with hot-reload
cd openflash/gui
npm install
cargo tauri dev

# Run tests
cargo test -p openflash-core
```

---

## 🗺️ Roadmap

### v1.1 (Q2 2026)
- [ ] SPI NAND support
- [ ] Improved BCH performance
- [ ] Batch operations

### v1.2 (Q3 2026)
- [ ] eMMC support
- [ ] Multi-device parallel dumping
- [ ] Plugin system

### Future
- [ ] Web-based analysis tools
- [ ] Hardware debugger integration
- [ ] Custom PCB designs

---

## 📄 License

OpenFlash is released under the **MIT License**. See [LICENSE](openflash/LICENSE) for details.

```
MIT License — Do whatever you want, just don't blame us.
```

---

## 🙏 Acknowledgments

- **[Embassy](https://embassy.dev/)** — Async embedded Rust framework
- **[Tauri](https://tauri.app/)** — Desktop app framework
- **[nusb](https://github.com/kevinmehall/nusb)** — Pure Rust USB library
- **Hardware hacking community** — For inspiration and testing

---

<p align="center">
  <strong>OpenFlash v1.0.0</strong><br>
  <em>Your data deserves to be free.</em>
</p>

<p align="center">
  <a href="https://github.com/openflash/openflash/stargazers">⭐ Star us on GitHub</a> •
  <a href="https://github.com/openflash/openflash/issues">🐛 Report Bug</a> •
  <a href="https://github.com/openflash/openflash/discussions">💬 Discussions</a>
</p>
