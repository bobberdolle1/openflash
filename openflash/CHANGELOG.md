# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.9.0] - 2026-Q3

### Added
- **Advanced AI Features**
  - New `ai_advanced` module in core library for ML and deep analysis
  - **ML-based Chip Identification**: Neural network model for chip recognition
    - Feature extraction from dump patterns
    - Confidence scoring with top-3 predictions
    - Training data from 500+ chip signatures
  
  - **Firmware Unpacking (binwalk integration)**
    - Automatic firmware structure detection
    - Support for 50+ compression/archive formats
    - Recursive extraction with depth limit
    - Entropy-based section identification
  
  - **Automatic Rootfs Extraction**
    - SquashFS, UBIFS, JFFS2, CramFS, ext2/3/4 extraction
    - Automatic mount point detection
    - File permission and ownership preservation
    - Symbolic link handling
  
  - **Vulnerability Scanning**
    - CVE database integration (NVD)
    - Binary pattern matching for known vulnerabilities
    - Hardcoded credential detection
    - Weak crypto algorithm identification
    - CVSS scoring with severity levels
  
  - **Custom Signature Database**
    - User-defined signature format (YAML)
    - Pattern types: hex, regex, entropy-based
    - Signature categories: malware, backdoor, debug, config
    - Import/export functionality

- **New Protocol Commands (0xC0-0xC9)**
  - `MlIdentify` (0xC0) - ML-based chip identification
  - `UnpackFirmware` (0xC1) - Start firmware unpacking
  - `UnpackStatus` (0xC2) - Get unpacking progress
  - `ExtractRootfs` (0xC3) - Extract root filesystem
  - `VulnScan` (0xC4) - Start vulnerability scan
  - `VulnResults` (0xC5) - Get vulnerability results
  - `LoadSignatures` (0xC6) - Load custom signatures
  - `ScanSignatures` (0xC7) - Scan with custom signatures
  - `ExportSignatures` (0xC8) - Export signature database
  - `GetMlModel` (0xC9) - Get ML model info

- **New CLI Commands**
  - `openflash unpack <dump> -o <dir>` - Unpack firmware
  - `openflash rootfs <dump> -o <dir>` - Extract rootfs
  - `openflash vulnscan <dump>` - Scan for vulnerabilities
  - `openflash identify <dump>` - ML chip identification
  - `openflash signatures load <file>` - Load custom signatures
  - `openflash signatures scan <dump>` - Scan with signatures

- **New Types and Structures**
  - `MlChipIdentifier`, `ChipPrediction`, `FeatureVector` - ML identification
  - `FirmwareUnpacker`, `UnpackResult`, `ExtractedSection` - Unpacking
  - `RootfsExtractor`, `ExtractedFile`, `FilesystemType` - Rootfs extraction
  - `VulnScanner`, `Vulnerability`, `CvssScore`, `Severity` - Vulnerability scanning
  - `SignatureDatabase`, `CustomSignature`, `SignatureMatch` - Custom signatures

### Changed
- Protocol version updated to 0x19
- Core library version updated to 1.9.0
- CLI version: 1.9.0
- pyopenflash version: 1.9.0
- Added `is_ai_advanced()` method to Command enum
- Extended lib.rs exports with ai_advanced types

### Tests
- 15+ new unit tests for ai_advanced module
- ML identifier tests with mock model
- Firmware unpacker tests
- Rootfs extractor tests
- Vulnerability scanner tests
- Signature database tests

## [1.8.0] - 2026-Q2

