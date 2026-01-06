# Troubleshooting

Common issues and solutions.

## Connection Issues

### "No devices found"

**Symptoms**: Clicking Scan shows no devices

**Solutions**:
1. Check USB cable (use data cable, not charge-only)
2. Try different USB port
3. Check if device appears in system:
   - Windows: Device Manager → Ports (COM & LPT)
   - macOS: `ls /dev/tty.usb*`
   - Linux: `ls /dev/ttyACM*`
4. Reflash firmware to microcontroller
5. Try Mock mode to verify app works

### "Connection failed" after connecting

**Symptoms**: Device found but ping fails

**Solutions**:
1. Disconnect and reconnect USB
2. Reset microcontroller (press reset button)
3. Check firmware version matches app version
4. Try different USB cable

## Chip Detection Issues

### "Unknown chip" or wrong detection

**Symptoms**: Chip ID shown but not recognized

**Solutions**:
1. Check wiring - especially CLE, ALE, CE#
2. Verify 3.3V power to chip
3. Add 10kΩ pull-up on R/B# line
4. Check for cold solder joints
5. Try slower timing (if supported)
6. [Report chip](https://github.com/openflash/openflash/issues/new?template=chip_support.md) for database addition

### All 0xFF or 0x00 chip ID

**Symptoms**: ID reads as FF FF FF FF FF or 00 00 00 00 00

**Causes**:
- No power to chip
- CE# not going low
- Data bus not connected
- Chip is dead

**Solutions**:
1. Verify VCC and GND connections
2. Check CE# signal with multimeter/scope
3. Verify data bus wiring (D0-D7)
4. Try known-good chip

## Dump Issues

### Dump contains all 0xFF

**Symptoms**: Entire dump is empty (0xFF bytes)

**Causes**:
- Chip is erased/empty
- RE# signal not working
- Data bus issue

**Solutions**:
1. Check RE# wiring
2. Verify data bus connections
3. Try reading chip ID first (if that works, data bus is OK)
4. Chip may actually be empty

### Dump has repeating patterns

**Symptoms**: Same data repeats every N bytes

**Causes**:
- Address lines not connected
- Partial address bus failure

**Solutions**:
1. Check all address-related signals (ALE)
2. Verify column and row address cycles

### Corrupted data / bit errors

**Symptoms**: Data looks mostly correct but has errors

**Causes**:
- Signal integrity issues
- Timing too fast
- ECC errors in original data

**Solutions**:
1. Shorten wires
2. Add decoupling capacitors
3. Use ECC correction in analysis
4. Try slower timing

## GUI Issues

### App won't start

**Windows**:
- Install [WebView2 Runtime](https://developer.microsoft.com/en-us/microsoft-edge/webview2/)
- Run as administrator

**macOS**:
- Right-click → Open (bypass Gatekeeper)
- Check System Preferences → Security

**Linux**:
- Make AppImage executable: `chmod +x OpenFlash*.AppImage`
- Install webkit2gtk: `sudo apt install libwebkit2gtk-4.1-0`

### UI is blank or frozen

**Solutions**:
1. Check for antivirus blocking
2. Try running from terminal to see errors
3. Delete config: `~/.config/openflash/`
4. Reinstall application

### Hex viewer is slow

**Symptoms**: Scrolling is laggy with large dumps

**Solutions**:
1. Use page navigation instead of scrolling
2. Reduce page size in settings
3. Close other applications
4. Large dumps (>1GB) may be slow

## Firmware Issues

### Pico not entering bootloader mode

**Solutions**:
1. Hold BOOTSEL before connecting USB
2. Keep holding until drive appears
3. Try different USB port
4. Check USB cable

### STM32 won't flash

**Solutions**:
1. Set BOOT0 jumper to 1
2. Reset board
3. Flash via ST-Link or serial
4. Set BOOT0 back to 0

### Firmware crashes / resets

**Symptoms**: Device disconnects during operation

**Causes**:
- Power issue
- Firmware bug
- USB enumeration problem

**Solutions**:
1. Use powered USB hub
2. Update to latest firmware
3. Report issue with steps to reproduce

## Performance Issues

### Dump is very slow

**Expected speeds**:
- RP2040: ~100KB/s
- STM32F1: ~50KB/s

**If slower**:
1. Check USB connection (use USB 2.0+ port)
2. Close other USB-heavy applications
3. Avoid USB hubs if possible

## Still Having Issues?

1. Check [GitHub Issues](https://github.com/openflash/openflash/issues) for similar problems
2. Ask in [Discussions](https://github.com/openflash/openflash/discussions)
3. Open a [bug report](https://github.com/openflash/openflash/issues/new?template=bug_report.md) with:
   - OS and version
   - Hardware used
   - Steps to reproduce
   - Error messages/logs
