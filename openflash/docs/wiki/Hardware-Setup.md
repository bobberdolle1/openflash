# Hardware Setup

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

- [Supported Chips](Supported-Chips.md) - Verify your chip is supported
- [Getting Started](Getting-Started.md) - Flash firmware and test
