# Supported NAND Chips

OpenFlash supports ONFI-compliant NAND flash chips. Below is a list of tested and known-working chips.

## Fully Tested ✅

| Manufacturer | Model | Capacity | Page Size | Status |
|--------------|-------|----------|-----------|--------|
| Samsung | K9F4G08U0D | 512MB | 2KB | ✅ Tested |
| Samsung | K9F1G08U0D | 128MB | 2KB | ✅ Tested |
| Hynix | HY27UF084G2B | 512MB | 2KB | ✅ Tested |

## Database (Auto-Detection)

These chips are in our database and should work automatically:

### Samsung
| Model | Capacity | Page | Block | ID |
|-------|----------|------|-------|-----|
| K9F1G08U0D | 128MB | 2KB | 64 | EC F1 00 95 |
| K9F2G08U0D | 256MB | 2KB | 64 | EC DA 10 95 |
| K9F4G08U0D | 512MB | 2KB | 64 | EC DC 10 95 |
| K9F8G08U0M | 1GB | 2KB | 64 | EC D3 51 95 |
| K9GAG08U0E | 2GB | 8KB | 128 | EC D5 84 72 |
| K9LCG08U0A | 4GB | 8KB | 128 | EC D7 94 76 |

### SK Hynix
| Model | Capacity | Page | Block | ID |
|-------|----------|------|-------|-----|
| HY27UF081G2A | 128MB | 2KB | 64 | AD F1 80 1D |
| HY27UF082G2B | 256MB | 2KB | 64 | AD DA 80 15 |
| HY27UF084G2B | 512MB | 2KB | 64 | AD DC 80 15 |
| H27U1G8F2BTR | 128MB | 2KB | 64 | AD F1 00 1D |
| H27U4G8F2DTR | 512MB | 4KB | 64 | AD DC 90 95 |

### Micron
| Model | Capacity | Page | Block | ID |
|-------|----------|------|-------|-----|
| MT29F1G08ABADAH4 | 128MB | 2KB | 64 | 2C F1 80 95 |
| MT29F2G08ABAEAH4 | 256MB | 2KB | 64 | 2C DA 90 95 |
| MT29F4G08ABADAH4 | 512MB | 2KB | 64 | 2C DC 90 95 |
| MT29F8G08ADBDAH4 | 1GB | 4KB | 64 | 2C 38 00 26 |
| MT29F16G08CBACAH4 | 2GB | 4KB | 256 | 2C 48 04 46 |

### Toshiba/Kioxia
| Model | Capacity | Page | Block | ID |
|-------|----------|------|-------|-----|
| TC58NVG0S3HTA00 | 128MB | 2KB | 64 | 98 F1 80 15 |
| TC58NVG1S3HTA00 | 256MB | 2KB | 64 | 98 DA 90 15 |
| TC58NVG2S0HTA00 | 512MB | 4KB | 64 | 98 DC 90 26 |
| TC58NVG3S0FTA00 | 1GB | 4KB | 64 | 98 D3 90 26 |

### Macronix
| Model | Capacity | Page | Block | ID |
|-------|----------|------|-------|-----|
| MX30LF1G08AA | 128MB | 2KB | 64 | C2 F1 80 1D |
| MX30LF2G18AC | 256MB | 2KB | 64 | C2 DA 90 95 |
| MX30LF4G18AC | 512MB | 2KB | 64 | C2 DC 90 95 |

### Winbond
| Model | Capacity | Page | Block | ID |
|-------|----------|------|-------|-----|
| W29N01GVSIAA | 128MB | 2KB | 64 | EF F1 00 95 |
| W29N02GVSIAA | 256MB | 2KB | 64 | EF DA 10 95 |

### GigaDevice
| Model | Capacity | Page | Block | ID |
|-------|----------|------|-------|-----|
| GD9FU1G8F2A | 128MB | 2KB | 64 | C8 F1 80 1D |
| GD9FS2G8F2A | 256MB | 2KB | 64 | C8 DA 90 95 |

## Unknown Chips

If your chip isn't recognized:

1. OpenFlash will show "Unknown" with the chip ID
2. You can still try operations with manual settings
3. [Request chip support](https://github.com/openflash/openflash/issues/new?template=chip_support.md)

## Adding New Chips

To add a chip to the database:

1. Get the chip ID (shown in OpenFlash)
2. Find the datasheet for specifications
3. Submit a PR or issue with:
   - Manufacturer and model
   - Chip ID bytes
   - Page size, block size, capacity
   - Any special timing requirements

## Manufacturer ID Reference

| ID | Manufacturer |
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