### Added
- **Scripting & Automation**
  - New `scripting` module in core library for automation support
  - **Python API (pyopenflash)**: Full Python bindings via PyO3
    - `openflash.connect()` - Connect to device
    - `device.detect()` - Detect chip
    - `device.read_full()` / `device.read()` - Read operations
    - `device.write()` - Write operations
    - `openflash.ai.analyze()` - AI analysis
    - `analysis.export_report()` - Export reports
    - `openflash.Batch` - Batch processing
    - `openflash.load_dump()` / `openflash.compare_dumps()` - Utilities
  
  - **CLI Tool (openflash)**: Full-featured command-line interface
    - `openflash scan` - Scan for devices
    - `openflash detect` - Detect chip
    - `openflash read -o dump.bin` - Read chip
    - `openflash write -i firmware.bin` - Write chip
    - `openflash erase` - Erase chip
    - `openflash verify -f file.bin` - Verify contents
    - `openflash analyze dump.bin` - AI analysis
    - `openflash compare file1.bin file2.bin` - Compare dumps
    - `openflash clone` - Chip-to-chip clone
    - `openflash batch jobs.toml` - Batch processing
    - `openflash script script.py` - Run scripts
    - `openflash chips` - List supported chips
    - JSON/CSV/Table output formats
    - Progress bars and colored output
  
  - **Batch Processing**: Job queue with dependencies
    - `BatchProcessor` - Job queue manager
    - `BatchJob` - Job definition with dependencies
    - Read, Write, Erase, Verify, Analyze, Clone, Report job types
    - Stop-on-error and parallel execution modes
  
  - **Plugin System**: Extensible analysis plugins
    - `PluginManager` - Plugin lifecycle management
    - `PluginMetadata` - Plugin information
    - `PluginHook` - Pre/Post read/write, Analysis, Pattern/FS detection
    - `PluginContext` / `PluginResult` - Plugin I/O
  
  - **CI/CD Integration**: Types for automation pipelines
    - `CiJobConfig` - CI job configuration
    - `CiOperation` - Verify, Read, Write, Analyze, Compare operations
    - `CiArtifact` - Output artifacts (dumps, reports, logs)
    - `CiJobResult` / `CiOperationResult` - Execution results

- **New Protocol Commands (0xB0-0xBB)**
  - `BatchStart` (0xB0) - Start batch operation
  - `BatchStatus` (0xB1) - Get batch status
  - `BatchAbort` (0xB2) - Abort batch
  - `ScriptLoad` (0xB3) - Load script to device
  - `ScriptRun` (0xB4) - Run loaded script
  - `ScriptStatus` (0xB5) - Get script status
  - `PluginList` (0xB6) - List plugins
  - `PluginLoad` (0xB7) - Load plugin
  - `PluginUnload` (0xB8) - Unload plugin
  - `RemoteConnect` (0xB9) - Remote connection
  - `RemoteDisconnect` (0xBA) - Disconnect remote
  - `GetDeviceInfo` (0xBB) - Get device info

- **New Types and Structures**
  - `ScriptError`, `ScriptResult` - Error handling
  - `ConnectionConfig`, `DeviceInfo`, `DeviceHandle` - Device connection
  - `ReadOptions`, `WriteOptions`, `DumpResult`, `ReadStats` - I/O options
  - `ChipDetectionResult` - Chip detection
  - `AnalysisOptions`, `ScriptAnalysisResult` - Analysis configuration
  - `PatternInfo`, `FilesystemInfo`, `AnomalyInfo`, `KeyCandidate` - Analysis results
  - `ReportFormat`, `ReportOptions` - Report export
  - `CliCommand`, `CliConfig`, `CliOutputFormat` - CLI types
  - `OpenFlash` - High-level API class

### Changed
- Protocol version updated to 0x18
- Core library version updated to 1.8.0
- CLI version: 1.8.0
- pyopenflash version: 1.8.0
- Added `is_scripting()` method to Command enum
- Extended lib.rs exports with scripting types
- Added serde_json dependency to core

### Tests
- 20+ new unit tests for scripting module
- Connection and device handle tests
- Batch processor tests
- Plugin manager tests
- Read/Write/Analysis options tests
- CLI config tests
- CI job config tests

## [1.7.0] - 2026-Q2

### Added
- **Advanced Write Operations**
  - New `write_ops` module in core library for full chip programming
  - **Full Chip Programming**: Complete chip write with automatic verification
  - **Bad Block Management**: Automatic bad block detection, tracking, and remapping
    - Factory bad block scanning from OOB markers
    - Runtime bad block detection (erase/program failures)
    - Spare block allocation for transparent remapping
    - Bad block table (BBT) persistence
  - **Wear Leveling**: Intelligent write distribution for extended chip life
    - Per-block erase count tracking
    - Wear statistics and remaining life estimation
    - Hot/cold block identification
    - Automatic wear leveling candidates selection
  - **Incremental Backup/Restore**: Efficient backup of only changed blocks
    - Block-level change tracking with checksums (FNV-1a)
    - Full and incremental backup metadata
    - Parent-child backup chain support
  - **Chip-to-Chip Cloning**: Direct clone between compatible chips
    - Exact, skip-bad-blocks, and wear-aware clone modes
    - Automatic block mapping with bad block handling
    - Progress tracking with ETA estimation

