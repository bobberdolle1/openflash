# Hardware Setup

> **Which of these can you actually use?**
>
> Only the **SPI NOR on a single-board computer** section below. The Raspberry Pi
> Pico and Blue Pill wiring further down is for parallel NAND, and no firmware in
> this repository can drive a parallel NAND bus — the microcontroller firmware
> does not build, and the SBC agents' NAND scaffolding deliberately refuses
> because its address cycles are missing.
>
> That wiring is kept because it is correct reference material and because it is
> what the firmware will need once it exists. It is not a setup that works today.
> See [PLATFORMS.md](../PLATFORMS.md).

## SPI NOR on a single-board computer — the working setup

A Raspberry Pi, Orange Pi or Banana Pi running the OpenFlash agent, wired to a
SPI NOR chip. This is what the project can read end to end.

### The chip

SPI NOR in SOIC-8 or DIP-8 — the W25Q, MX25L, GD25Q and S25FL families and their
relatives. Standard pinout, looking at the top with the dot at pin 1:

```
          ┌───────∪───────┐
   CS# ──┤ 1           8 ├── VCC
DO (IO1) ──┤ 2           7 ├── HOLD#/RESET# (IO3)
WP# (IO2) ──┤ 3           6 ├── CLK
     GND ──┤ 4           5 ├── DI (IO0)
          └───────────────┘
```

### Raspberry Pi

The agent uses SPI0 with chip-select 0, at 10 MHz in mode 0.

| Chip pin | Signal | Pi header pin | GPIO |
|---|---|---|---|
| 1 | CS# | 24 | GPIO8 (CE0) |
| 2 | DO | 21 | GPIO9 (MISO) |
| 3 | WP# | tie to 3.3V | — |
| 4 | GND | 25 | — |
| 5 | DI | 19 | GPIO10 (MOSI) |
| 6 | CLK | 23 | GPIO11 (SCLK) |
| 7 | HOLD# | tie to 3.3V | — |
| 8 | VCC | 17 | 3.3V |

WP# and HOLD# are active low. Leaving them floating gives intermittent failures
that look like bad wiring, so tie both high.

Enable the bus before running the agent — `dtparam=spi=on` in
`/boot/firmware/config.txt`, then reboot. `ls /dev/spidev*` should list
`/dev/spidev0.0`.

### Orange Pi and Banana Pi

Same six connections, but the header layout differs between boards and between
revisions, so read your board's own pinout for MOSI, MISO, SCLK and CE0 rather
than copying the Pi's physical pin numbers. The agents open `/dev/spidev0.0` by
default and detect the board from `/proc/device-tree/model`.

You may need to enable an spidev overlay in `orangepiEnv.txt` or the equivalent.
If the agent cannot open the bus it still starts and reports no interfaces, which
distinguishes "SPI not enabled" from "no chip attached".

### Voltage

Most of these parts are 3.3V and connect directly. **1.8V parts exist** — the
`W25Q...IM` and `MX25U` families among them — and putting 3.3V on one destroys
it. Check the datasheet before wiring power, and use a level shifter and a 1.8V
supply if needed.

### In circuit or desoldered

A SOIC-8 test clip lets you read a chip without desoldering, which usually works
for reading and often does not for writing: the board around the chip loads the
bus, and the host processor may drive the same lines. Hold the board in reset if
you can, and expect to desolder if you get inconsistent reads.

---

## NAND Flash Pinout

Standard TSOP-48 NAND flash pinout:

```
        ┌──────────────────────┐
    NC ─┤ 1                 48 ├─ NC
    NC ─┤ 2                 47 ├─ NC
   GND ─┤ 3                 46 ├─ NC
   VCC ─┤ 4                 45 ├─ NC
    NC ─┤ 5                 44 ├─ NC
    NC ─┤ 6                 43 ├─ NC
   GND ─┤ 7                 42 ├─ NC
    NC ─┤ 8                 41 ├─ NC
    NC ─┤ 9                 40 ├─ NC
    NC ─┤ 10                39 ├─ NC
    NC ─┤ 11                38 ├─ NC
   GND ─┤ 12                37 ├─ VCC
   VCC ─┤ 13                36 ├─ GND
    NC ─┤ 14                35 ├─ NC
   WP# ─┤ 15                34 ├─ NC
    NC ─┤ 16                33 ├─ NC
   CE# ─┤ 17                32 ├─ NC
   GND ─┤ 18                31 ├─ GND
    NC ─┤ 19                30 ├─ NC
   CLE ─┤ 20                29 ├─ I/O7
   ALE ─┤ 21                28 ├─ I/O6
   WE# ─┤ 22                27 ├─ I/O5
   RE# ─┤ 23                26 ├─ I/O4
  R/B# ─┤ 24                25 ├─ VCC
        └──────────────────────┘
```

