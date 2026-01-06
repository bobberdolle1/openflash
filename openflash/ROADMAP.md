# OpenFlash Roadmap

Детальный план развития проекта OpenFlash.

## Текущий статус: v1.9.0

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
- GUI: SPI NOR operations panel, UFS LUN selector

### v1.7 — Advanced Write Operations
- Full chip programming с верификацией
- Bad block management (автоматическое управление)
- Wear leveling write (отслеживание износа)
- Incremental backup/restore (только изменённые блоки)
- Clone chip-to-chip (клонирование между чипами)
- 12 новых протокольных команд (0xA0-0xAB)
- 15 unit тестов для write_ops модуля

### v1.8 — Scripting & Automation
- **Python API (pyopenflash)** — полноценные Python bindings через PyO3
- **CLI tool (openflash)** — командная строка с clap
- **Batch processing** — очередь задач с зависимостями
- **Custom analysis plugins** — система плагинов с хуками
- **CI/CD integration** — типы для автоматизации
- 12 новых протокольных команд (0xB0-0xBB)
- 20+ unit тестов для scripting модуля

**Python API пример:**
```python
import openflash

device = openflash.connect()
dump = device.read_full()
analysis = openflash.ai.analyze(dump)
analysis.export_report("report.md")
```

**CLI примеры:**
```bash
openflash scan                    # Поиск устройств
openflash detect                  # Определение чипа
openflash read -o dump.bin        # Чтение дампа
openflash write -i firmware.bin   # Запись прошивки
openflash analyze dump.bin        # AI анализ
openflash batch jobs.toml         # Пакетная обработка
```

### v1.9 — Advanced AI Features ← ТЕКУЩАЯ
**Цель:** ML и глубокий анализ

| Фича | Приоритет | Сложность |
|------|-----------|-----------|
| ML-based chip identification | 🟡 Medium | High |
| Firmware unpacking (binwalk) | 🔴 High | Medium |
| Automatic rootfs extraction | 🔴 High | High |
| Vulnerability scanning | 🟡 Medium | High |
| Custom signature database | � LowM | Medium |

**Детали:**
- Интеграция с binwalk для распаковки
- Автоматическое извлечение файловых систем
- База сигнатур уязвимостей (CVE)
- Пользовательские сигнатуры для поиска
- 10 новых протокольных команд (0xC0-0xC9)
- 15+ unit тестов для ai_advanced модуля

**CLI примеры:**
```bash
openflash unpack dump.bin -o extracted/   # Распаковка прошивки
openflash rootfs dump.bin -o rootfs/      # Извлечение rootfs
openflash vulnscan dump.bin               # Сканирование уязвимостей
openflash identify dump.bin               # ML идентификация чипа
```

---

## 🚀 Мажорные релизы

### v2.0 — Multi-device & Enterprise
**Цель:** Масштабирование и профессиональное использование

| Фича | Приоритет | Сложность |
|------|-----------|-----------|
| Multi-device parallel dumping | 🔴 High | High |
| Device farm management | 🟡 Medium | High |
| Remote operation (server mode) | 🟡 Medium | Medium |
| Production line integration | 🟢 Low | High |
| REST API | 🔴 High | Medium |

**Архитектура:**
```
┌─────────────────────────────────────────────────────────┐
│  OpenFlash Server                                       │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐     │
│  │  REST API   │  │  WebSocket  │  │  gRPC       │     │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘     │
│         └────────────────┼────────────────┘            │
│                          ▼                              │
│  ┌─────────────────────────────────────────────────┐   │
│  │  Device Manager                                  │   │
│  │  ├── Device Pool                                │   │
│  │  ├── Job Queue                                  │   │
│  │  └── Result Aggregator                          │   │
│  └─────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────┘
         │              │              │
    ┌────▼────┐    ┌────▼────┐    ┌────▼────┐
    │ Device 1│    │ Device 2│    │ Device N│
    │ (RP2040)│    │ (ESP32) │    │(STM32F4)│
    └─────────┘    └─────────┘    └─────────┘
```

---

### v2.1 — Hardware Expansion
**Цель:** Официальное железо

| Фича | Приоритет | Сложность |
|------|-----------|-----------|
| Official OpenFlash PCB | 🔴 High | High |
| TSOP-48 ZIF adapter board | 🔴 High | Medium |
| BGA rework station integration | 🟢 Low | High |
| Logic analyzer mode | 🟡 Medium | Medium |
| JTAG/SWD passthrough | 🟢 Low | Medium |

**OpenFlash PCB v1:**
- RP2040 + ESP32 combo
- TSOP-48 ZIF socket
- SPI NAND/NOR socket
- eMMC socket
- USB-C + WiFi
- OLED display
- ~$25 BOM

---

### v3.0 — OpenFlash Pro
**Цель:** Коммерческая версия

| Фича | Приоритет | Сложность |
|------|-----------|-----------|
| Cloud sync & backup | 🟡 Medium | High |
| Team collaboration | 🟡 Medium | High |
| Chip database crowdsourcing | 🔴 High | Medium |
| AI model updates OTA | 🟡 Medium | Medium |
| Enterprise support | 🟢 Low | Low |

---

## 📊 Приоритеты по категориям

### Hardware Support
1. SPI NOR flash (v1.6)
2. 16-bit NAND (v1.6)
3. UFS (v1.6)
4. Official PCB (v2.1)

### Software Features
1. Python API (v1.8)
2. CLI tool (v1.8)
3. Firmware unpacking (v1.9)
4. REST API (v2.0)

### AI/Analysis
1. Rootfs extraction (v1.9)
2. Vulnerability scanning (v1.9)
3. ML chip identification (v1.9)
4. Cloud AI updates (v3.0)

### Enterprise
1. Multi-device (v2.0)
2. Server mode (v2.0)
3. Team features (v3.0)

---

## 🗓️ Примерный таймлайн

| Версия | Ожидаемая дата | Статус |
|--------|----------------|--------|
| v1.5 | Q1 2026 | ✅ Released |
| v1.6 | Q1 2026 | ✅ Released |
| v1.7 | Q2 2026 | ✅ Released |
| v1.8 | Q2 2026 | ✅ Released |
| v1.9 | Q3 2026 | ✅ Released |
| v2.0 | Q4 2026 | 📋 Planned |
| v2.1 | Q1 2027 | 📋 Planned |
| v3.0 | 2028 | 🔮 Future |

---

## 💡 Хотите предложить фичу?

1. Проверьте [Issues](https://github.com/openflash/openflash/issues)
2. Создайте Feature Request
3. Обсудите в [Discussions](https://github.com/openflash/openflash/discussions)

---

*Последнее обновление: Январь 2026*
