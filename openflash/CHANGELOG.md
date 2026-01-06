# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- **SPI NAND Support** (v1.1 feature)
  - New `spi_nand` module in core library with chip database
  - Support for 20+ SPI NAND chips (GigaDevice, Winbond, Macronix, Micron, Toshiba, XTX)
  - SPI NAND protocol commands (READ_ID, PAGE_READ, PROGRAM, ERASE)
  - Internal ECC status reporting
  - Quad SPI (QSPI) support for faster transfers
  - SPI NAND driver for RP2040 firmware
  - Interface selector in GUI (Parallel/SPI toggle)
  - Updated documentation with SPI NAND wiring diagrams

### Changed
- Protocol commands reorganized with dedicated ranges for Parallel NAND (0x10-0x1F) and SPI NAND (0x20-0x3F)
- ChipInfo now includes interface type field
- DeviceManager tracks current interface mode

## [1.0.0] - 2026-01-XX

### Added

#### Core Library
- ONFI chip database with 30+ supported NAND flash chips
- Hamming ECC algorithm for single-bit error correction
- BCH ECC algorithm with GF(2^13) arithmetic for multi-bit correction
- Filesystem signature detection (SquashFS, UBIFS, JFFS2, U-Boot, gzip, LZMA, XZ)
- Entropy-based data analysis
- USB protocol definitions for host-device communication

#### Desktop Application
- Modern dark theme GUI with glassmorphism effects
- Device scanning and connection management
- Mock device for testing without hardware
- NAND dump operations with progress tracking
- Interactive hex viewer with search and navigation
- Bitmap visualization with entropy coloring
- Automatic filesystem analysis
- File save/load with recent files tracking
- Configuration persistence
- Cross-platform support (Windows, macOS, Linux)

#### Firmware
- RP2040 (Raspberry Pi Pico) support
  - USB CDC communication
  - GPIO bit-bang NAND interface
  - Full NAND operations (read, write, erase)
- STM32F103 (Blue Pill) support
  - USB CDC communication
  - GPIO NAND interface

#### Infrastructure
- GitHub Actions CI/CD pipeline
- Automated testing for core library
- Multi-platform release builds
- Comprehensive documentation

### Security
- Input validation on all USB commands
- Safe memory handling in firmware
- No arbitrary code execution paths

---

## Version History

- **1.0.0** - Initial public release
- **0.x.x** - Development versions (not released)

[Unreleased]: https://github.com/openflash/openflash/compare/v1.0.0...HEAD
[1.0.0]: https://github.com/openflash/openflash/releases/tag/v1.0.0
