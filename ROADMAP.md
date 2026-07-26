# OpenFlash Roadmap

## Текущий статус

**Работает от начала до конца:** SPI NOR через Raspberry Pi, Orange Pi или
Banana Pi с агентом OpenFlash — определение чипа, дамп, стирание, запись,
проверка. Плюс офлайн-анализ уже снятых дампов для всех пяти интерфейсов.

Всё остальное из списка ниже либо не собирается, либо не подключено ни к чему.
Подробности по платам — в [PLATFORMS.md](openflash/docs/PLATFORMS.md).

---

## Что было объявлено завершённым

Раньше здесь стоял заголовок «✅ Завершённые версии» и перечень из двадцати с
лишним пунктов. Значительная часть из них — код, который не собирается, не
вызывается ниоткуда или описывает несуществующее железо. Номер версии
проставлялся быстрее, чем работала функция.

Список сохранён, но с отметками о фактическом состоянии, потому что он полезен
как перечень намерений — и вреден как перечень достижений.

### v1.x — Foundation

| Объявлено | Фактически |
|---|---|
| v1.0: Parallel NAND, Hamming/BCH ECC, SquashFS/UBIFS/JFFS2 | парсеры ФС и ECC работают; **читать parallel NAND нечем** — прошивки нет. BCH до недавнего времени портил данные и был отключён, сейчас исправлен |
| v1.1: SPI NAND (QSPI) | база чипов и парсеры; прошивки нет |
| v1.2: eMMC support | база чипов, разбор CSD/EXT_CSD; прошивки нет |
| v1.3: AI-анализ | эвристики над дампом (энтропия, паттерны, аномалии) — работает; «AI» здесь означает статистику, а не модель |
| v1.4: Filesystem detection, OOB analysis, key search | работает на офлайн-дампах |
| v1.5: ESP32 & STM32F4 support | **не собирается**; у ESP32 к тому же своя нумерация опкодов |
| v1.6: SPI NOR, UFS, ONFI 5.0 | SPI NOR работает. UFS — только разбор дескрипторов и сборка SCSI CDB, устройства нет |
| v1.7: Write operations, bad block management, wear leveling, chip cloning | запись и проверка работают. **Bad block management отсутствует** — считается эвристика «блок из одних нулей», а не таблица из spare-области |
| v1.8: Python API, CLI, batch processing, plugins | Python API и CLI работают. `Batch.run` возбуждает исключение, системы плагинов нет |
| v1.9: ML chip identification, firmware unpacking, vulnerability scanning | распаковка и сканирование работают на дампах. «ML-идентификация» — сопоставление по таблице |

### v2.x — Scale & Hardware

| Объявлено | Фактически |
|---|---|
| v2.0: Multi-device, REST API, device farm | `core/src/server.rs` — 1975 строк структур; **у core нет HTTP-зависимости**, сервер ничего не слушает и ниоткуда не вызывается |
| v2.1: OpenFlash PCB, TSOP-48 адаптер, логический анализатор, JTAG/SWD | **платы не существует**: ни схемы, ни разводки, ни герберов в репозитории |
| v2.2: 150+ новых чипов | база реальна — 207 записей на четыре интерфейса |
| v2.3: RP2350, Arduino GIGA, Raspberry Pi SBC, Orange Pi (9 платформ) | из перечисленного работают Raspberry Pi и Orange Pi. RP2350 и Arduino GIGA — заготовки без таблицы команд |
| v2.3.5: Teensy 4.x (USB HS 480 Мбит/с), Banana Pi (11 платформ) | Banana Pi работает. У Teensy `usb.rs` — заглушка, `poll_command` всегда возвращает `None`, то есть команду принять нельзя. **Работают 3 платформы из 11** |

### v3.0 — Cloud & Pro

Объявлено: облачная синхронизация, командная работа, краудсорсинг базы чипов,
OTA-обновления моделей, тарифы Free/Pro/Enterprise.

