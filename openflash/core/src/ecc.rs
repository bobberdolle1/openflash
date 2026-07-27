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
    /// A codec is present but not correct, so it refuses to run rather than
    /// return bytes it cannot vouch for.
    ///
    /// Nothing returns this today. It is kept because refusing is the right
    /// behaviour for a codec that is known to be wrong — silently returning
    /// mis-corrected flash contents is worse than returning an error — and the
    /// variant is what makes that refusal expressible.
    NotImplemented(&'static str),
}

// ============================================================================
// Galois field GF(2^m) for BCH
// ============================================================================

/// The primitive polynomial used for GF(2^m), as a bit pattern where bit `i` is
/// the coefficient of x^i.
///
/// Both are the conventional choices for NAND BCH. GF(2^13) holds a 512-byte
/// sector — 4096 data bits plus at most 13·t parity bits, comfortably under
/// 8191 — and GF(2^14) a 1024-byte one.
///
/// A non-primitive polynomial would still give a ring, but α would not generate
/// the whole multiplicative group, so some field elements would be unreachable
/// and the log table would have holes. That is a silent failure, so
/// `every_primitive_polynomial_generates_the_whole_field` checks the order of α
/// for each entry rather than trusting the constant.
fn primitive_polynomial(m: usize) -> u32 {
    match m {
        13 => 0x201B, // x^13 + x^4 + x^3 + x + 1
        14 => 0x4443, // x^14 + x^10 + x^6 + x + 1
        other => panic!("no primitive polynomial recorded for GF(2^{other})"),
    }
}

/// GF(2^m), with log and exponent tables for fast multiplication.
pub struct GaloisField {
    /// Extension degree.
    m: usize,
    /// Order of the multiplicative group, 2^m − 1.
    n: usize,
    /// `exp_table[i]` is α^i.
    exp_table: Vec<u16>,
    /// `log_table[x]` is the `i` with α^i = x; −1 for x = 0.
    log_table: Vec<i16>,
}

impl GaloisField {
    /// GF(2^13), which is what a 512-byte sector needs.
    pub fn new() -> Self {
        Self::with_degree(13)
    }

    /// GF(2^m) for one of the degrees in [`primitive_polynomial`].
    pub fn with_degree(m: usize) -> Self {
        let n = (1usize << m) - 1;
        let poly = primitive_polynomial(m);

        let mut exp_table = vec![0u16; n + 1];
        let mut log_table = vec![-1i16; n + 1];

        let mut x: u32 = 1;
        for (i, entry) in exp_table.iter_mut().enumerate().take(n) {
            *entry = x as u16;
            log_table[x as usize] = i as i16;

            x <<= 1;
            if x & (1 << m) != 0 {
                x ^= poly;
            }
        }
        exp_table[n] = exp_table[0];

        Self {
            m,
            n,
            exp_table,
            log_table,
        }
    }

    /// Extension degree `m`.
    pub fn degree(&self) -> usize {
        self.m
    }

    /// Order of the multiplicative group, 2^m − 1.
    pub fn order(&self) -> usize {
        self.n
    }

    #[inline]
    pub fn mul(&self, a: u16, b: u16) -> u16 {
        if a == 0 || b == 0 {
            return 0;
        }
        let log_a = self.log_table[a as usize] as usize;
        let log_b = self.log_table[b as usize] as usize;
        self.exp_table[(log_a + log_b) % self.n]
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
        self.exp_table[(log_a + self.n - log_b) % self.n]
    }

    #[inline]
    pub fn pow(&self, a: u16, n: usize) -> u16 {
        if a == 0 {
            return 0;
        }
        let log_a = self.log_table[a as usize] as usize;
        self.exp_table[(log_a * n) % self.n]
    }

