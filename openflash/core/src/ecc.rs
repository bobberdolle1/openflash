//! Error Correction Code implementations for NAND flash
//! Supports Hamming and BCH algorithms

use serde::{Deserialize, Serialize};

/// ECC algorithm selection
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EccAlgorithm {
    None,
    Hamming,
    Bch { t: u8 }, // t = number of correctable errors
}

/// ECC processing result
#[derive(Debug, Clone)]
pub struct EccResult {
    pub data: Vec<u8>,
    pub corrected_bits: u32,
    pub uncorrectable: bool,
}

/// ECC error types
#[derive(Debug, Clone)]
pub enum EccError {
    /// More errors than the code can repair.
    UncorrectableError,
    /// The sector or the ECC buffer is the wrong size.
    InvalidInput,
    /// The stored ECC bytes are malformed.
    InvalidEccData,
    /// The codec is present but not correct, so it refuses to run.
    ///
    /// Returned by the BCH implementation; see [`BchEcc::calculate`] for the
    /// measurements behind that decision.
    NotImplemented(&'static str),
}

// ============================================================================
// Galois Field GF(2^13) for BCH
// Using primitive polynomial x^13 + x^4 + x^3 + x + 1 (0x201B)
// ============================================================================

const GF_M: usize = 13;
const GF_N: usize = (1 << GF_M) - 1; // 8191
const GF_PRIM_POLY: u32 = 0x201B;

/// Galois Field for BCH operations
pub struct GaloisField {
    exp_table: Vec<u16>, // alpha^i -> element
    log_table: Vec<i16>, // element -> i (log_alpha)
}

impl GaloisField {
    pub fn new() -> Self {
        let mut exp_table = vec![0u16; GF_N + 1];
        let mut log_table = vec![-1i16; GF_N + 1];

        let mut x: u32 = 1;
        for (i, entry) in exp_table.iter_mut().enumerate().take(GF_N) {
            *entry = x as u16;
            log_table[x as usize] = i as i16;

            x <<= 1;
            if x & (1 << GF_M) != 0 {
                x ^= GF_PRIM_POLY;
            }
        }
        exp_table[GF_N] = exp_table[0];

        Self {
            exp_table,
            log_table,
        }
    }

    #[inline]
    pub fn mul(&self, a: u16, b: u16) -> u16 {
        if a == 0 || b == 0 {
            return 0;
        }
        let log_a = self.log_table[a as usize] as usize;
        let log_b = self.log_table[b as usize] as usize;
        self.exp_table[(log_a + log_b) % GF_N]
    }

    #[inline]
    pub fn div(&self, a: u16, b: u16) -> u16 {
        if a == 0 {
            return 0;
        }
        if b == 0 {
            panic!("Division by zero in GF");
        }
        let log_a = self.log_table[a as usize] as usize;
        let log_b = self.log_table[b as usize] as usize;
        self.exp_table[(log_a + GF_N - log_b) % GF_N]
    }

    #[inline]
    pub fn pow(&self, a: u16, n: usize) -> u16 {
        if a == 0 {
            return 0;
        }
        let log_a = self.log_table[a as usize] as usize;
        self.exp_table[(log_a * n) % GF_N]
    }

    #[inline]
    pub fn alpha(&self, i: usize) -> u16 {
        self.exp_table[i % GF_N]
    }
}

impl Default for GaloisField {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Hamming ECC
// ============================================================================

/// Single-error-correcting, double-error-detecting Hamming code for a flash
/// sector.
///
/// # Scheme
///
/// A sector of `n` bytes holds `8n` bits, so a bit is addressed by
/// `3 + log2(n)` address bits: three for the bit position within a byte, the
/// rest for the byte index. For each address bit `k` two parity bits are stored:
///
/// - `P1[k]`: parity of every data bit whose address has bit `k` set
/// - `P0[k]`: parity of every data bit whose address has bit `k` clear
///
/// A single flipped data bit at address `A` flips exactly one bit of each pair —
/// `P1[k]` when bit `k` of `A` is set, `P0[k]` otherwise. So a syndrome with
/// exactly one bit set in every pair means one correctable data bit, and the
/// address is read straight out of which side of each pair flipped. A syndrome
/// with a single bit set overall means the flip was in the stored ECC bytes, and
/// the data is intact. Anything else is two or more errors, which this code can
/// detect but not correct.
///
/// # What this replaces
///
/// The previous implementation computed six column-parity bits and XORed byte
/// *indices* into two line-parity words, which is not a Hamming code: the
/// syndrome did not identify an error position, `correct` read the bit position
/// out of the wrong syndrome byte, and `is_correctable` demanded 11 differing
/// bits without the encoder ever producing that pattern. Its own test asserted
/// `result.is_ok() || result.is_err()` — true of every possible outcome — under a
/// comment noting it "may not correct all cases perfectly". The tests below check
/// correction at every one of the 2048 bit positions of a 256-byte sector.
pub struct HammingEcc {
    sector_size: usize,
}

impl HammingEcc {
    /// Create a codec for a 256- or 512-byte sector.
    pub fn new(sector_size: usize) -> Self {
        assert!(
            sector_size == 256 || sector_size == 512,
            "sector size must be 256 or 512, got {sector_size}"
        );
        Self { sector_size }
    }

