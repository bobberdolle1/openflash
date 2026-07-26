# openflash (Python bindings)

Python bindings for [OpenFlash](https://github.com/bobberdolle1/openflash), an
open-source flash memory programmer.

```python
import openflash

# The single attached USB device, an SBC agent, or the emulator.
device = openflash.connect()
device = openflash.connect_tcp("raspberrypi.local:9999")
device = openflash.connect_unix("/tmp/openflash.sock")
device = openflash.connect_emulated(2 * 1024 * 1024)   # no hardware involved

chip = device.detect()
print(f"{chip.manufacturer} {chip.model}, {chip.capacity} bytes")

dump = device.read_full()
dump.save("dump.bin")

device.write(open("firmware.bin", "rb").read(), start=0, verify=True)

analysis = openflash.ai.analyze(dump)
print(analysis.summary)
```

Every device method performs real I/O and raises when it cannot. There is no
mode in which `connect()` succeeds without a device or `read()` returns invented
bytes.

`connect_emulated` runs against an in-process emulator with real flash
semantics — programming only clears bits, an erase is needed before rewriting —
so scripts can be developed without hardware. Nothing real is read or written.

## Install

```bash
pip install maturin
cd openflash/pyopenflash
maturin develop --release
```

See `docs/PLATFORMS.md` in the repository for which interfaces and boards
actually work.
