<div align="center">

# OpenFlash

**An open-source flash memory programmer built from a cheap microcontroller.**

[![CI](https://github.com/bobberdolle1/openflash/actions/workflows/ci.yml/badge.svg)](https://github.com/bobberdolle1/openflash/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.77%2B-orange.svg?logo=rust)](https://www.rust-lang.org/)

[English](#what-it-is) · [Русский](#по-русски)

</div>

---

## What it is

Commercial flash programmers cost hundreds to thousands. OpenFlash aims at the
same job — read, erase and write the flash chip out of a router, a TV, a car ECU
— from a $4 development board, a few jumper wires and open source software.

Three parts:

- **`openflash`**, a command-line tool that identifies a chip, dumps it,
  programs it and verifies the result.
- **A desktop application** (Tauri + React) over the same core.
- **Device firmware** for the board wired to the chip.

Plus a library that analyses dumps you already have: filesystem and signature
detection, ECC, entropy and pattern analysis, firmware unpacking.

## Honest status

This section is deliberately first, because a table of supported boards is easy
to write and hard to earn.

**Works end to end today:** SPI NOR flash, over a Raspberry Pi running the
OpenFlash agent. Identify, dump, erase, program, verify — all covered by tests
that run in CI.

**Works without hardware:** the built-in emulator implements the device side of
the protocol with real flash semantics, so you can try every command and see
exactly what the tools do.

**Does not work yet:** the microcontroller firmware. The RP2040, STM32 and ESP32
drivers are substantial — thousands of lines of PIO, FSMC and SPI code — but none
of them currently builds: they pin dependency versions that no longer resolve and
have no target configuration, linker script or runner. The RP2350 and Arduino
GIGA directories are stubs. Parallel NAND, SPI NAND, eMMC and UFS have host-side
chip databases and parsers, but no device firmware that can drive them.

`docs/PLATFORMS.md` gives the per-board, per-interface detail, including line
counts and what specifically is missing. Nothing in this repository claims a
platform works because a directory for it exists.

**Chip database:** 207 parts with exact entries — 70 SPI NOR, 66 parallel NAND,
45 SPI NAND, 26 eMMC — plus capacity-derived fallbacks for unknown ids. Being in
the database means OpenFlash can name the part and its geometry; whether it can
*read* it depends on the interface, above.

## Try it now, without hardware

```bash
git clone https://github.com/bobberdolle1/openflash
cd openflash/openflash
cargo build --release -p openflash-cli

# A 2 MiB emulated SPI NOR chip, kept in a file between commands
alias of='./target/release/openflash --emulate 2097152 --emulate-image /tmp/chip.bin'

of detect
of --yes write -i some-firmware.bin
of read -o dump.bin
of verify --file some-firmware.bin
```

Every emulated run says so on stderr. The emulator programs like real flash —
bits can only be cleared, a program wrapping a page boundary wraps within the
page, an erase needs the write-enable latch — so a mistake in the tooling shows
up here rather than on your chip.

## With hardware

Today that means a Raspberry Pi with the agent running on it:

```bash
# On the Pi
cargo run --release -p openflash-firmware-raspberry-pi

# On your machine
openflash --unix /tmp/openflash.sock detect
openflash --unix /tmp/openflash.sock read -o dump.bin
```

Wiring is in `docs/HARDWARE_GUIDE.md`. USB devices are found automatically once
firmware exists that speaks the current protocol; `openflash scan` lists what is
attached.

## Commands

| Command | What it does |
|---|---|
| `scan` | list attached devices |
| `info` | what the device reports about itself |
| `detect` | read the chip id and look it up |
| `read -o FILE` | dump the chip, or a range with `--start`/`--length` |
| `write -i FILE` | erase the affected sectors, program, verify |
| `erase` | erase a sector-aligned range and confirm it is blank |
| `verify --file FILE` | read the chip back and compare |
| `analyze FILE` | filesystems, entropy, patterns, anomalies |
| `compare A B` | byte-level diff with a report |
| `unpack`, `rootfs`, `vulnscan` | offline firmware analysis |
| `chips` | browse the chip database |

`read` and `verify` open the device read-only, so a dump cannot modify the chip
it is reading. `write` and `erase` ask before touching real hardware unless you
pass `--yes`. Commands whose subsystem is not implemented say so and exit
non-zero rather than printing a success message.

## Safety

Flash programming destroys data when it goes wrong, so the tools are built to
fail loudly:

- every transfer is CRC-protected in both directions; corruption is reported,
  never returned as if it were chip contents
- a reply that does not match the command that was sent is an error, not data
- programming erases first by default, and a sector the write only partially
  covers is read out and rewritten so neighbouring data survives
- a write is verified by reading it back; a mismatch reports the exact offset
- an erase range must be sector-aligned, because erasing more than asked would
  destroy data the user did not name

## Build

```bash
cd openflash
cargo test            # protocol, core, CLI, GUI backend, Pi agent
cargo clippy --all-targets -- -D warnings
```

The desktop app needs Node and, on Linux, webkit:

```bash
sudo apt install libwebkit2gtk-4.1-dev libudev-dev
cd gui && npm ci && npm run tauri dev
```

## Layout

```
openflash/
├── protocol/   wire format shared by host and firmware (no_std)
├── core/       transport, device operations, emulator, chip databases, analysis
├── cli/        the openflash command
├── gui/        Tauri + React desktop app
├── pyopenflash/ Python bindings
├── firmware/   per-board device code
└── docs/       PROTOCOL.md, PLATFORMS.md, HARDWARE_GUIDE.md
```

`docs/PROTOCOL.md` is the wire format. Read it before writing firmware — the
opcode table lives in one crate and a test fails the build if a firmware
reintroduces its own.

## Contributing

The most valuable contributions right now, in order:

1. **Get one microcontroller firmware building and talking.** The Teensy 4 is
   closest: its opcode table already matches except `GetVersion`.
2. **Hardware-in-the-loop testing.** A self-hosted runner with a Pico and a
   W25Q part would let CI prove a real read, which is the only thing that
   really protects against regressions here.
3. **Chip database entries.** Add a part, add a test.
4. **Parallel NAND on the Pi agent.** The scaffold is there; it needs the
   address cycles.

`CONTRIBUTING.md` has the process. Please do not add a platform or a feature to
the documentation before the code behind it works — the project has been through
that once already, and digging out was most of the work in `docs/PLATFORMS.md`.

## Licence

MIT. See [LICENSE](LICENSE).

---

## По-русски

**OpenFlash — открытый программатор флеш-памяти на дешёвой отладочной плате.**

Задача та же, что у коммерческих программаторов за $200–2000: прочитать,
стереть и записать флеш-чип из роутера, телевизора или блока управления
автомобилем. Только на плате за $4, нескольких проводах и открытом коде.

### Честный статус

**Работает полностью:** SPI NOR через Raspberry Pi с агентом OpenFlash —
определение чипа, дамп, стирание, запись, проверка. Всё покрыто тестами в CI.

**Работает без железа:** встроенный эмулятор реализует устройство со настоящей
семантикой флеш-памяти, так что все команды можно попробовать и увидеть, что
именно делает инструмент.

**Пока не работает:** прошивки для микроконтроллеров. Драйверы для RP2040, STM32
и ESP32 объёмные — тысячи строк кода PIO, FSMC и SPI — но ни один сейчас не
собирается: зафиксированы версии зависимостей, которые больше не разрешаются, и
нет ни целевой платформы, ни линкер-скрипта. Каталоги RP2350 и Arduino GIGA —
заглушки. Для parallel NAND, SPI NAND, eMMC и UFS есть базы чипов и парсеры на
стороне хоста, но нет прошивки, которая умеет с ними работать.

Подробности по каждой плате и интерфейсу — в `docs/PLATFORMS.md`. Ни одна
платформа здесь не считается поддержанной только потому, что для неё есть
каталог.

**База чипов:** 207 моделей с точными записями (70 SPI NOR, 66 parallel NAND,
45 SPI NAND, 26 eMMC) плюс определение по байту ёмкости для незнакомых ID.
Наличие в базе означает, что OpenFlash назовёт чип и его геометрию; сможет ли
он его прочитать — зависит от интерфейса, см. выше.

### Попробовать без железа

```bash
git clone https://github.com/bobberdolle1/openflash
cd openflash/openflash
cargo build --release -p openflash-cli

# Эмулируемый чип SPI NOR на 2 МиБ, состояние сохраняется в файле
alias of='./target/release/openflash --emulate 2097152 --emulate-image /tmp/chip.bin'

of detect
of --yes write -i firmware.bin
of read -o dump.bin
of verify --file firmware.bin
```

Каждый эмулируемый запуск помечается в stderr. Эмулятор программирует как
настоящая флеш-память: биты можно только сбрасывать, запись через границу
страницы заворачивается внутри страницы, стиранию нужен write-enable. Ошибка в
инструментах проявится здесь, а не на вашем чипе.

### Безопасность

Неудачная запись флеш-памяти уничтожает данные, поэтому инструменты сделаны так,
чтобы падать громко:

- каждая передача защищена CRC в обе стороны; повреждение сообщается, а не
  возвращается как содержимое чипа
- ответ, не соответствующий отправленной команде, — это ошибка, а не данные
- перед записью сектор стирается, а частично затронутый сектор считывается и
  перезаписывается, чтобы соседние данные выжили
- запись проверяется обратным чтением, при расхождении указывается точное
  смещение
- диапазон стирания обязан быть выровнен по сектору

### Как помочь

1. **Поднять хотя бы одну прошивку для микроконтроллера.** Ближе всех Teensy 4 —
   его таблица опкодов уже совпадает, кроме `GetVersion`.
2. **Тесты на реальном железе в CI** (self-hosted runner с Pico и W25Q).
3. **Новые чипы в базу** — запись плюс тест.
4. **Parallel NAND в агенте Raspberry Pi** — каркас есть, нужны адресные циклы.

Пожалуйста, не добавляйте платформу или функцию в документацию раньше, чем
заработает код: проект уже проходил через это, и разбор последствий занял
большую часть работы над `docs/PLATFORMS.md`.