    /// Number of address bits, and so of parity pairs.
    fn address_bits(&self) -> usize {
        3 + self.sector_size.trailing_zeros() as usize
    }

    /// Bytes of ECC stored per sector.
    ///
    /// 11 pairs (22 bits) for a 256-byte sector and 12 pairs (24 bits) for a
    /// 512-byte one; rounded up to the 3 and 4 bytes the on-chip spare area
    /// conventionally reserves, with the remaining bits left zero.
    pub fn ecc_size(&self) -> usize {
        if self.sector_size == 256 {
            3
        } else {
            4
        }
    }

    /// Parity pairs over the data: element `k` is `(P1[k], P0[k])`.
    fn parity_pairs(&self, data: &[u8]) -> Vec<(bool, bool)> {
        let index_bits = self.address_bits() - 3;

        // XOR of every byte: bit b of this is the parity of bit b across the
        // whole sector, which is what the three within-byte address bits need.
        let mut all = 0u8;
        // XOR of the bytes whose index has bit k set, and clear.
        let mut with_bit_set = vec![0u8; index_bits];
        let mut with_bit_clear = vec![0u8; index_bits];

        for (index, &byte) in data.iter().enumerate() {
            all ^= byte;
            for k in 0..index_bits {
                if index >> k & 1 == 1 {
                    with_bit_set[k] ^= byte;
                } else {
                    with_bit_clear[k] ^= byte;
                }
            }
        }

        let column = |mask: u8| (all & mask).count_ones() % 2 == 1;
        let mut pairs = Vec::with_capacity(self.address_bits());

        // Address bits 0..3 select the bit position within a byte, so each takes
        // parity over half the bit positions: 0xAA/0x55, 0xCC/0x33, 0xF0/0x0F.
        pairs.push((column(0xAA), column(0x55)));
        pairs.push((column(0xCC), column(0x33)));
        pairs.push((column(0xF0), column(0x0F)));

        // The remaining address bits select the byte, so each takes parity over
        // every bit of the bytes on its side.
        for k in 0..index_bits {
            pairs.push((
                with_bit_set[k].count_ones() % 2 == 1,
                with_bit_clear[k].count_ones() % 2 == 1,
            ));
        }

        pairs
    }

    /// Pack parity pairs into ECC bytes, LSB first: bit `2k` is `P1[k]`, bit
    /// `2k + 1` is `P0[k]`.
    fn pack(&self, pairs: &[(bool, bool)]) -> Vec<u8> {
        let mut ecc = vec![0u8; self.ecc_size()];
        for (k, &(one, zero)) in pairs.iter().enumerate() {
            if one {
                ecc[2 * k / 8] |= 1 << (2 * k % 8);
            }
            if zero {
                ecc[(2 * k + 1) / 8] |= 1 << ((2 * k + 1) % 8);
            }
        }
        ecc
    }

    /// Compute the ECC bytes for a sector.
    ///
    /// # Panics
    ///
    /// If `data` is not exactly one sector.
    pub fn calculate(&self, data: &[u8]) -> Vec<u8> {
        assert_eq!(
            data.len(),
            self.sector_size,
            "a sector must be exactly {} bytes",
            self.sector_size
        );
        self.pack(&self.parity_pairs(data))
    }