- **New Protocol Commands (0xA0-0xAB)**
  - `FullChipProgram` (0xA0) - Full chip programming with verify
  - `ReadBadBlockTable` (0xA1) - Read bad block table
  - `WriteBadBlockTable` (0xA2) - Write bad block table
  - `ScanBadBlocks` (0xA3) - Scan for bad blocks
  - `MarkBadBlock` (0xA4) - Mark block as bad
  - `GetWearInfo` (0xA5) - Get wear leveling info
  - `ProgramWithVerify` (0xA6) - Program page with verification
  - `EraseWithVerify` (0xA7) - Erase block with verification
  - `IncrementalRead` (0xA8) - Read only changed blocks
  - `CloneStart` (0xA9) - Start chip-to-chip clone
  - `CloneStatus` (0xAA) - Get clone operation status
  - `CloneAbort` (0xAB) - Abort clone operation

- **New Types and Structures**
  - `WriteError` - Comprehensive error types for write operations
  - `BadBlockTable`, `BadBlockEntry`, `BadBlockReason` - BBT management
  - `WearLevelingManager`, `BlockWearInfo`, `WearStatistics` - Wear tracking
  - `ChipProgrammer`, `ProgramOptions`, `ProgramProgress` - Programming control
  - `ChangeTracker`, `BackupMetadata` - Incremental backup support
  - `ChipCloner`, `CloneOptions`, `CloneMode`, `CloneProgress` - Cloning support

### Changed
- Protocol version updated to 0x17
- Core library version updated to 1.7.0
- Added `is_write_ops()` method to Command enum
- Extended lib.rs exports with write_ops types

### Tests
- 15 new unit tests for write operations module
- Bad block table creation and management tests
- Wear leveling tracking and limit tests
- Change tracker and checksum tests
- Chip programmer address conversion tests
- Clone compatibility and block mapping tests

## [1.6.0] - 2026-01-XX

### Added
- **SPI NOR Flash Support**
  - New `spi_nor` module in core library with chip database
  - Support for 30+ SPI NOR chips: W25Q series (Winbond), MX25L series (Macronix), IS25LP series (ISSI)
  - JEDEC ID auto-detection (command 0x9F)
  - SFDP (Serial Flash Discoverable Parameters) parsing for extended chip info
  - Standard read (0x03), fast read (0x0B), dual read (0x3B), quad read (0x6B)
  - Page program (0x02), sector erase (0x20), block erase (0xD8), chip erase (0xC7)
  - Status register read/write with protection bit decoding (BP0-BP4, TB, SEC, CMP)
  - 3-byte and 4-byte addressing modes for chips > 16MB
  - SPI NOR firmware for all platforms: RP2040, STM32F1, STM32F4, ESP32

- **UFS (Universal Flash Storage) Support**
  - New `ufs` module in core library
  - UFS 2.0, 2.1, 3.0, 3.1, 4.0 version detection
  - Device, Unit, and Geometry descriptor parsing
  - SCSI command builders: READ(10), READ(16) for addresses beyond 2TB
  - LUN (Logical Unit) support: UserData, BootA, BootB, RPMB
  - Sense data decoding for error handling
  - Manufacturer database (Samsung, SK Hynix, Micron, Toshiba/Kioxia, Western Digital)

- **ONFI 5.0 Support**
  - Extended ONFI module for version 5.0 compliance
  - NV-DDR3 timing parameters (up to 1.6GT/s)
  - Extended ECC information parsing
  - ZQ calibration and DCC training feature flags
  - Multi-plane operation support

- **16-bit Parallel NAND Bus Support**
  - NandBusWidth enum (X8, X16)
  - Bus width auto-detection
  - 16-bit byte swapping functions with endianness support
  - x16 chip variants in database (Samsung, Micron, Hynix)

- **New GUI Components**
  - SPI NOR operations panel with protection status display
  - Sector/block/chip erase buttons with address input
  - UFS LUN selector for choosing target logical unit
  - Updated interface selector (Parallel NAND, SPI NAND, SPI NOR, eMMC, UFS)

- **Protocol Extensions**
  - SPI NOR commands (0x60-0x7F range)
  - UFS commands (0x80-0x9F range)
  - FlashInterface::SpiNor, FlashInterface::Ufs, FlashInterface::ParallelNand16 variants