**Note**: Pinout varies by manufacturer. Always check your chip's datasheet!

## Raspberry Pi Pico (RP2040)

### Pinout

| NAND Signal | Pico Pin | GPIO |
|-------------|----------|------|
| CLE         | Pin 1    | GP0  |
| ALE         | Pin 2    | GP1  |
| WE#         | Pin 4    | GP2  |
| RE#         | Pin 5    | GP3  |
| CE#         | Pin 6    | GP4  |
| R/B#        | Pin 7    | GP5  |
| D0          | Pin 9    | GP6  |
| D1          | Pin 10   | GP7  |
| D2          | Pin 11   | GP8  |
| D3          | Pin 12   | GP9  |
| D4          | Pin 14   | GP10 |
| D5          | Pin 15   | GP11 |
| D6          | Pin 16   | GP12 |
| D7          | Pin 17   | GP13 |
| GND         | Pin 3,8  | GND  |
| VCC (3.3V)  | Pin 36   | 3V3  |

### Wiring Diagram

```
Raspberry Pi Pico              NAND Flash
┌─────────────────┐           ┌──────────┐
│ GP0  (Pin 1)  ──┼───────────┼── CLE    │
│ GP1  (Pin 2)  ──┼───────────┼── ALE    │
│ GP2  (Pin 4)  ──┼───────────┼── WE#    │
│ GP3  (Pin 5)  ──┼───────────┼── RE#    │
│ GP4  (Pin 6)  ──┼───────────┼── CE#    │
│ GP5  (Pin 7)  ──┼───────────┼── R/B#   │
│ GP6  (Pin 9)  ──┼───────────┼── D0     │
│ GP7  (Pin 10) ──┼───────────┼── D1     │
│ GP8  (Pin 11) ──┼───────────┼── D2     │
│ GP9  (Pin 12) ──┼───────────┼── D3     │
│ GP10 (Pin 14) ──┼───────────┼── D4     │
│ GP11 (Pin 15) ──┼───────────┼── D5     │
│ GP12 (Pin 16) ──┼───────────┼── D6     │
│ GP13 (Pin 17) ──┼───────────┼── D7     │
│ 3V3  (Pin 36) ──┼───────────┼── VCC    │
│ GND  (Pin 3)  ──┼───────────┼── GND    │
└─────────────────┘           └──────────┘
```

## STM32F103 (Blue Pill)

### Pinout

| NAND Signal | Blue Pill | GPIO |
|-------------|-----------|------|
| CLE         | PA0       | PA0  |
| ALE         | PA1       | PA1  |
| WE#         | PA2       | PA2  |
| RE#         | PA3       | PA3  |
| CE#         | PA4       | PA4  |
| R/B#        | PA5       | PA5  |
| D0-D7       | PB0-PB7   | PB0-7|
| GND         | GND       | GND  |
| VCC (3.3V)  | 3.3V      | 3V3  |

## Important Notes

### ⚠️ Voltage Warning
- NAND flash operates at **3.3V**
- **Never connect 5V** to NAND pins
- Both Pico and Blue Pill are 3.3V, so direct connection is safe

### Pull-up Resistors
- R/B# (Ready/Busy) needs a **10kΩ pull-up** to VCC
- Some chips have internal pull-ups, but external is recommended

### Decoupling Capacitors
- Add **100nF capacitor** between VCC and GND near the NAND chip
- Helps with signal integrity

### Signal Integrity
- Keep wires short (< 10cm)
- Use twisted pairs for data lines if possible
- Ground plane helps reduce noise

## TSOP-48 Adapter

For easier connections, use a TSOP-48 breakout board:
- Search "TSOP48 adapter" on AliExpress/eBay
- Provides easy access to all pins
- Some include ZIF socket for chip removal

## Next Steps

- [Supported Chips](Supported-Chips.md) — what the database knows, and what can
  actually be read
- [Getting Started](Getting-Started.md) — build the tools and run the agent
- [PLATFORMS.md](../PLATFORMS.md) — which boards work, and what the rest need