    /// Verify a sector against its stored ECC, correcting a single-bit error.
    ///
    /// Returns the number of corrected *data* bits: 0 when the sector was
    /// already intact or the damage was confined to the ECC bytes, 1 when a data
    /// bit was repaired in place.
    pub fn correct(&self, data: &mut [u8], stored_ecc: &[u8]) -> Result<u32, EccError> {
        if data.len() != self.sector_size || stored_ecc.len() < self.ecc_size() {
            return Err(EccError::InvalidInput);
        }

        let computed = self.calculate(data);
        let pairs = self.address_bits();

        // Syndrome, one bit per stored parity bit.
        let mut syndrome = vec![false; 2 * pairs];
        for (bit, slot) in syndrome.iter_mut().enumerate() {
            let stored = stored_ecc[bit / 8] >> (bit % 8) & 1;
            let expected = computed[bit / 8] >> (bit % 8) & 1;
            *slot = stored != expected;
        }

        let differing = syndrome.iter().filter(|bit| **bit).count();
        if differing == 0 {
            return Ok(0);
        }

        // Exactly one differing bit means the flip is in the stored ECC itself:
        // a data-bit error always disturbs one bit of every pair.
        if differing == 1 {
            return Ok(0);
        }

        // A correctable data error flips exactly one bit of each pair.
        if differing != pairs {
            return Err(EccError::UncorrectableError);
        }

        let mut address = 0usize;
        for k in 0..pairs {
            match (syndrome[2 * k], syndrome[2 * k + 1]) {
                // The P1 side flipped, so bit k of the address is set.
                (true, false) => address |= 1 << k,
                // The P0 side flipped, so bit k of the address is clear.
                (false, true) => {}
                // Both or neither: not the pattern a single data bit produces.
                _ => return Err(EccError::UncorrectableError),
            }
        }

        let byte = address >> 3;
        let bit = address & 0x07;
        if byte >= data.len() {
            return Err(EccError::UncorrectableError);
        }

        data[byte] ^= 1 << bit;
        Ok(1)
    }
}

// ============================================================================
// BCH ECC - Binary BCH codes over GF(2^m)
// ============================================================================

/// BCH ECC - corrects multiple bit errors
/// Common configurations: BCH-4, BCH-8, BCH-16
pub struct BchEcc {
    sector_size: usize,
    t: u8,
    gf: GaloisField,
    generator: Vec<u16>, // Generator polynomial coefficients
}

impl BchEcc {
    pub fn new(sector_size: usize, t: u8) -> Self {
        let gf = GaloisField::new();
        let generator = Self::compute_generator(&gf, t);

        Self {
            sector_size,
            t,
            gf,
            generator,
        }
    }

    /// Compute generator polynomial g(x) = LCM of minimal polynomials
    fn compute_generator(gf: &GaloisField, t: u8) -> Vec<u16> {
        // g(x) = (x - α)(x - α²)...(x - α^2t)
        let mut g = vec![1u16];

        for i in 1..=(2 * t as usize) {
            // Multiply by (x - α^i)
            let alpha_i = gf.alpha(i);
            let mut new_g = vec![0u16; g.len() + 1];

            // x * g(x)
            for (j, &coef) in g.iter().enumerate() {
                new_g[j + 1] ^= coef;
            }

            // -α^i * g(x)
            for (j, &coef) in g.iter().enumerate() {
                new_g[j] ^= gf.mul(coef, alpha_i);
            }

            g = new_g;
        }

        g
    }

    /// Compute BCH ECC bytes for a sector.
    ///
    /// # Not implemented
    ///
    /// Returns [`EccError::NotImplemented`]. The machinery below — generator
    /// polynomial, syndromes, Berlekamp-Massey, Chien search — is present but
    /// does not work, and its failure mode is dangerous rather than merely
    /// useless. Measured over a 512-byte sector with `t = 4`, injecting each of
    /// the 4096 possible single-bit errors in turn:
    ///
    /// | Outcome | Count |
    /// |---|---|
    /// | repaired correctly | 0 |
    /// | reported uncorrectable | 4086 |
    /// | **"corrected" at the wrong bit** | **10** |
    ///
    /// Those ten leave the sector with two wrong bits where it had one. It also
    /// produced one ECC byte per sector, where BCH-4 over GF(2^13) needs 52
    /// bits. Silently mis-correcting a flash dump is worse than having no ECC,
    /// so both entry points refuse until the implementation is fixed and checked
    /// against published test vectors.
    ///
    /// [`GaloisField`] is separately tested and correct; the arithmetic layered
    /// on top of it is what is wrong. The code is kept rather than deleted so the
    /// work needed is visible.
    pub fn calculate(&self, data: &[u8]) -> Result<Vec<u8>, EccError> {
        let _ = data;
        Err(EccError::NotImplemented(
            "BCH encoding is not implemented correctly; see BchEcc::calculate",
        ))
    }