Фактически: `core/src/cloud.rs` — 1021 строка структур данных. У core нет
HTTP-клиента, так что обратиться к сети этот код не может ни при каких условиях;
домен `api.openflash.io` не существует; ни CLI, ни GUI, ни Python-биндинги на
модуль не ссылаются. Опкоды облачных команд были удалены из протокола, поскольку
занимали значения, нужные под настоящие команды.

Ничего из этого не работает и не работало.

---

## 🚀 Будущие версии

### v3.1 — FPGA & High-Speed
**Цель:** Максимальная скорость и точность timing

| Фича | Описание |
|------|----------|
| FPGA programmer | Lattice iCE40/ECP5 для NV-DDR3/4 timing, 100+ MB/s |
| Tang Nano support | Sipeed Tang Nano 9K/20K — дешёвые FPGA ($15-30) |
| USB 3.0 bridge | FT601/FX3 для 300+ MB/s transfers |
| Parallel read optimization | Чтение нескольких страниц одновременно |
| DMA transfers | Zero-copy на всех платформах |

---

### v3.2 — Extended Flash Support
**Цель:** Поддержка всех типов flash памяти

| Фича | Описание |
|------|----------|
| OneNAND | Samsung KFM/KFN series (legacy devices) |
| HyperFlash | Cypress/Infineon S26KS/S26HL (automotive) |
| OctalSPI | Macronix MX25/MX66 OctaFlash |
| 3D NAND optimizations | Samsung V-NAND, Micron 3D TLC/QLC specific |
| QLC NAND | 4-bit per cell support с расширенным ECC |
| RPMB access | eMMC Replay Protected Memory Block |
| SD/microSD raw | Прямой доступ к raw NAND внутри SD карт |

---

### v3.3 — Forensics & Security
**Цель:** Профессиональные инструменты для forensics

| Фича | Описание |
|------|----------|
| Write-blocker mode | Hardware write protection, гарантированный read-only |
| Chain of custody | Криптографическое подтверждение целостности |
| Court-ready reports | PDF отчёты с hash verification для суда |
| Audit logging | Полный лог операций с timestamps и signatures |
| Encrypted storage | AES-256 шифрование дампов at rest |
| Data carving | Восстановление удалённых файлов из raw dumps |
| Timeline reconstruction | Временная шкала изменений на основе FS metadata |

---

### v3.4 — AI & Analysis v2
**Цель:** Продвинутый AI-анализ

| Фича | Описание |
|------|----------|
| Firmware similarity | Fuzzy hashing (TLSH/ssdeep) для поиска похожих прошивок |
| Backdoor detection | ML-детекция известных backdoor паттернов |
| Crypto key extraction | Автоматический поиск RSA/EC ключей, сертификатов |
| Bootloader analysis | U-Boot, Barebox, custom bootloader parsing |
| Device tree extraction | Автоматический парсинг DTB/FDT |
| Symbol recovery | Восстановление символов из stripped binaries |
| Diff analysis v2 | Semantic diff между версиями firmware |

---

### v3.5 — Developer Tools
**Цель:** Интеграция в workflow разработчиков

| Фича | Описание |
|------|----------|
| VS Code extension | Hex view, analysis, flash operations из IDE |
| GitHub Actions | CI/CD action для firmware verification |
| GitLab CI template | Готовый pipeline для embedded проектов |
| Rust crate (crates.io) | openflash-core как библиотека |
| C/C++ bindings | FFI для embedded toolchains |
| GDB integration | Чтение flash через GDB remote protocol |
| OpenOCD plugin | Интеграция с OpenOCD для debug + flash |

---

### v3.6 — RISC-V & New Platforms
**Цель:** Поддержка RISC-V и новых MCU

| Фича | Описание |
|------|----------|
| ESP32-C3/C6 | RISC-V варианты ESP32 |
| CH32V series | WCH CH32V103/203/303 — дешёвые RISC-V ($0.50-2) |
| GD32VF103 | GigaDevice RISC-V (совместим с STM32F103) |
| BL602/BL616 | Bouffalo Lab WiFi+BLE RISC-V |
| Milk-V Duo | RISC-V SBC ($9) |
| LicheePi 4A | TH1520 RISC-V SBC |
| BeagleV | StarFive RISC-V |