    #[inline]
    pub fn alpha(&self, i: usize) -> u16 {
        self.exp_table[i % self.n]
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
// BCH ECC - binary BCH codes over GF(2^m)
// ============================================================================

/// Binary BCH code correcting up to `t` bit errors in a sector.
///
/// This is the code NAND actually uses: 4-, 8-, 16- or 24-bit BCH over a 512- or
/// 1024-byte sector. Hamming corrects one bit, which stopped being enough once
/// cells shrank.
///
/// # Layout and bit order
///
/// A codeword is the data bytes followed by the ECC bytes. Bit index 0 is the
/// *most* significant bit of `data[0]`, counting up through the data and then
/// through the ECC, and bit index `i` is the coefficient of x^(N−1−i) where
/// N = 8·sector + parity bits. When the parity length is not a whole number of
/// bytes the spare low bits of the last ECC byte are padding: they are written as
/// zero and ignored on decode, because they are not part of the codeword.
///
/// This convention is internally consistent, which is what matters for data this
/// crate encodes itself. It is *not* automatically the convention of any
/// particular flash controller: a hardware NAND controller picks its own bit
/// order, sector-to-spare mapping, and sometimes scrambles the data, so ECC
/// bytes lifted from a dump taken by such a controller will not generally verify
/// here. Matching a specific controller is a separate job from having a correct
/// BCH codec.
///
/// # Field choice
///
/// A codeword has to fit in the field: N must not exceed 2^m − 1. GF(2^13) holds
/// a 512-byte sector, GF(2^14) a 1024-byte one, and the smallest field that fits
/// is chosen automatically.
///
/// # What this replaces
///
/// The previous implementation did not work, and its failure mode was dangerous
/// rather than merely useless: over a 512-byte sector with `t = 4` it repaired
/// none of the 4096 single-bit errors, reported 4086 of them as uncorrectable
/// and "corrected" ten at the wrong bit, leaving the sector with two wrong bits
/// where it had one. It also emitted one ECC byte where BCH-4 over GF(2^13)
/// needs seven.
///
/// The root cause was in the generator polynomial. It computed
/// g(x) = ∏(x − α^i) with coefficients in GF(2^13), but a *binary* BCH code has
/// a generator over GF(2) — the LCM of the minimal polynomials of the roots —
/// and everything downstream inherited the mistake. The encoder and the syndrome
/// evaluation also disagreed about bit order, and the Chien search searched only
/// the data bits, so an error in the parity could never be located.
pub struct BchEcc {
    sector_size: usize,
    t: u8,
    gf: GaloisField,
    /// Generator polynomial over GF(2); `generator[d]` is the coefficient of
    /// x^d, so `generator[parity_bits]` is the leading 1.
    generator: Vec<u8>,
    /// Degree of the generator, and so the number of parity bits.
    parity_bits: usize,
}

impl BchEcc {
    /// Codec for `sector_size` bytes correcting `t` bit errors.
    ///
    /// # Panics
    ///
    /// If `t` is zero, or if no supported field is large enough for the sector.
    pub fn new(sector_size: usize, t: u8) -> Self {
        assert!(t > 0, "BCH needs t >= 1");
        let data_bits = sector_size * 8;

        for m in [13usize, 14] {
            let gf = GaloisField::with_degree(m);
            let generator = Self::compute_generator(&gf, t);
            let parity_bits = generator.len() - 1;

            if data_bits + parity_bits <= gf.order() {
                return Self {
                    sector_size,
                    t,
                    gf,
                    generator,
                    parity_bits,
                };
            }
        }

        panic!(
            "no supported Galois field is large enough for a {sector_size}-byte \
             sector with t = {t}"
        );
    }

    /// Generator polynomial over GF(2): the least common multiple of the minimal
    /// polynomials of α^1 … α^2t.
    ///
    /// Only odd powers need visiting. α^2i is a conjugate of α^i — squaring is
    /// the field automorphism here — so it shares a minimal polynomial and is
    /// picked up with the rest of its conjugacy class.
    ///
    /// Each minimal polynomial is built as ∏(x + α^j) over the class. That
    /// product is computed in GF(2^m), and every coefficient comes out as 0 or 1
    /// because the class is closed under squaring; the assertion below states
    /// that rather than assuming it, since a coefficient outside {0, 1} would
    /// mean the class was built wrongly and the result would not be a binary
    /// code at all.
    fn compute_generator(gf: &GaloisField, t: u8) -> Vec<u8> {
        let n = gf.order();
        let mut covered = vec![false; n];
        let mut generator = vec![1u8];

        let mut power = 1usize;
        while power <= 2 * t as usize {
            if !covered[power % n] {
                // The conjugacy class of α^power under squaring.
                let mut class = Vec::new();
                let mut j = power % n;
                loop {
                    class.push(j);
                    covered[j] = true;
                    j = (2 * j) % n;
                    if j == power % n {
                        break;
                    }
                }

                // Minimal polynomial of the class, as GF(2^m) coefficients.
                let mut minimal = vec![1u16];
                for &exponent in &class {
                    let root = gf.alpha(exponent);
                    let mut next = vec![0u16; minimal.len() + 1];
                    for (degree, &coefficient) in minimal.iter().enumerate() {
                        next[degree + 1] ^= coefficient; // x · minimal
                        next[degree] ^= gf.mul(coefficient, root); // α^j · minimal
                    }
                    minimal = next;
                }

                let binary: Vec<u8> = minimal
                    .iter()
                    .map(|&coefficient| {
                        assert!(
                            coefficient <= 1,
                            "minimal polynomial of a conjugacy class must have \
                             coefficients in GF(2), got {coefficient}"
                        );
                        coefficient as u8
                    })
                    .collect();

                generator = binary_polynomial_mul(&generator, &binary);
            }
            power += 2;
        }

        generator
    }

    /// Number of parity bits, which is the degree of the generator.
    pub fn parity_bits(&self) -> usize {
        self.parity_bits
    }

    /// Number of ECC bytes stored per sector.
    ///
    /// This is `ceil(parity_bits / 8)`, so 7 bytes for 4-bit BCH over 512 bytes,
    /// 13 for 8-bit and 26 for 16-bit — the sizes NAND datasheets quote.
    pub fn ecc_size(&self) -> usize {
        self.parity_bits.div_ceil(8)
    }

    /// Total codeword length in bits, data plus parity.
    fn codeword_bits(&self) -> usize {
        self.sector_size * 8 + self.parity_bits
    }

    /// Compute the ECC bytes for a sector.
    pub fn calculate(&self, data: &[u8]) -> Result<Vec<u8>, EccError> {
        if data.len() != self.sector_size {
            return Err(EccError::InvalidInput);
        }

        let parity = self.parity_bits;
        let mut remainder = vec![0u8; parity];

        // remainder := (message(x) · x^parity) mod generator(x), computed by
        // feeding the message bits high-degree first and then `parity` zeros.
        let feed = |bit: u8, remainder: &mut Vec<u8>| {
            let overflow = remainder[parity - 1];
            for index in (1..parity).rev() {
                remainder[index] = remainder[index - 1];
            }
            remainder[0] = bit;
            if overflow != 0 {
                for (index, slot) in remainder.iter_mut().enumerate() {
                    *slot ^= self.generator[index];
                }
            }
        };

        for &byte in data {
            for shift in (0..8).rev() {
                feed((byte >> shift) & 1, &mut remainder);
            }
        }
        for _ in 0..parity {
            feed(0, &mut remainder);
        }

        // ECC bit j is the coefficient of x^(parity-1-j).
        let mut ecc = vec![0u8; self.ecc_size()];
        for j in 0..parity {
            if remainder[parity - 1 - j] != 0 {
                ecc[j / 8] |= 1 << (7 - j % 8);
            }
        }
        Ok(ecc)
    }

    /// Whether codeword bit `index` is set, given the data and ECC bytes.
    #[inline]
    fn codeword_bit(&self, data: &[u8], ecc: &[u8], index: usize) -> u8 {
        let data_bits = self.sector_size * 8;
        if index < data_bits {
            (data[index / 8] >> (7 - index % 8)) & 1
        } else {
            let j = index - data_bits;
            (ecc[j / 8] >> (7 - j % 8)) & 1
        }
    }

    /// Syndromes S_l = c(α^l) for l = 1 … 2t.
    ///
    /// All 2t are evaluated directly. For a binary code the even ones are
    /// determined by the odd ones (S_2l = S_l²), but computing them is cheap
    /// beside the Chien search and it keeps the Berlekamp-Massey input plain.
    fn syndromes(&self, data: &[u8], ecc: &[u8]) -> Vec<u16> {
        let total = self.codeword_bits();
        let mut syndromes = vec![0u16; 2 * self.t as usize];

        for index in 0..total {
            if self.codeword_bit(data, ecc, index) == 0 {
                continue;
            }
            let exponent = total - 1 - index;
            for (l, syndrome) in syndromes.iter_mut().enumerate() {
                *syndrome ^= self.gf.alpha((l + 1) * exponent);
            }
        }

        syndromes
    }

    /// Berlekamp-Massey: the shortest error-locator polynomial consistent with
    /// the syndromes. Returns σ(x) and its degree, the number of errors it
    /// claims.
    fn berlekamp_massey(&self, syndromes: &[u16]) -> (Vec<u16>, usize) {
        let size = 2 * self.t as usize + 2;
        let mut sigma = vec![0u16; size];
        sigma[0] = 1;
        let mut previous = vec![0u16; size];
        previous[0] = 1;

        let mut errors = 0usize;
        let mut shift = 1usize;
        let mut previous_discrepancy = 1u16;

        for round in 0..syndromes.len() {
            let mut discrepancy = syndromes[round];
            for i in 1..=errors {
                discrepancy ^= self.gf.mul(sigma[i], syndromes[round - i]);
            }

            if discrepancy == 0 {
                shift += 1;
                continue;
            }

            let before = sigma.clone();
            let scale = self.gf.div(discrepancy, previous_discrepancy);
            for i in 0..size.saturating_sub(shift) {
                if previous[i] != 0 {
                    sigma[i + shift] ^= self.gf.mul(scale, previous[i]);
                }
            }

            if 2 * errors <= round {
                errors = round + 1 - errors;
                previous = before;
                previous_discrepancy = discrepancy;
                shift = 1;
            } else {
                shift += 1;
            }
        }

        sigma.truncate(errors + 1);
        (sigma, errors)
    }

    /// Chien search: the exponents e for which σ(α^−e) = 0, which are the error
    /// positions.
    ///
    /// Only exponents inside the codeword are searched. The code is shortened —
    /// 4148 bits of a possible 8191 for a 512-byte sector with t = 4 — and a root
    /// outside that range cannot be a real error position.
    fn chien_search(&self, sigma: &[u16]) -> Vec<usize> {
        let order = self.gf.order();
        let mut positions = Vec::new();

        for exponent in 0..self.codeword_bits() {
            let inverse = (order - exponent % order) % order;
            let mut value = 0u16;
            for (degree, &coefficient) in sigma.iter().enumerate() {
                if coefficient != 0 {
                    value ^= self.gf.mul(coefficient, self.gf.alpha(inverse * degree));
                }
            }
            if value == 0 {
                positions.push(exponent);
            }
        }

        positions
    }

    /// Verify a sector against its stored ECC and repair what the code can.
    ///
    /// Returns the number of corrected bits across the whole codeword, so a flip
    /// in the stored ECC counts too — that is a real bit error in the spare area
    /// and worth reporting, even though the data was intact.
    ///
    /// `data` is left untouched unless the correction is confirmed. After
    /// flipping the located bits the syndromes are recomputed and must all
    /// vanish; if they do not, the sector is reported as uncorrectable and
    /// nothing is written back. A BCH decoder handed more than `t` errors can
    /// otherwise land on a valid-looking but wrong codeword, and writing that
    /// out would corrupt a dump while reporting success.
    pub fn correct(&self, data: &mut [u8], stored_ecc: &[u8]) -> Result<u32, EccError> {
        if data.len() != self.sector_size {
            return Err(EccError::InvalidInput);
        }
        if stored_ecc.len() < self.ecc_size() {
            return Err(EccError::InvalidEccData);
        }

        let syndromes = self.syndromes(data, stored_ecc);
        if syndromes.iter().all(|&s| s == 0) {
            return Ok(0);
        }

        let (sigma, errors) = self.berlekamp_massey(&syndromes);
        if errors == 0 || errors > self.t as usize {
            return Err(EccError::UncorrectableError);
        }

        let positions = self.chien_search(&sigma);
        if positions.len() != errors {
            return Err(EccError::UncorrectableError);
        }

        // Apply to copies, so a failed verification leaves the caller's buffer
        // exactly as it was.
        let mut fixed_data = data.to_vec();
        let mut fixed_ecc = stored_ecc[..self.ecc_size()].to_vec();
        let total = self.codeword_bits();
        let data_bits = self.sector_size * 8;

        for exponent in &positions {
            let index = total - 1 - exponent;
            if index < data_bits {
                fixed_data[index / 8] ^= 1 << (7 - index % 8);
            } else {
                let j = index - data_bits;
                fixed_ecc[j / 8] ^= 1 << (7 - j % 8);
            }
        }

        if self
            .syndromes(&fixed_data, &fixed_ecc)
            .iter()
            .any(|&s| s != 0)
        {
            return Err(EccError::UncorrectableError);
        }

        data.copy_from_slice(&fixed_data);
        Ok(positions.len() as u32)
    }
}

/// Multiply two polynomials over GF(2), where index = degree and each
/// coefficient is 0 or 1.
fn binary_polynomial_mul(a: &[u8], b: &[u8]) -> Vec<u8> {
    let mut product = vec![0u8; a.len() + b.len() - 1];
    for (i, &ai) in a.iter().enumerate() {
        if ai == 0 {
            continue;
        }
        for (j, &bj) in b.iter().enumerate() {
            if bj != 0 {
                product[i + j] ^= 1;
            }
        }
    }
    product
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
        EccAlgorithm::Bch { t } => {
            let codec = BchEcc::new(512, *t);
            let mut all_ecc = Vec::new();
            for chunk in data.chunks(512) {
                if chunk.len() != 512 {
                    return Err(EccError::InvalidInput);
                }
                all_ecc.extend(codec.calculate(chunk)?);
            }
            Ok((data.to_vec(), all_ecc))
        }
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
        EccAlgorithm::Bch { t } => {
            let codec = BchEcc::new(512, *t);
            let per_sector = codec.ecc_size();
            let mut corrected = 0u32;

            for (index, chunk) in data.chunks_mut(512).enumerate() {
                if chunk.len() != 512 {
                    return Err(EccError::InvalidInput);
                }
                let start = index * per_sector;
                let end = start + per_sector;
                if end > ecc_data.len() {
                    return Err(EccError::InvalidEccData);
                }
                corrected += codec.correct(chunk, &ecc_data[start..end])?;
            }
            Ok(corrected)
        }
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

    // ------------------------------------------------------------------
    // BCH
    // ------------------------------------------------------------------

    /// A field built on a non-primitive polynomial has holes: α fails to reach
    /// every element, and the log table keeps its −1 sentinel where it should
    /// hold an exponent. Every multiplication through such a hole is wrong, so
    /// the constants are checked rather than trusted.
    #[test]
    fn every_primitive_polynomial_generates_the_whole_field() {
        for m in [13usize, 14] {
            let n = (1usize << m) - 1;
            let poly = primitive_polynomial(m);

            let mut seen = vec![false; 1usize << m];
            let mut x: u32 = 1;
            for step in 0..n {
                assert!(
                    !seen[x as usize],
                    "GF(2^{m}) with 0x{poly:04X}: α^{step} repeats an earlier \
                     element, so the polynomial is not primitive"
                );
                seen[x as usize] = true;
                x <<= 1;
                if x & (1 << m) != 0 {
                    x ^= poly;
                }
            }
            assert_eq!(x, 1, "GF(2^{m}): α^{n} must return to 1");

            // And the table the codec actually uses has no gaps.
            let gf = GaloisField::with_degree(m);
            for element in 1..=n {
                assert_ne!(
                    gf.log_table[element], -1,
                    "GF(2^{m}): element {element} has no logarithm"
                );
            }
        }
    }

    /// The generator's degree is the parity length, and the resulting ECC sizes
    /// are the ones NAND datasheets quote for these configurations: 7 bytes for
    /// 4-bit BCH over 512 bytes, 13 for 8-bit, 26 for 16-bit, and 42 for 24-bit
    /// over 1024 bytes. Getting a different number here means the generator is
    /// not the LCM of the right minimal polynomials — which is exactly how the
    /// previous implementation went wrong, emitting a single ECC byte.
    #[test]
    fn parity_length_matches_the_published_nand_ecc_sizes() {
        for (sector, t, expected_bytes) in [
            (512usize, 4u8, 7usize),
            (512, 8, 13),
            (512, 16, 26),
            (1024, 24, 42),
        ] {
            let codec = BchEcc::new(sector, t);
            assert_eq!(
                codec.parity_bits(),
                codec.gf.degree() * t as usize,
                "{sector}-byte sector, t={t}: parity should be m·t"
            );
            assert_eq!(
                codec.ecc_size(),
                expected_bytes,
                "{sector}-byte sector, t={t}: ECC size"
            );
        }
    }

    /// A 1024-byte sector does not fit in GF(2^13) — 8192 data bits already
    /// exceed the 8191 non-zero elements — so it has to move up to GF(2^14).
    #[test]
    fn the_field_grows_with_the_sector() {
        assert_eq!(BchEcc::new(512, 4).gf.degree(), 13);
        assert_eq!(BchEcc::new(1024, 24).gf.degree(), 14);
    }

    /// The defining property: α^1 … α^2t are roots of the generator. This is
    /// what makes the syndromes of a clean codeword vanish, and it is checked
    /// directly rather than inferred from the round trip working.
    #[test]
    fn the_generator_has_the_required_roots() {
        let codec = BchEcc::new(512, 4);

        for power in 1..=2 * codec.t as usize {
            let root = codec.gf.alpha(power);
            let mut value = 0u16;
            for (degree, &coefficient) in codec.generator.iter().enumerate() {
                if coefficient != 0 {
                    value ^= codec.gf.pow(root, degree);
                }
            }
            assert_eq!(value, 0, "α^{power} must be a root of the generator");
        }
    }

    #[test]
    fn a_clean_codeword_has_zero_syndromes() {
        let codec = BchEcc::new(512, 4);
        let data = sample_sector(512);
        let ecc = codec.calculate(&data).unwrap();

        assert_eq!(ecc.len(), 7);
        assert!(
            codec.syndromes(&data, &ecc).iter().all(|&s| s == 0),
            "an undamaged codeword must have no syndrome"
        );

        // And decoding reports nothing to fix, without touching the data.
        let mut copy = data.clone();
        assert_eq!(codec.correct(&mut copy, &ecc).unwrap(), 0);
        assert_eq!(copy, data);
    }

    /// Flip each bit of the codeword in turn — data *and* parity — and require
    /// the original back. The previous implementation repaired none of these and
    /// mis-corrected ten, so this is the test that matters.
    #[test]
    fn every_single_bit_error_in_the_codeword_is_repaired() {
        let codec = BchEcc::new(512, 4);
        let data = sample_sector(512);
        let ecc = codec.calculate(&data).unwrap();
        let total = codec.codeword_bits();
        let data_bits = 512 * 8;

        for index in 0..total {
            let mut corrupted = data.clone();
            let mut damaged_ecc = ecc.clone();
            if index < data_bits {
                corrupted[index / 8] ^= 1 << (7 - index % 8);
            } else {
                let j = index - data_bits;
                damaged_ecc[j / 8] ^= 1 << (7 - j % 8);
            }

            let corrected = codec
                .correct(&mut corrupted, &damaged_ecc)
                .unwrap_or_else(|e| panic!("codeword bit {index}: {e:?}"));

            assert_eq!(corrected, 1, "codeword bit {index}: one flip, one repair");
            assert_eq!(
                corrupted, data,
                "codeword bit {index}: repaired to the wrong value"
            );
        }
    }

    /// A deterministic spread of error patterns of every weight up to `t`. Fixed
    /// seed, so a failure is reproducible.
    #[test]
    fn error_patterns_up_to_t_are_repaired() {
        let codec = BchEcc::new(512, 4);
        let data = sample_sector(512);
        let ecc = codec.calculate(&data).unwrap();
        let total = codec.codeword_bits();
        let data_bits = 512 * 8;

        let mut rng = 0x5EED_1234u64;
        let mut next = move || {
            rng = rng
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            (rng >> 33) as usize
        };

        for weight in 2..=codec.t as usize {
            for attempt in 0..120 {
                let mut positions = Vec::new();
                while positions.len() < weight {
                    let candidate = next() % total;
                    if !positions.contains(&candidate) {
                        positions.push(candidate);
                    }
                }

                let mut corrupted = data.clone();
                let mut damaged_ecc = ecc.clone();
                for &index in &positions {
                    if index < data_bits {
                        corrupted[index / 8] ^= 1 << (7 - index % 8);
                    } else {
                        let j = index - data_bits;
                        damaged_ecc[j / 8] ^= 1 << (7 - j % 8);
                    }
                }

                let corrected = codec
                    .correct(&mut corrupted, &damaged_ecc)
                    .unwrap_or_else(|e| {
                        panic!("weight {weight} attempt {attempt} at {positions:?}: {e:?}")
                    });

                assert_eq!(corrected as usize, weight);
                assert_eq!(
                    corrupted, data,
                    "weight {weight} attempt {attempt} at {positions:?}: wrong result"
                );
            }
        }
    }

    /// Beyond `t` the code cannot repair, and the one outcome that must never
    /// happen is a confident wrong answer. Either it reports uncorrectable, or —
    /// if the pattern happens to land on another valid codeword — it returns
    /// data that verifies; what it must not do is hand back a sector that is
    /// neither the original nor consistent with its parity.
    #[test]
    fn more_errors_than_t_are_not_mis_corrected() {
        let codec = BchEcc::new(512, 4);
        let data = sample_sector(512);
        let ecc = codec.calculate(&data).unwrap();
        let total = codec.codeword_bits();
        let data_bits = 512 * 8;

        let mut rng = 0xC0FF_EE11u64;
        let mut next = move || {
            rng = rng
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            (rng >> 33) as usize
        };

        let mut detected = 0;
        let mut silently_wrong = 0;

        for _ in 0..200 {
            let weight = codec.t as usize + 1;
            let mut positions = Vec::new();
            while positions.len() < weight {
                let candidate = next() % total;
                if !positions.contains(&candidate) {
                    positions.push(candidate);
                }
            }

            let mut corrupted = data.clone();
            let mut damaged_ecc = ecc.clone();
            for &index in &positions {
                if index < data_bits {
                    corrupted[index / 8] ^= 1 << (7 - index % 8);
                } else {
                    let j = index - data_bits;
                    damaged_ecc[j / 8] ^= 1 << (7 - j % 8);
                }
            }

            match codec.correct(&mut corrupted, &damaged_ecc) {
                Err(EccError::UncorrectableError) => {
                    detected += 1;
                    // A refusal must leave the caller's buffer untouched, so the
                    // caller still has the raw bytes to record or retry with.
                    let mut expected = data.clone();
                    for &index in &positions {
                        if index < data_bits {
                            expected[index / 8] ^= 1 << (7 - index % 8);
                        }
                    }
                    assert_eq!(corrupted, expected, "a refusal must not rewrite data");
                }
                Ok(_) => {
                    if corrupted != data {
                        silently_wrong += 1;
                    }
                }
                Err(other) => panic!("unexpected error: {other:?}"),
            }
        }

        assert_eq!(
            silently_wrong, 0,
            "{silently_wrong} sectors were corrected to something that is neither \
             the original nor detectably wrong"
        );
        assert!(
            detected > 190,
            "only {detected} of 200 five-bit errors were detected by a t=4 code"
        );
    }

    /// Damaging the data without touching the ECC is what a real bit-rot looks
    /// like, and 8-bit BCH is the common configuration on modern parts.
    #[test]
    fn bch_8_repairs_eight_scattered_data_bits() {
        let codec = BchEcc::new(512, 8);
        let data = sample_sector(512);
        let ecc = codec.calculate(&data).unwrap();
        assert_eq!(ecc.len(), 13);

        let mut corrupted = data.clone();
        for (offset, bit) in [
            (0usize, 0u32),
            (7, 3),
            (63, 7),
            (128, 1),
            (200, 5),
            (301, 2),
            (400, 6),
            (511, 4),
        ] {
            corrupted[offset] ^= 1 << bit;
        }

        assert_eq!(codec.correct(&mut corrupted, &ecc).unwrap(), 8);
        assert_eq!(corrupted, data);
    }

    #[test]
    fn bch_rejects_a_wrong_sized_sector_or_ecc() {
        let codec = BchEcc::new(512, 4);
        assert!(matches!(
            codec.calculate(&[0u8; 256]),
            Err(EccError::InvalidInput)
        ));

        let mut short = [0u8; 256];
        assert!(matches!(
            codec.correct(&mut short, &[0u8; 7]),
            Err(EccError::InvalidInput)
        ));

        let mut sector = [0u8; 512];
        assert!(matches!(
            codec.correct(&mut sector, &[0u8; 3]),
            Err(EccError::InvalidEccData)
        ));
    }

    /// The facade is what the GUI's dump processing calls, so it has to split
    /// into sectors and keep the ECC blocks lined up.
    #[test]
    fn the_bch_facade_round_trips_across_several_sectors() {
        let data: Vec<u8> = (0..2048u32).map(|i| (i * 37 % 253) as u8).collect();
        let (encoded, ecc) = encode_with_ecc(&data, &EccAlgorithm::Bch { t: 4 }).unwrap();

        assert_eq!(encoded, data);
        // Four 512-byte sectors, 7 ECC bytes each.
        assert_eq!(ecc.len(), 28);

        // One bit in every sector, at a different place each time.
        let mut corrupted = data.clone();
        corrupted[3] ^= 0x01;
        corrupted[600] ^= 0x80;
        corrupted[1100] ^= 0x10;
        corrupted[2000] ^= 0x40;

        let corrected = decode_with_ecc(&mut corrupted, &ecc, &EccAlgorithm::Bch { t: 4 }).unwrap();
        assert_eq!(corrected, 4);
        assert_eq!(corrupted, data);
    }

    #[test]
    fn the_bch_facade_reports_a_missing_ecc_block() {
        let data = vec![0x5Au8; 1024];
        let (_, ecc) = encode_with_ecc(&data, &EccAlgorithm::Bch { t: 4 }).unwrap();

        // Only the first sector's ECC survives.
        let mut copy = data.clone();
        assert!(matches!(
            decode_with_ecc(&mut copy, &ecc[..7], &EccAlgorithm::Bch { t: 4 }),
            Err(EccError::InvalidEccData)
        ));
    }

    /// A trailing partial sector cannot be protected by a code defined over a
    /// fixed sector size, and quietly leaving it uncovered would make a later
    /// decode read the wrong ECC bytes for every following sector.
    #[test]
    fn the_bch_facade_refuses_a_partial_trailing_sector() {
        let data = vec![0u8; 600];
        assert!(matches!(
            encode_with_ecc(&data, &EccAlgorithm::Bch { t: 4 }),
            Err(EccError::InvalidInput)
        ));
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
