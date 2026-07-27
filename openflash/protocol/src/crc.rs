//! CRC-16/CCITT-FALSE, used to protect every frame header and payload.
//!
//! Table-free and `const`-friendly so bare-metal firmware can use it without
//! spending flash on a lookup table.

/// Polynomial for CRC-16/CCITT-FALSE.
const POLY: u16 = 0x1021;

/// Initial CRC register value for CRC-16/CCITT-FALSE.
pub const INIT: u16 = 0xFFFF;

/// Update `crc` with a single byte.
pub const fn update(mut crc: u16, byte: u8) -> u16 {
    crc ^= (byte as u16) << 8;
    let mut bit = 0;
    while bit < 8 {
        crc = if crc & 0x8000 != 0 {
            (crc << 1) ^ POLY
        } else {
            crc << 1
        };
        bit += 1;
    }
    crc
}

/// Compute the CRC-16/CCITT-FALSE of `data`.
pub fn checksum(data: &[u8]) -> u16 {
    let mut crc = INIT;
    for &byte in data {
        crc = update(crc, byte);
    }
    crc
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The canonical CRC-16/CCITT-FALSE check value: the string "123456789"
    /// must produce 0x29B1. This pins the implementation to the standard so a
    /// host and a firmware built from different revisions cannot disagree.
    #[test]
    fn matches_the_standard_check_value() {
        assert_eq!(checksum(b"123456789"), 0x29B1);
    }

    #[test]
    fn empty_input_is_the_init_value() {
        assert_eq!(checksum(&[]), INIT);
    }

    #[test]
    fn detects_every_single_bit_flip_in_a_short_frame() {
        let original = [0x4F, 0x46, 0x02, 0x14, 0x00, 0x00, 0x05, 0x00];
        let expected = checksum(&original);

        for byte_index in 0..original.len() {
            for bit in 0..8 {
                let mut corrupted = original;
                corrupted[byte_index] ^= 1 << bit;
                assert_ne!(
                    checksum(&corrupted),
                    expected,
                    "flipping bit {bit} of byte {byte_index} went undetected"
                );
            }
        }
    }

    #[test]
    fn update_is_equivalent_to_checksum() {
        let data = b"openflash";
        let mut crc = INIT;
        for &byte in data {
            crc = update(crc, byte);
        }
        assert_eq!(crc, checksum(data));
    }
}