---

### v3.7 — Enterprise Scale
**Цель:** Масштабирование для production

| Фича | Описание |
|------|----------|
| Kubernetes operator | Auto-scaling device farm в k8s |
| Prometheus metrics | Мониторинг производительности |
| Grafana dashboards | Визуализация статистики |
| LDAP/SAML auth | Enterprise SSO |
| Multi-region cloud | Geo-distributed infrastructure |
| On-premise deploy | Self-hosted OpenFlash Cloud |
| Compliance (SOC2) | Сертификация для enterprise |

---

### v4.0 — Next Generation
**Цель:** Архитектурные улучшения

| Фича | Описание |
|------|----------|
| WebAssembly core | Анализ дампов в браузере без установки |
| Distributed dumping | Параллельное чтение одного чипа несколькими устройствами |
| Real-time collab | Совместный анализ как Google Docs |
| Plugin sandbox | WASM-изолированные плагины |
| Custom protocols | DSL для описания новых flash протоколов |
| Hardware abstraction | Унифицированный HAL для всех платформ |

---

## 🔧 Технический долг

| Область | Задачи |
|---------|--------|
| Performance | SIMD для ECC, async I/O везде, memory-mapped files |
| Testing | 90%+ coverage, hardware-in-the-loop tests, fuzzing |
| Documentation | API reference, video tutorials, cookbook |
| Code quality | Clippy pedantic, безопасный unsafe, no panics |

---

## 📊 Chip Database Goals

| Тип | Текущее | Цель v4.0 |
|-----|---------|-----------|
| Parallel NAND | 60+ | 150+ |
| SPI NAND | 55+ | 120+ |
| SPI NOR | 75+ | 200+ |
| eMMC | 40+ | 80+ |
| UFS | 10+ | 30+ |
| OneNAND | 0 | 20+ |
| HyperFlash | 0 | 15+ |

---

## 🎯 Приоритеты

### Ближайшие — то, что действительно блокирует проект

Всё, что ниже в разделе «будущие версии», предполагает работающий программатор.
Сейчас его нет ни на одном микроконтроллере, поэтому порядок такой:

1. **Поднять хотя бы одну прошивку для микроконтроллера.** Всем семи нужны
   USB-стек, диспетчер с фреймингом и настройка целевой платформы. Это
   единственное, что отделяет проект от заявленной идеи «программатор из платы
   за $4».
2. **Тесты на реальном железе в CI.** Ни один чип из базы никогда не
   проверялся на физическом устройстве. Self-hosted runner с Pico и W25Q
   доказывал бы, что чтение действительно работает.
3. **Вынести базу чипов из `match`-веток в таблицы.** Сейчас 207 записей можно
   запросить по ID, но нельзя перечислить, из-за чего `openflash chips` не умеет
   выводить список.
4. **Адресные циклы для parallel NAND** в агенте SBC — каркас есть, операции
   намеренно отказывают.
5. **Раскладки BCH конкретных контроллеров** — сам кодек корректен, но у
   железного контроллера свой порядок бит и своё размещение в spare-области.

### Долгосрочные направления

1. **Скорость** — FPGA и USB 3.0 для 100+ MB/s
2. **Покрытие чипов** — максимум поддерживаемых устройств
3. **Forensics** — профессиональные инструменты
4. **AI** — умный анализ без ручной работы
5. **Интеграции** — встраивание в существующие workflow

---

## Правило для этого файла

Версия не считается завершённой, пока её функция не работает и не покрыта
тестом. Раздел «что было объявлено завершённым» выше — результат нарушения этого
правила: номера проставлялись под написанный код, а не под работающий.

Если функция описана здесь как готовая, у неё должен быть тест в CI или
инструкция, по которой её можно воспроизвести на железе.
