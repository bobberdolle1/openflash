# OpenFlash Roadmap

Детальный план развития проекта OpenFlash.

## Текущий статус: v2.1.0

---

## ✅ Завершённые версии

### v1.0 — Initial Release
- Parallel NAND read/write
- 30+ чипов в базе данных
- Hamming + BCH ECC
- Детекция SquashFS/UBIFS/JFFS2

### v1.1 — SPI NAND Support
- 20+ SPI NAND чипов
- Quad SPI (QSPI) поддержка
- Internal ECC статус
- Всего 4 провода!

### v1.2 — eMMC Support
- eMMC/MMC через SPI mode
- CID/CSD/EXT_CSD регистры
- Block read/write операции
- Boot partition доступ

### v1.25 — STM32F1 Expansion
- SPI NAND для Blue Pill
- eMMC для Blue Pill
- Полный паритет с RP2040

### v1.3 — AI-Powered Analysis
- Pattern recognition
- Anomaly detection
- Recovery suggestions
- Chip-specific recommendations

### v1.4 — AI Analysis v1.4
- Filesystem detection (YAFFS2, UBIFS, ext4, FAT...)
- OOB/spare area analysis
- Encryption key search (AES-128/192/256)
- Wear leveling analysis
- Memory map visualization
- Dump comparison
- Report export


### v1.5 — ESP32 & STM32F4 Support
- ESP32 firmware с WiFi/BLE
- STM32F4 firmware (USB OTG, FSMC)
- Web interface для ESP32
- 4 платформы: RP2040, STM32F1, STM32F4, ESP32

### v1.6 — NOR Flash & UFS Support
- SPI NOR flash (W25Q, MX25L, IS25LP) — 30+ чипов
- UFS (Universal Flash Storage) — версии 2.0-4.0
- ONFI 5.0 support с NV-DDR3
- 16-bit parallel NAND bus
- 10 property-based тестов

### v1.7 — Advanced Write Operations
- Full chip programming с верификацией
- Bad block management
- Wear leveling write
- Incremental backup/restore
- Clone chip-to-chip
- 12 новых протокольных команд (0xA0-0xAB)

### v1.8 — Scripting & Automation
- Python API (pyopenflash) через PyO3
- CLI tool (openflash) с clap
- Batch processing
- Custom analysis plugins
- CI/CD integration
- 12 новых протокольных команд (0xB0-0xBB)

### v1.9 — Advanced AI Features
- ML-based chip identification
- Firmware unpacking (binwalk)
- Automatic rootfs extraction
- Vulnerability scanning
- Custom signature database
- 10 новых протокольных команд (0xC0-0xC9)

### v2.0 — Multi-device & Enterprise
- Multi-device parallel dumping
- Device farm management
- Remote operation (server mode)
- Production line integration
- REST API
- 16 новых протокольных команд (0xD0-0xDF)

### v2.1 — Hardware Expansion ← ТЕКУЩАЯ
**Статус:** ✅ Released

| Фича | Статус |
|------|--------|
| Official OpenFlash PCB | ✅ Done |
| TSOP-48 ZIF adapter board | ✅ Done |
| BGA rework station integration | ✅ Done |
| Logic analyzer mode | ✅ Done |
| JTAG/SWD passthrough | ✅ Done |

**OpenFlash PCB v1:**
- RP2040 + ESP32 combo
- TSOP-48 ZIF socket
- SPI NAND/NOR socket (SOP-8)
- eMMC socket
- USB-C + WiFi
- OLED display (128x64)
- ~$25 BOM

**Реализация:**
- Новый модуль `hardware` в core library
- 16 новых протокольных команд (0xE0-0xEF)
- 14 unit тестов для hardware модуля
- TSOP-48 pinout для Samsung, Hynix, Micron, Toshiba
- Logic analyzer до 24 MHz с VCD/Sigrok экспортом
- JTAG chain scanning и SWD debug interface

---

## 🚀 Будущие релизы

### v3.0 — OpenFlash Pro
**Цель:** Коммерческая версия

| Фича | Приоритет |
|------|-----------|
| Cloud sync & backup | 🟡 Medium |
| Team collaboration | 🟡 Medium |
| Chip database crowdsourcing | 🔴 High |
| AI model updates OTA | 🟡 Medium |
| Enterprise support | 🟢 Low |

---

## 🗓️ Таймлайн

| Версия | Дата | Статус |
|--------|------|--------|
| v1.5 | Q1 2026 | ✅ Released |
| v1.6 | Q1 2026 | ✅ Released |
| v1.7 | Q2 2026 | ✅ Released |
| v1.8 | Q2 2026 | ✅ Released |
| v1.9 | Q3 2026 | ✅ Released |
| v2.0 | Q4 2026 | ✅ Released |
| v2.1 | Q1 2027 | ✅ Released |
| v3.0 | 2028 | 🔮 Future |

---

*Последнее обновление: Январь 2027*
