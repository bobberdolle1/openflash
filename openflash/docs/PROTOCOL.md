# OpenFlash wire protocol

This is the contract between a host (the CLI, the GUI, the Python bindings) and
a device (microcontroller firmware, or an agent running on a single-board
computer). It is implemented once, in the `openflash-protocol` crate, and shared
by both sides — host and firmware depend on the same code rather than on two
copies of a table that agree by convention.

Revision described here: **2**.

## Why revision 2 exists

Revision 1 put a bare `[command][args…]` into a 64-byte USB packet. It had no
length field, no checksum and no delimiter, which meant:

- a dropped or inserted byte desynchronised the stream permanently, and nothing
  detected it. The host went on attributing each reply to the wrong request.
- a reply's size was implied by the command, so a host and a firmware that
  disagreed about a payload length would hang or silently truncate.
- corruption on the wire was indistinguishable from flash contents. For a tool
  whose entire output is a byte-exact image, that is the worst failure mode
  available.

There was also no single command table. Each firmware declared its own, and they
had drifted apart:

| Operation | rp2040, stm32f1, stm32f4 | esp32, raspberry_pi, orange_pi | teensy4 | Host |
|---|---|---|---|---|
| `Ping` | `0x01` | `0x00` | `0x01` | `0x01` |
| NAND read page | `0x05` | `0x11` | `0x12` | `0x12` |
| NAND read id | `0x07` | `0x10` | `0x14` | `0x14` |
| SPI NOR read id | `0x60` | `0x70` | `0x60` | `0x60` |
| `GetVersion` | — | `0x01`/`0x02` | `0x03` | `0x0A` |

A host could therefore talk fully to no board, and to the ESP32 not at all — not
even a ping. `protocol/tests/firmware_conformance.rs` now fails the build if a
firmware reintroduces its own numbering.

## Framing

Every message in both directions is one frame:

```text
offset  size  field
     0     2  magic, ASCII "OF" (0x4F 0x46)
     2     1  protocol version (2)
     3     1  command
     4     1  status         (0x00 in a request)
     5     1  flags          (reserved, must be 0)
     6     2  payload length (u16, little endian)
     8     2  header CRC-16/CCITT-FALSE over offsets 0..8
    10   len  payload
10+len     2  payload CRC-16/CCITT-FALSE
```

Overhead is 12 bytes. Maximum payload is 8192 bytes, chosen so a device can
statically allocate one frame buffer and still carry the largest single transfer
any interface needs: a 4 KiB SPI NOR sector, or a 4096+256-byte NAND page with
its spare area.

Three properties matter:

- **The header carries its own CRC.** A receiver validates `payload length`
  before it trusts it, so a corrupted length can never make a device wait for
  bytes that will never arrive, nor make a host allocate against garbage.
- **Length prefixing, not delimiter scanning.** A payload that happens to
  contain `4F 46` is not special.
- **The magic allows resynchronisation.** A receiver that has lost sync scans
  forward for a magic followed by a header whose CRC checks out. The host
  reports how many bytes it had to discard rather than papering over it: a link
  that loses bytes is a link whose dumps cannot be trusted.

CRC-16/CCITT-FALSE (polynomial `0x1021`, initial value `0xFFFF`) is pinned by a
test against the standard check value — `"123456789"` must produce `0x29B1` — so
a host and a firmware built from different revisions cannot disagree about it.

## Status codes

A response carries a status. `Ok` is the only success.

| Code | Name | Meaning |
|---|---|---|
| `0x00` | `Ok` | the command completed |
| `0x01` | `Error` | failed, no more specific code applies |
| `0x02` | `Busy` | a previous command is still running |
| `0x03` | `Timeout` | the device timed out talking to the chip |
| `0x04` | `UnsupportedCommand` | this firmware does not implement the opcode |
| `0x05` | `InvalidArgument` | the payload was malformed or out of range |
| `0x06` | `UnsupportedOnInterface` | valid command, wrong active interface |
| `0x07` | `ChipNotFound` | no chip responded |
| `0x08` | `EccFailure` | data was read but ECC could not correct it |
| `0x09` | `WriteProtected` | the chip or the device refused a write |
| `0x0A` | `VerifyFailed` | a verify-after-write comparison failed |

A device must answer every frame it can parse, including one naming an opcode it
does not know: silence makes a host wait for a timeout instead of reporting
"unsupported".

## Command space

Only operations a device performs live in the command space.

| Range | Contents |
|---|---|
| `0x00` | never a command — an idle bus reads as all zeroes |
| `0x01`–`0x0F` | general: ping, bus config, reset, interface select, version, capabilities |
| `0x03`–`0x07` | deprecated revision 1 parallel-NAND aliases; accepted on receive, never sent |
| `0x10`–`0x1F` | parallel NAND |
| `0x20`–`0x3F` | SPI NAND |
| `0x40`–`0x5F` | eMMC |
| `0x60`–`0x7F` | SPI NOR |
| `0x80`–`0x9F` | UFS |
| `0xA0`–`0xAF` | bad-block and wear management |
| `0xB0`–`0xDF` | reserved: host-side batching, scripting, job and server control |
| `0xE0`–`0xEF` | hardware expansion: PCB, adapters, logic analyzer, JTAG/SWD |
| `0xF0`–`0xFE` | reserved: cloud features, host-to-server only |
| `0xFF` | never a command — an idle or floating bus reads as all ones |