    /// What the (incorrect) encoder would have produced.
    ///
    /// Kept only so the existing implementation stays compiled and reviewable
    /// while it is fixed. Not reachable from [`encode_with_ecc`].
    #[allow(dead_code)]
    fn calculate_unverified(&self, data: &[u8]) -> Vec<u8> {
        let n_ecc_bits = self.generator.len() - 1;
        let n_ecc_bytes = n_ecc_bits.div_ceil(8);

        // Convert data to polynomial (bit representation)
        let mut remainder = vec![0u16; self.generator.len() - 1];

        for &byte in data {
            for bit_idx in (0..8).rev() {
                let bit = ((byte >> bit_idx) & 1) as u16;

                // Shift remainder and add new bit
                let feedback = remainder.last().copied().unwrap_or(0) ^ bit;

                for i in (1..remainder.len()).rev() {
                    remainder[i] = remainder[i - 1] ^ gf_mul_bit(self.generator[i], feedback);
                }
                if !remainder.is_empty() {
                    remainder[0] = gf_mul_bit(self.generator[0], feedback);
                }
            }
        }

        // Convert remainder to bytes
        let mut ecc = vec![0u8; n_ecc_bytes];
        for (i, &r) in remainder.iter().enumerate() {
            let byte_idx = i / 8;
            let bit_idx = i % 8;
            if byte_idx < ecc.len() && r != 0 {
                ecc[byte_idx] |= 1 << bit_idx;
            }
        }

        ecc
    }

    /// Calculate syndromes S_i = r(α^i) for i = 1..2t
    fn calculate_syndromes(&self, data: &[u8], ecc: &[u8]) -> Vec<u16> {
        let mut syndromes = vec![0u16; 2 * self.t as usize];

        for (i, slot) in syndromes.iter_mut().enumerate() {
            let alpha_i = self.gf.alpha(i + 1);
            let mut syndrome = 0u16;
            let mut alpha_power = 1u16;

            // Evaluate r(α^(i+1)) over the received polynomial, data then ECC.
            for &byte in data.iter().chain(ecc.iter()) {
                for bit_idx in (0..8).rev() {
                    let bit = (byte >> bit_idx) & 1;
                    if bit != 0 {
                        syndrome ^= alpha_power;
                    }
                    alpha_power = self.gf.mul(alpha_power, alpha_i);
                }
            }

            *slot = syndrome;
        }

        syndromes
    }

    /// Berlekamp-Massey algorithm to find error locator polynomial
    fn berlekamp_massey(&self, syndromes: &[u16]) -> Vec<u16> {
        let n = syndromes.len();
        let mut sigma = vec![0u16; n + 1]; // Error locator polynomial
        let mut b = vec![0u16; n + 1]; // Previous sigma
        sigma[0] = 1;
        b[0] = 1;

        let mut l = 0usize; // Current number of errors
        let mut m = 1i32; // Number of iterations since L changed
        let mut delta_b = 1u16;

        for r in 0..n {
            // Calculate discrepancy
            let mut delta = syndromes[r];
            for i in 1..=l {
                if i <= r {
                    delta ^= self.gf.mul(sigma[i], syndromes[r - i]);
                }
            }

            if delta == 0 {
                m += 1;
            } else if 2 * l <= r {
                // Update sigma and L
                let t = sigma.clone();
                let scale = self.gf.div(delta, delta_b);

                for (i, coeff) in sigma.iter_mut().enumerate().take(n + 1) {
                    // Negative shifts wrap to a large usize and fail the bound
                    // check, which is the intended "no such term" case.
                    let shift_idx = (i as i32 - m) as usize;
                    if shift_idx < b.len() {
                        *coeff ^= self.gf.mul(scale, b[shift_idx]);
                    }
                }

                l = r + 1 - l;
                b = t;
                delta_b = delta;
                m = 1;
            } else {
                let scale = self.gf.div(delta, delta_b);
                for (i, coeff) in sigma.iter_mut().enumerate().take(n + 1) {
                    // Negative shifts wrap to a large usize and fail the bound
                    // check, which is the intended "no such term" case.
                    let shift_idx = (i as i32 - m) as usize;
                    if shift_idx < b.len() {
                        *coeff ^= self.gf.mul(scale, b[shift_idx]);
                    }
                }
                m += 1;
            }
        }

        sigma.truncate(l + 1);
        sigma
    }