- **Property-Based Tests**
  - 10 new property tests with proptest crate (100+ iterations each)
  - JEDEC ID lookup consistency
  - SFDP parsing round-trip
  - Status register bit decoding
  - UFS descriptor parsing
  - SCSI command building
  - Sense data decoding
  - ONFI version detection
  - ONFI 5.0 ECC parsing
  - 16-bit byte swapping correctness

### Changed
- Protocol version updated to 0x16
- Core library version updated to 1.6.0
- GUI version updated to 1.6.0
- Total test count: 131 tests in core library

### Documentation
- Updated Supported-Chips wiki with SPI NOR chips
- Updated ROADMAP with v1.6 completion
- Added SPI NOR and UFS sections to documentation

## [1.5.0] - 2026-01-XX

### Added
- **ESP32 Support**
  - New `esp32` firmware module for ESP32 series microcontrollers
  - Support for ESP32, ESP32-S2, ESP32-S3, ESP32-C3
  - WiFi/BLE connectivity for wireless flash operations
  - Web server mode for browser-based control
  - UART and USB Serial/JTAG communication
  - Full support for Parallel NAND, SPI NAND, and eMMC

- **STM32F4 Support**
  - New `stm32f4` firmware module for STM32F4 series
  - Support for STM32F401, STM32F411, STM32F446
  - Native USB OTG FS for faster transfers
  - FSMC-based parallel NAND for high-speed operations
  - DMA support for SPI NAND and eMMC
  - Higher clock speeds (up to 180MHz)

- **New ESP32 Commands**
  - `wifi_scan` - Scan for available WiFi networks
  - `wifi_connect` - Connect to WiFi network
  - `wifi_status` - Get WiFi connection status
  - `start_web_server` - Start web-based control interface
  - `stop_web_server` - Stop web server

### Changed
- Protocol version updated to 0x15
- Core library version updated to 1.5.0
- GUI version updated to 1.5.0
- Firmware versions updated to 1.5.0

### Documentation
- Added ESP32 wiring diagrams
- Added STM32F4 wiring diagrams
- Updated Supported-Chips wiki with new platforms
- Updated Hardware-Setup wiki with ESP32/STM32F4 sections

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

- **1.9.0** - Advanced AI Features: ML chip identification, firmware unpacking, rootfs extraction, vulnerability scanning
- **1.8.0** - Scripting & Automation: Python API, CLI tool, batch processing, plugins, CI/CD
- **1.7.0** - Advanced write operations, bad block management, wear leveling, incremental backup, cloning
- **1.6.0** - SPI NOR flash, UFS, ONFI 5.0, 16-bit NAND support
- **1.5.0** - ESP32 support, STM32F4 support, WiFi connectivity
- **1.4.0** - AI v1.4: Filesystem detection, OOB analysis, key search, wear analysis, memory map
- **1.3.0** - AI-powered analysis features
- **1.25.0** - STM32F1 SPI NAND & eMMC support
- **1.2.0** - eMMC support
- **1.1.0** - SPI NAND support
- **1.0.0** - Initial public release
- **0.x.x** - Development versions (not released)

[Unreleased]: https://github.com/openflash/openflash/compare/v1.9.0...HEAD
[1.9.0]: https://github.com/openflash/openflash/compare/v1.8.0...v1.9.0
[1.8.0]: https://github.com/openflash/openflash/compare/v1.7.0...v1.8.0
[1.7.0]: https://github.com/openflash/openflash/compare/v1.6.0...v1.7.0
[1.6.0]: https://github.com/openflash/openflash/compare/v1.5.0...v1.6.0
[1.5.0]: https://github.com/openflash/openflash/compare/v1.4.0...v1.5.0
[1.4.0]: https://github.com/openflash/openflash/compare/v1.3.0...v1.4.0
[1.3.0]: https://github.com/openflash/openflash/compare/v1.25.0...v1.3.0
[1.25.0]: https://github.com/openflash/openflash/compare/v1.2.0...v1.25.0
[1.2.0]: https://github.com/openflash/openflash/compare/v1.1.0...v1.2.0
[1.1.0]: https://github.com/openflash/openflash/compare/v1.0.0...v1.1.0
[1.0.0]: https://github.com/openflash/openflash/releases/tag/v1.0.0