`0x00` and `0xFF` are excluded deliberately. They are what a host reads from a
disconnected, floating or held-in-reset bus, and what a NAND chip's own RESET
opcode is. If either decoded as a valid command, electrical noise would decode
as a real operation. Revision 1 had `CloudStatus = 0xFF`, which collided with
all three of those and with the invalid-command sentinel.

Cloud and server opcodes are host-to-server concerns and are not in the device
command space at all.

The authoritative list is `openflash_protocol::Command`, and
`Command::ALL` is checked to be complete and collision-free by a test.

## Handshake

A host must run this before anything else:

1. `Ping` (`0x01`), empty payload. Confirms something is listening.
2. `GetVersion` (`0x0A`), empty payload. The reply is six bytes:

   ```text
   [protocol][fw major][fw minor][fw patch][platform][interface bitmap]
   ```

   The host refuses to continue when `protocol` is not the revision it speaks.
   Two builds that disagree about the wire format would otherwise appear to work
   and return wrong data.

The interface bitmap has bit *n* set when the device implements the interface
whose `FlashInterface` value is *n*. A device must advertise only what is
actually wired up: a host uses this to decide what to offer, so over-claiming
turns into operations that fail one at a time.

## Payload layouts

Multi-byte integers in payloads are little endian. Addresses are byte addresses.

### General

| Command | Request | Response |
|---|---|---|
| `Ping` `0x01` | — | — |
| `BusConfig` `0x02` | `[clock MHz][mode][quad]` | — |
| `Reset` `0x08` | — | — |
| `SetInterface` `0x09` | `[interface]` | — |
| `GetVersion` `0x0A` | — | 6 bytes, see above |
| `GetCapabilities` `0x0B` | — | `[interface bitmap]` |

### SPI NOR

| Command | Request | Response |
|---|---|---|
| `SpiNorReadJedecId` `0x60` | — | 3 bytes |
| `SpiNorRead` `0x62` | `[addr u32][len u16]` | `len` bytes |
| `SpiNorFastRead` `0x63` | `[addr u32][len u16]` | `len` bytes |
| `SpiNorPageProgram` `0x66` | `[addr u32][data…]` | — |
| `SpiNorSectorErase` `0x67` | `[addr u32]` | — |
| `SpiNorBlockErase32K` `0x68` | `[addr u32]` | — |
| `SpiNorBlockErase64K` `0x69` | `[addr u32]` | — |
| `SpiNorChipErase` `0x6A` | — | — |
| `SpiNorReadStatus1` `0x6B` | — | 1 byte |
| `SpiNorWriteEnable` `0x71` | — | — |
| `SpiNorWriteDisable` `0x72` | — | — |

A device must reject a program that would cross a 256-byte page boundary with
`InvalidArgument` rather than passing it to the chip, which wraps it to the
start of the same page and corrupts data. It must likewise reject an erase at an
address that is not aligned to the requested granularity.

The remaining interfaces (parallel NAND, SPI NAND, eMMC, UFS) have opcodes
assigned but no payload layout is fixed yet, because no firmware in this
repository implements them end to end. Fixing a layout before something
implements it is how the four incompatible tables came about, so those rows are
deliberately absent.

## Transports

The frame format is identical on every transport.

| Transport | Details |
|---|---|
| USB | bulk endpoints `0x01` OUT and `0x81` IN, vendor `0xC0DE`, product `0xCAFE` |
| TCP | the SBC agents listen on a port; frames back to back on the stream |
| Unix socket | the SBC agents' default, `/tmp/openflash.sock` |

No transport guarantees that one read yields one frame: USB splits at the
endpoint packet size and TCP may split anywhere. Both sides must buffer and
retry decoding until a whole frame has arrived. The host implementation is
`openflash_core::transport`; the device side of the same logic is in the
Raspberry Pi agent's `serve_client`.

## Adding a firmware

1. Depend on the shared crate, without default features so it stays `no_std`
   and allocation-free:

   ```toml
   openflash-protocol = { version = "3.1", default-features = false }
   ```

2. Do not declare a command table. Use `openflash_protocol::Command`.
3. Implement `Ping`, `GetVersion` and `GetCapabilities`. Advertise only the
   interfaces that work.
4. Answer every frame, including with `UnsupportedCommand`.
5. Validate before acting: page boundaries, erase alignment, address ranges.
6. Run `cargo test -p openflash-protocol` — the conformance test reads your
   source and will fail if any opcode disagrees.

See `docs/PLATFORMS.md` for what each existing firmware actually implements.