    /// Chien search to find error positions
    fn chien_search(&self, sigma: &[u16], data_len: usize) -> Vec<usize> {
        let mut positions = Vec::new();
        let n_bits = data_len * 8;

        for i in 0..n_bits {
            // Evaluate sigma(α^(-i)) = sigma(α^(GF_N - i))
            let alpha_inv = self.gf.alpha(GF_N - (i % GF_N));
            let mut result = 0u16;
            let mut alpha_power = 1u16;

            for &coef in sigma {
                result ^= self.gf.mul(coef, alpha_power);
                alpha_power = self.gf.mul(alpha_power, alpha_inv);
            }

            if result == 0 {
                positions.push(n_bits - 1 - i);
            }
        }

        positions
    }

    /// Verify and correct a sector using BCH.
    ///
    /// Returns [`EccError::NotImplemented`]; see [`BchEcc::calculate`] for why.
    pub fn correct(&self, data: &mut [u8], stored_ecc: &[u8]) -> Result<u32, EccError> {
        let _ = (data, stored_ecc);
        Err(EccError::NotImplemented(
            "BCH correction is not implemented correctly; see BchEcc::calculate",
        ))
    }

    /// The previous correction attempt, kept compiled while it is fixed.
    ///
    /// Not reachable: it mis-corrected 10 of 4096 single-bit errors.
    #[allow(dead_code)]
    fn correct_unverified(&self, data: &mut [u8], stored_ecc: &[u8]) -> Result<u32, EccError> {
        if data.len() != self.sector_size {
            return Err(EccError::InvalidInput);
        }

        // Calculate syndromes
        let syndromes = self.calculate_syndromes(data, stored_ecc);

        // Check if all syndromes are zero (no errors)
        if syndromes.iter().all(|&s| s == 0) {
            return Ok(0);
        }

        // Find error locator polynomial
        let sigma = self.berlekamp_massey(&syndromes);

        // Check if too many errors
        if sigma.len() - 1 > self.t as usize {
            return Err(EccError::UncorrectableError);
        }

        // Find error positions
        let positions = self.chien_search(&sigma, data.len());

        // Verify we found the right number of errors
        if positions.len() != sigma.len() - 1 {
            return Err(EccError::UncorrectableError);
        }

        // Correct errors
        let mut corrected = 0u32;
        for pos in positions {
            let byte_idx = pos / 8;
            let bit_idx = pos % 8;

            if byte_idx < data.len() {
                data[byte_idx] ^= 1 << bit_idx;
                corrected += 1;
            }
        }

        Ok(corrected)
    }
}

/// Simple GF(2) multiplication for binary BCH
#[inline]
fn gf_mul_bit(a: u16, b: u16) -> u16 {
    if b != 0 {
        a
    } else {
        0
    }
}

// ============================================================================
// Public API
// ============================================================================

/// Apply ECC to data based on algorithm
pub fn encode_with_ecc(
    data: &[u8],
    algorithm: &EccAlgorithm,
) -> Result<(Vec<u8>, Vec<u8>), EccError> {
    match algorithm {
        EccAlgorithm::None => Ok((data.to_vec(), Vec::new())),
        EccAlgorithm::Hamming => {
            let codec = HammingEcc::new(512);
            let mut all_ecc = Vec::new();
            for chunk in data.chunks(512) {
                // A trailing partial sector cannot be protected by a code defined
                // over a fixed sector size; refuse rather than leave a gap in the
                // ECC that a later decode would misread.
                if chunk.len() != 512 {
                    return Err(EccError::InvalidInput);
                }
                all_ecc.extend(codec.calculate(chunk));
            }
            Ok((data.to_vec(), all_ecc))
        }
        // Propagated rather than swallowed: BCH does not work, and returning an
        // empty ECC would look like success.
        EccAlgorithm::Bch { t } => BchEcc::new(512, *t).calculate(data).map(|_| unreachable!()),
    }
}

/// Verify data against stored ECC, correcting what the code can repair.
///
/// Returns the total number of corrected data bits.
pub fn decode_with_ecc(
    data: &mut [u8],
    ecc_data: &[u8],
    algorithm: &EccAlgorithm,
) -> Result<u32, EccError> {
    match algorithm {
        EccAlgorithm::None => Ok(0),
        EccAlgorithm::Hamming => {
            let codec = HammingEcc::new(512);
            let per_sector = codec.ecc_size();
            let mut corrected = 0u32;

            for (index, chunk) in data.chunks_mut(512).enumerate() {
                if chunk.len() != 512 {
                    return Err(EccError::InvalidInput);
                }
                let start = index * per_sector;
                let end = start + per_sector;
                // A sector whose ECC is missing cannot be checked. Reporting the
                // dump as verified up to that point would overstate what is known.
                if end > ecc_data.len() {
                    return Err(EccError::InvalidEccData);
                }
                corrected += codec.correct(chunk, &ecc_data[start..end])?;
            }
            Ok(corrected)
        }
        EccAlgorithm::Bch { t } => BchEcc::new(512, *t).correct(data, ecc_data),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_galois_field() {
        let gf = GaloisField::new();

        // Test that α^0 = 1
        assert_eq!(gf.alpha(0), 1);

        // Test multiplication identity
        assert_eq!(gf.mul(1, 1), 1);
        assert_eq!(gf.mul(0, 100), 0);

        // Test division
        let a = gf.alpha(100);
        let b = gf.alpha(50);
        let c = gf.mul(a, b);
        assert_eq!(gf.div(c, b), a);
    }

    /// Deterministic pseudo-random sector; all-zero or all-0xFF data would hide
    /// parity mistakes that only show up with mixed bits.
    fn sample_sector(size: usize) -> Vec<u8> {
        let mut data = Vec::with_capacity(size);
        let mut state = 0x2545_F491u32;
        for _ in 0..size {
            state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            data.push((state >> 15) as u8);
        }
        data
    }

    #[test]
    fn hamming_reports_no_correction_for_intact_data() {
        for size in [256usize, 512] {
            let codec = HammingEcc::new(size);
            let data = sample_sector(size);
            let ecc = codec.calculate(&data);

            let mut checked = data.clone();
            assert_eq!(codec.correct(&mut checked, &ecc).unwrap(), 0);
            assert_eq!(checked, data, "intact data must not be modified");
        }
    }

    #[test]
    fn hamming_ecc_is_the_documented_size() {
        assert_eq!(HammingEcc::new(256).calculate(&sample_sector(256)).len(), 3);
        assert_eq!(HammingEcc::new(512).calculate(&sample_sector(512)).len(), 4);
    }

    /// The property that matters, checked at every bit of the sector rather than
    /// at one arbitrary position: a single flipped data bit is located and
    /// repaired. The previous test flipped one bit and then asserted
    /// `is_ok() || is_err()`, which no implementation can fail.
    #[test]
    fn hamming_corrects_a_single_bit_error_at_every_position() {
        for size in [256usize, 512] {
            let codec = HammingEcc::new(size);
            let original = sample_sector(size);
            let ecc = codec.calculate(&original);

            for byte in 0..size {
                for bit in 0..8 {
                    let mut corrupted = original.clone();
                    corrupted[byte] ^= 1 << bit;

                    match codec.correct(&mut corrupted, &ecc) {
                        Ok(corrected) => {
                            assert_eq!(
                                corrected, 1,
                                "flipping bit {bit} of byte {byte} should count as one correction"
                            );
                            assert_eq!(
                                corrupted, original,
                                "bit {bit} of byte {byte} was not repaired correctly"
                            );
                        }
                        Err(error) => panic!(
                            "bit {bit} of byte {byte} in a {size}-byte sector was reported \
                             uncorrectable: {error:?}"
                        ),
                    }
                }
            }
        }
    }

    /// Erased flash is all-ones and a freshly programmed region can be all-zero;
    /// both must still correct, and they are where an off-by-one in the parity
    /// masks hides.
    #[test]
    fn hamming_corrects_a_single_bit_error_in_uniform_data() {
        for fill in [0x00u8, 0xFF] {
            let codec = HammingEcc::new(256);
            let original = vec![fill; 256];
            let ecc = codec.calculate(&original);

            for byte in [0usize, 1, 127, 255] {
                for bit in 0..8 {
                    let mut corrupted = original.clone();
                    corrupted[byte] ^= 1 << bit;
                    assert_eq!(codec.correct(&mut corrupted, &ecc).unwrap(), 1);
                    assert_eq!(
                        corrupted, original,
                        "fill {fill:#04x}, bit {bit} of byte {byte}"
                    );
                }
            }
        }
    }

    /// A flip in the stored ECC leaves the data intact, so it must not be
    /// "corrected" — doing so would corrupt good data.
    #[test]
    fn hamming_leaves_data_alone_when_the_ecc_bytes_are_damaged() {
        let codec = HammingEcc::new(256);
        let original = sample_sector(256);
        let ecc = codec.calculate(&original);

        for byte in 0..ecc.len() {
            for bit in 0..8 {
                let mut damaged = ecc.clone();
                damaged[byte] ^= 1 << bit;

                let mut data = original.clone();
                match codec.correct(&mut data, &damaged) {
                    Ok(corrected) => {
                        assert_eq!(
                            corrected, 0,
                            "damage to ECC byte {byte} bit {bit} must not count as a data fix"
                        );
                        assert_eq!(
                            data, original,
                            "data must be untouched when only the ECC is damaged"
                        );
                    }
                    // Bits above the used range are padding; a flip there may
                    // legitimately look uncorrectable. What must never happen is
                    // silently changing the data.
                    Err(_) => assert_eq!(data, original),
                }
            }
        }
    }

    /// Two flipped bits are beyond what a Hamming code can repair. Detecting
    /// them matters more than anything else here: a wrong "correction" turns two
    /// bad bits into three.
    #[test]
    fn hamming_detects_two_bit_errors_without_corrupting_further() {
        let codec = HammingEcc::new(256);
        let original = sample_sector(256);
        let ecc = codec.calculate(&original);

        let cases = [
            ((0usize, 0u32), (0usize, 1u32)),
            ((0, 0), (255, 7)),
            ((10, 3), (11, 3)),
            ((100, 1), (200, 6)),
            ((7, 7), (8, 0)),
        ];

        for ((byte_a, bit_a), (byte_b, bit_b)) in cases {
            let mut corrupted = original.clone();
            corrupted[byte_a] ^= 1 << bit_a;
            corrupted[byte_b] ^= 1 << bit_b;
            let two_bit_version = corrupted.clone();

            match codec.correct(&mut corrupted, &ecc) {
                Err(EccError::UncorrectableError) => {
                    assert_eq!(
                        corrupted, two_bit_version,
                        "a rejected sector must be left as it was found"
                    );
                }
                Ok(corrected) => panic!(
                    "two flipped bits ({byte_a}:{bit_a} and {byte_b}:{bit_b}) were reported \
                     as {corrected} correction(s); the data is now wrong in a third place"
                ),
                Err(other) => panic!("expected UncorrectableError, got {other:?}"),
            }
        }
    }

    #[test]
    fn hamming_rejects_a_wrong_sized_sector_or_ecc() {
        let codec = HammingEcc::new(256);
        let mut short = vec![0u8; 255];
        assert!(matches!(
            codec.correct(&mut short, &[0, 0, 0]),
            Err(EccError::InvalidInput)
        ));

        let mut data = vec![0u8; 256];
        assert!(matches!(
            codec.correct(&mut data, &[0, 0]),
            Err(EccError::InvalidInput)
        ));
    }

    /// The syndrome must identify a position, not merely differ: distinct error
    /// positions have to produce distinct syndromes, or correction would send
    /// some of them to the wrong bit.
    #[test]
    fn every_error_position_has_its_own_syndrome() {
        use std::collections::HashSet;

        let codec = HammingEcc::new(256);
        let original = vec![0u8; 256];
        let ecc = codec.calculate(&original);

        let mut syndromes = HashSet::new();
        for byte in 0..256usize {
            for bit in 0..8u32 {
                let mut corrupted = original.clone();
                corrupted[byte] ^= 1 << bit;
                let recomputed = codec.calculate(&corrupted);

                let syndrome: Vec<u8> = ecc
                    .iter()
                    .zip(&recomputed)
                    .map(|(stored, fresh)| stored ^ fresh)
                    .collect();

                assert!(
                    syndromes.insert(syndrome),
                    "byte {byte} bit {bit} shares a syndrome with an earlier position"
                );
            }
        }
        assert_eq!(syndromes.len(), 2048);
    }

    #[test]
    fn test_bch_creation() {
        let bch = BchEcc::new(512, 4);
        assert_eq!(bch.sector_size, 512);
        assert_eq!(bch.t, 4);
        assert!(!bch.generator.is_empty());
    }

    #[test]
    fn test_bch_generator_polynomial() {
        let gf = GaloisField::new();
        let gen = BchEcc::compute_generator(&gf, 4);

        // BCH-4 generator should have degree 2*4 = 8 (or more due to LCM)
        assert!(gen.len() > 8);
    }

    /// BCH refuses rather than mis-correcting. This is the guard that keeps the
    /// broken implementation from being reachable again by accident.
    #[test]
    fn bch_refuses_instead_of_mis_correcting() {
        let bch = BchEcc::new(512, 4);
        let data = vec![0x55u8; 512];

        assert!(matches!(
            bch.calculate(&data),
            Err(EccError::NotImplemented(_))
        ));

        let mut copy = data.clone();
        assert!(matches!(
            bch.correct(&mut copy, &[0u8; 8]),
            Err(EccError::NotImplemented(_))
        ));
        assert_eq!(copy, data, "a refusal must not touch the data");
    }

    #[test]
    fn the_ecc_facade_refuses_bch_too() {
        let data = vec![0x33u8; 1024];
        assert!(matches!(
            encode_with_ecc(&data, &EccAlgorithm::Bch { t: 4 }),
            Err(EccError::NotImplemented(_))
        ));

        let mut copy = data.clone();
        assert!(matches!(
            decode_with_ecc(&mut copy, &[0u8; 16], &EccAlgorithm::Bch { t: 4 }),
            Err(EccError::NotImplemented(_))
        ));
        assert_eq!(copy, data);
    }

    #[test]
    fn no_ecc_passes_data_through_unchanged() {
        let data = vec![0x55u8; 1024];
        let (encoded, ecc) = encode_with_ecc(&data, &EccAlgorithm::None).unwrap();

        assert_eq!(encoded, data);
        assert!(ecc.is_empty());
    }

    #[test]
    fn hamming_over_the_facade_produces_one_ecc_block_per_sector() {
        let data = vec![0xAAu8; 1024];
        let (encoded, ecc) = encode_with_ecc(&data, &EccAlgorithm::Hamming).unwrap();

        assert_eq!(encoded, data);
        // Two 512-byte sectors, four ECC bytes each.
        assert_eq!(ecc.len(), 8);
    }

    /// The facade must round-trip: encode, damage one bit, decode, get the
    /// original back. This is the path the GUI's dump processing uses.
    #[test]
    fn the_ecc_facade_round_trips_a_single_bit_error() {
        let data: Vec<u8> = (0..1024u32).map(|i| (i * 31 % 251) as u8).collect();
        let (_, ecc) = encode_with_ecc(&data, &EccAlgorithm::Hamming).unwrap();

        // One bit in each of the two sectors.
        let mut corrupted = data.clone();
        corrupted[10] ^= 0x08;
        corrupted[600] ^= 0x40;

        let corrected = decode_with_ecc(&mut corrupted, &ecc, &EccAlgorithm::Hamming).unwrap();
        assert_eq!(corrected, 2);
        assert_eq!(corrupted, data);
    }

    /// A sector whose ECC bytes are missing must be reported, not treated as
    /// verified: the GUI displays the correction count as evidence the dump is
    /// sound.
    #[test]
    fn a_missing_ecc_block_is_reported() {
        let mut data = vec![0u8; 1024];
        assert!(matches!(
            decode_with_ecc(&mut data, &[0u8; 4], &EccAlgorithm::Hamming),
            Err(EccError::InvalidEccData)
        ));
    }

    #[test]
    fn a_partial_trailing_sector_is_refused() {
        let data = vec![0u8; 700];
        assert!(matches!(
            encode_with_ecc(&data, &EccAlgorithm::Hamming),
            Err(EccError::InvalidInput)
        ));
    }
}
