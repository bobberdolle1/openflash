# Chip database

## What "supported" means here

Two different things, and the difference matters:

- **In the database** — OpenFlash recognises the chip's id and knows its
  geometry: page size, block size, capacity, spare area. This is what lets it
  name the part and lay out a dump correctly.
- **Readable** — OpenFlash can actually talk to the chip and take a dump. That
  needs firmware on the device side for the chip's interface.

Today the database holds **207 parts across four interfaces**, and exactly one of
those interfaces can be read: **SPI NOR**, over a Raspberry Pi, Orange Pi or
Banana Pi running the agent. Parallel NAND, SPI NAND and eMMC parts are in the
database and their dumps can be parsed, but no firmware in this repository can
take such a dump. `docs/PLATFORMS.md` has the per-board detail.

| Interface | Parts in the database | Can be read today |
|---|---|---|
| SPI NOR | 70 | yes — the three SBC agents |
| Parallel NAND | 65 | no |
| SPI NAND | 45 | no |
| eMMC | 27 | no |

An earlier version of this page had a "Fully Tested ✅" table listing three
parallel NAND chips. Nothing in this repository has ever been able to read a
parallel NAND chip, so those cannot have been tested through it. No chip in the
database has been verified against physical hardware by this project — see the
note at the bottom.

## Looking a chip up

The database is queried by chip id:

```bash
openflash chips --id EF4018          # SPI NOR, JEDEC id
openflash chips --id "EC F1 00 95 40"  # parallel NAND, ONFI id
```

Separators are optional; `EF4018`, `ef 40 18` and `EF:40:18` are the same query.
All four databases are searched and every match is reported, because the leading
byte is a JEDEC manufacturer code that product lines share.

To read the id off a chip that is attached:

```bash
openflash detect
```

### Exact entries and derived geometry

Each database also has a fallback that derives geometry from the id — for SPI
NOR, the third byte is log2 of the capacity, so a size can be computed for a part
nobody has catalogued. Those two answers are not equally trustworthy, so they are
distinguished:

- a plain result is a catalogue entry
- a result marked `[derived from the id, not a catalogue entry]` was computed

A search across all four databases returns catalogue entries only. Derived
answers are offered when you name the interface, which tells OpenFlash which
catalogue is the right one to ask:

```bash
openflash chips --id 1F8701 --interface spi-nor
```

### Listing the whole database

Not available yet. The databases are written as `match` arms keyed by chip id, so
they can be queried but not iterated, and `openflash chips` without `--id`
reports that rather than printing a stand-in list. Turning them into tables that
can be both looked up and enumerated is tracked in the README's contributing
list.

## Adding a chip

1. Get the id — `openflash detect`, or the datasheet.
2. Find page size, block size, capacity and spare-area size in the datasheet.
3. Add an entry to the matching module in `core/src/`:
   `spi_nor.rs`, `spi_nand.rs`, `onfi.rs` (parallel NAND) or `emmc.rs`.
4. Add a test that looks your id up and asserts the geometry.

The test is the part that matters. A wrong page size in the database produces a
dump that is silently misaligned, which is worse than not recognising the chip
at all.

## Manufacturer ids

The first byte of a chip id is a JEDEC manufacturer code:

| id | Manufacturer |
|----|--------------|
| 0x01 | AMD/Spansion |
| 0x20 | ST/Numonyx |
| 0x2C | Micron |
| 0x89 | Intel |
| 0x98 | Toshiba/Kioxia |
| 0xAD | SK Hynix |
| 0xC2 | Macronix |
| 0xC8 | GigaDevice |
| 0xEC | Samsung |
| 0xEF | Winbond |

## On testing

No entry in this database has been verified against a physical chip by this
project — nobody working on it has had the hardware. Entries come from
datasheets, and the code paths around them are covered by unit tests and by an
emulator that implements real flash semantics. That catches a mistyped page size
against the datasheet; it does not catch a datasheet that is wrong, or a part
whose real behaviour differs from its documentation.

If you have run OpenFlash against a real chip, saying so in an issue is a genuine
contribution, whether it worked or not.
