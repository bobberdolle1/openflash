# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.4.0] - 2026-01-XX

### Added
- **AI Analysis v1.4 - Major Upgrade**
  - **Filesystem Detection**: Auto-detect YAFFS2, UBIFS, JFFS2, SquashFS, CramFS, ext2/3/4, FAT16/32, NTFS, F2FS
  - **OOB/Spare Area Analysis**: Automatic ECC scheme detection (Hamming, BCH4-40, LDPC, Reed-Solomon)
  - **Encryption Key Search**: Deep scan for AES-128/192/256 keys with entropy analysis
  - **Dump Comparison**: Compare two dumps with diff analysis, similarity scoring, bit-flip detection
  - **Wear Leveling Analysis**: Estimate erase counts, identify hot/cold blocks, remaining life prediction
  - **Memory Map Generation**: Visual memory layout with partition detection
  - **Report Export**: Generate comprehensive Markdown analysis reports
  - New pattern types: BootLoader, Kernel, DeviceTree, ConfigData, OobData, WearLevelMeta

- **New AI GUI Features**
  - Filesystem detection tab with clickable navigation
  - OOB analysis panel with visual layout diagram
  - Encryption key candidates list with context
  - Wear analysis with circular life indicator and block heatmap
  - Interactive memory map visualization with legend
  - Export report button (📄) for saving analysis as Markdown
  - Flash life metric in summary card

- **New Tauri Commands**
  - `ai_compare_dumps` - Compare two dumps and find differences
  - `ai_search_keys` - Deep scan for encryption keys
  - `ai_generate_report` - Export analysis as Markdown report

### Changed
- AI analyzer now supports configurable OOB size via `with_oob()` builder
- AI analyzer supports deep scan mode via `with_deep_scan()` for thorough key search
- Extended `AiAnalysisResult` with filesystems, oob_analysis, key_candidates, wear_analysis, memory_map
- GUI version updated to 1.4.0
- Core library version updated to 1.4.0

## [1.3.0] - 2026-01-XX

### Added
- **AI-Powered Analysis Engine**
  - New `ai` module in core library with intelligent data analysis
  - Pattern recognition: encrypted, compressed, executable, text, empty, repeating data
  - Anomaly detection: bad blocks, bit rot, truncation, header corruption
  - Data recovery suggestions with success probability estimates
  - Chip-specific recommendations (ECC, timing, page size)
  - Data quality scoring and encryption/compression probability
  - Automatic summary generation

- **AI Analysis GUI**
  - New "🤖 AI" tab in desktop application
  - Interactive pattern visualization with click-to-navigate
  - Severity-coded anomaly display (Critical/Warning/Info)
  - Recovery action prioritization
  - Real-time analysis metrics (quality, encryption, compression)
  - Tabbed interface: Patterns, Issues, Recovery, Tips

- **New Tauri Commands**
  - `ai_analyze_dump` - Full AI analysis of dump data
  - `ai_detect_patterns` - Quick pattern detection
  - `ai_get_recommendations` - Chip-specific recommendations

### Changed
- GUI version updated to 1.3.0
- Core library version updated to 1.3.0

## [1.25.0] - 2026-01-XX

### Added
- **STM32F1 SPI NAND Support**
  - New `spi_nand` module for STM32F1 firmware
  - Hardware SPI peripheral support for high-speed communication
  - Full SPI NAND command set (READ_ID, PAGE_READ, PROGRAM, ERASE)
  - Internal ECC status reporting
  - Feature register access (protection, status, configuration)
  - Quad SPI enable support
  - Block unlock functionality

- **STM32F1 eMMC Support**
  - New `emmc` module for STM32F1 firmware
  - SPI mode eMMC/MMC communication
  - Card initialization with high-capacity detection
  - CID/CSD register reading
  - Single and multi-block read operations
  - Single block write operations
  - Block erase support
  - CRC7 command checksum calculation

### Changed
- STM32F1 firmware version updated to 1.25.0
- STM32F1 main.rs now includes spi_nand and emmc modules

## [1.2.0] - 2026-01-XX

### Added
- **eMMC Support**
  - New `emmc` module in core library with chip database
  - Support for eMMC chips (Samsung, Micron, SanDisk, Toshiba, Kingston)
  - MMC/SD protocol commands via SPI mode
  - CID/CSD/EXT_CSD register parsing
  - Block read/write operations (512 bytes)
  - Boot partition access support
  - CRC7/CRC16 calculation
  - eMMC driver for RP2040 firmware (SPI1 interface)
  - Updated documentation with eMMC wiring diagrams

### Changed
- Protocol commands extended with eMMC range (0x40-0x5F)
- FlashInterface enum now includes Emmc variant
- README updated with eMMC support information

## [1.1.0] - 2026-01-XX

### Added
- **SPI NAND Support**
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

- **1.4.0** - AI v1.4: Filesystem detection, OOB analysis, key search, wear analysis, memory map
- **1.3.0** - AI-powered analysis features
- **1.25.0** - STM32F1 SPI NAND & eMMC support
- **1.2.0** - eMMC support
- **1.1.0** - SPI NAND support
- **1.0.0** - Initial public release
- **0.x.x** - Development versions (not released)

[Unreleased]: https://github.com/openflash/openflash/compare/v1.4.0...HEAD
[1.4.0]: https://github.com/openflash/openflash/compare/v1.3.0...v1.4.0
[1.3.0]: https://github.com/openflash/openflash/compare/v1.25.0...v1.3.0
[1.25.0]: https://github.com/openflash/openflash/compare/v1.2.0...v1.25.0
[1.2.0]: https://github.com/openflash/openflash/compare/v1.1.0...v1.2.0
[1.1.0]: https://github.com/openflash/openflash/compare/v1.0.0...v1.1.0
[1.0.0]: https://github.com/openflash/openflash/releases/tag/v1.0.0
