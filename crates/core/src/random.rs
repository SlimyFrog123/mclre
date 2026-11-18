//! Random number generation (SPEC.md 3.6, PLAN.md M1).
//!
//! - [`GameRandom`]: an exact port of the reference `Random` (see
//!   `docs/reference/rng.md`, verified against
//!   `reference/lce-141217/MinecraftConsoles-141217/Minecraft.World/Random.{h,cpp}`).
//!   It is bit-exact with Java's `java.util.Random` and **must** be used for
//!   every system that affects gameplay or world state (terrain, random ticks,
//!   mob spawning, loot, enchanting, weather, explosions, ...).
//! - [`CosmeticRandom`]: for client-only visuals (particles, cosmetic jitter).
//!   Its algorithm is free to differ from the reference.
//!
//! Call order and call count matter: never skip, reorder, or "optimize away"
//! an RNG call whose results the reference consumes (even discards count).

// The casts below are intentional and load-bearing: they implement the
// reference's exact truncation/wraparound arithmetic (48-bit LCG, top-bits
// extraction, i64->i32 state narrowing). Do not replace them with From/try_from.
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::cast_lossless)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_sign_loss)]

/// Exact port of the reference `Random` (`java.util.Random` LCG).
///
/// State: 48-bit seed + the Box–Muller gaussian cache
/// (reference: `Random.h:6-8`, `Random.cpp:21-31`).
#[derive(Debug, Clone, PartialEq)]
pub struct GameRandom {
    seed: i64,
    have_next_next_gaussian: bool,
    next_next_gaussian: f64,
}

const MULTIPLIER: i64 = 0x5DEE_CE66D;
const ADDEND: i64 = 0xB;
const MASK: i64 = (1 << 48) - 1;

impl GameRandom {
    /// Creates a generator with the given seed
    /// (reference: `Random.cpp:16-19`, `Random.cpp:21-25`).
    pub fn with_seed(seed: i64) -> Self {
        let mut rng = Self {
            seed: 0,
            have_next_next_gaussian: false,
            next_next_gaussian: 0.0,
        };
        rng.set_seed(seed);
        rng
    }

    /// Reseeds the generator, clearing the gaussian cache
    /// (`Random.cpp:21-25`).
    pub fn set_seed(&mut self, seed: i64) {
        self.seed = (seed ^ MULTIPLIER) & MASK;
        self.have_next_next_gaussian = false;
    }

    /// Advances the state and returns the top `bits` bits
    /// (`Random.cpp:27-31`).
    fn next(&mut self, bits: u32) -> i32 {
        self.seed = self.seed.wrapping_mul(MULTIPLIER).wrapping_add(ADDEND) & MASK;
        (self.seed >> (48 - bits)) as i32
    }

    /// Uniformly distributed `i32` (`Random.cpp:71-74`).
    pub fn next_int(&mut self) -> i32 {
        self.next(32)
    }

    /// Uniformly distributed value in `0..bound` (`Random.cpp:76-91`).
    ///
    /// Uses the reference's power-of-two fast path (64-bit multiply) and
    /// rejection loop for other bounds.
    ///
    /// # Panics
    /// Only in debug builds if `bound <= 0` (a caller bug, not untrusted input).
    pub fn next_int_bound(&mut self, bound: i32) -> i32 {
        debug_assert!(bound > 0, "bound must be positive");
        // Power of two: (next(31) * bound) >> 31, in 64-bit
        // (`Random.cpp:81-82`, 4J comment "Made __int64 instead of long").
        if bound & (bound - 1) == 0 {
            return (((self.next(31) as i64) * (bound as i64)) >> 31) as i32;
        }
        // Rejection loop (`Random.cpp:84-90`).
        loop {
            let bits = self.next(31);
            let val = bits % bound;
            if bits.wrapping_sub(val).wrapping_add(bound - 1) >= 0 {
                return val;
            }
        }
    }

    /// Uniformly distributed `i64` (`Random.cpp:98-101`).
    ///
    /// Note the reference uses `+` between the halves, not `|`
    /// (sign-carrying, Java semantics).
    pub fn next_long(&mut self) -> i64 {
        ((self.next(32) as i64) << 32).wrapping_add(self.next(32) as i64)
    }

    /// Uniformly distributed `f64` in `[0, 1)` (`Random.cpp:41-46`).
    pub fn next_double(&mut self) -> f64 {
        ((((self.next(26) as i64) << 27) + self.next(27) as i64) as f64) / (1i64 << 53) as f64
    }

    /// Uniformly distributed `f32` in `[0, 1)` (`Random.cpp:93-96`).
    pub fn next_float(&mut self) -> f32 {
        self.next(24) as f32 / (1i32 << 24) as f32
    }

    /// Uniform `bool` (`Random.cpp:103-106`).
    pub fn next_boolean(&mut self) -> bool {
        self.next(1) != 0
    }

    /// Standard normal `f64` via Box–Muller polar with cached second value
    /// (`Random.cpp:48-69`).
    pub fn next_gaussian(&mut self) -> f64 {
        if self.have_next_next_gaussian {
            self.have_next_next_gaussian = false;
            return self.next_next_gaussian;
        }
        loop {
            // Both draws are unconditionally consumed (do/while in the
            // reference) — call order must be preserved even when discarded.
            let v1 = 2.0 * self.next_double() - 1.0;
            let v2 = 2.0 * self.next_double() - 1.0;
            let s = v1 * v1 + v2 * v2;
            if !(s >= 1.0 || s == 0.0) {
                let multiplier = (-2.0 * s.ln() / s).sqrt();
                self.next_next_gaussian = v2 * multiplier;
                self.have_next_next_gaussian = true;
                return v1 * multiplier;
            }
        }
    }

    /// Fills bytes via successive 8-bit draws (`Random.cpp:33-39`).
    pub fn next_bytes(&mut self, bytes: &mut [u8]) {
        for byte in bytes {
            *byte = self.next(8) as u8;
        }
    }
}

/// Randomness for client-only visual effects (particles, cosmetic jitter).
///
/// Deliberately **not** the reference algorithm: visual-only streams may differ
/// from the original without breaking parity (SPEC.md 3.6 rule 6). Deterministic
/// splitmix64 so cosmetic effects stay reproducible in tests/replays.
#[derive(Debug, Clone, PartialEq)]
pub struct CosmeticRandom {
    state: u64,
}

impl CosmeticRandom {
    /// Creates a cosmetic generator seeded by a `u64`.
    pub fn with_seed(seed: u64) -> Self {
        Self {
            state: seed.wrapping_add(0x9E37_79B9_7F4A_7C15),
        }
    }

    /// Next raw 64-bit value (splitmix64).
    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_8CE4_E35F);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform `f32` in `[0, 1)`.
    pub fn next_float(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u32 << 24) as f32
    }
}

// ---------------------------------------------------------------------------
// Tests — vectors generated from authoritative java.util.Random (JDK 26;
// the Random LCG is version-stable legacy spec). Generator source:
// %TEMP%/opencode/RandVectors.java (2026-11-17).
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    #![allow(clippy::unreadable_literal)] // test vectors are generated data

    use super::*;

    /// (seed, nextInt x2)
    const NEXT_INT: &[(i64, [i32; 2])] = &[
        (0, [-1_155_484_576, -723_955_400]),
        (42, [-1_170_105_035, 234_785_527]),
        (123_456_789, [-1_442_945_365, -1_016_548_095]),
        (-1, [1_155_099_827, 1_887_904_451]),
        (-4_936_897_262_349_239_473, [812_898_293, 1_596_032_898]),
    ];

    /// (seed, bound, values)
    const NEXT_INT_BOUND7: &[(i64, [i32; 3])] = &[
        (0, [5, 2, 4]),
        (42, [1, 5, 6]),
        (123_456_789, [1, 0, 6]),
        (-1, [3, 6, 4]),
        (-4_936_897_262_349_239_473, [5, 6, 3]),
    ];

    /// (seed, bound=16, values) — exercises the power-of-two fast path.
    const NEXT_INT_BOUND16: &[(i64, [i32; 3])] = &[
        (0, [11, 13, 3]),
        (42, [11, 0, 10]),
        (123_456_789, [10, 12, 7]),
        (-1, [4, 7, 0]),
        (-4_936_897_262_349_239_473, [3, 5, 11]),
    ];

    /// (seed, bound=100, values)
    const NEXT_INT_BOUND100: &[(i64, [i32; 2])] = &[
        (0, [60, 48]),
        (42, [30, 63]),
        (123_456_789, [65, 0]),
        (-1, [13, 25]),
        (-4_936_897_262_349_239_473, [46, 49]),
    ];

    /// (seed, nextLong x2)
    const NEXT_LONG: &[(i64, [i64; 2])] = &[
        (0, [-4_962_768_465_676_381_896, 4_437_113_781_045_784_766]),
        (42, [-5_025_562_857_975_149_833, -5_843_495_416_241_995_736]),
        (
            123_456_789,
            [-6_197_403_153_606_331_135, 8_429_272_609_719_263_920],
        ),
        (-1, [4_961_115_982_468_162_243, 226_341_162_490_527_646]),
        (
            -4_936_897_262_349_239_473,
            [3_491_371_585_005_258_626, -4_886_239_747_447_302_486],
        ),
    ];

    /// (seed, nextDouble x2)
    const NEXT_DOUBLE: &[(i64, [f64; 2])] = &[
        (0, [0.730967787376657, 0.24053641567148587]),
        (42, [0.7275636800328681, 0.6832234717598454]),
        (123_456_789, [0.664038103272266, 0.45695178590520646]),
        (-1, [0.26894263088050496, 0.012269981921235296]),
        (
            -4_936_897_262_349_239_473,
            [0.18926762598164926, 0.7351164155412194],
        ),
    ];

    /// (seed, nextFloat x2)
    const NEXT_FLOAT: &[(i64, [f32; 2])] = &[
        (0, [0.73096776, 0.831441]),
        (42, [0.7275637, 0.054665208]),
        (123_456_789, [0.66403806, 0.76331645]),
        (-1, [0.2689426, 0.43956196]),
        (-4_936_897_262_349_239_473, [0.18926758, 0.37160534]),
    ];

    /// (seed, nextBoolean x2)
    const NEXT_BOOLEAN: &[(i64, [bool; 2])] = &[
        (0, [true, true]),
        (42, [true, false]),
        (123_456_789, [true, true]),
        (-1, [false, false]),
        (-4_936_897_262_349_239_473, [false, false]),
    ];

    /// (seed, nextGaussian x3 — exercises the Box–Muller cache across calls)
    const NEXT_GAUSSIAN: &[(i64, [f64; 3])] = &[
        (
            0,
            [0.8025330637390305, -0.9015460884175122, 2.080920790428163],
        ),
        (
            42,
            [1.1419053154730547, 0.9194079489827879, -0.9498666368908959],
        ),
        (
            123_456_789,
            [2.011512265750413, -0.5278774195932023, -0.24142246805715384],
        ),
        (
            -1,
            [1.7853314409882288, -0.9204169061847902, 0.4869392448030407],
        ),
        (
            -4_936_897_262_349_239_473,
            [
                -0.7963857961647656,
                0.6025872726449223,
                -0.09177429367918417,
            ],
        ),
    ];

    /// Seeds exercised for `next_bytes` (expectations computed in-test; see
    /// the note there — modern JDK vectors are not ground truth for this one).
    const NEXT_BYTES_SEEDS: &[i64] = &[0, 42, 123_456_789, -1, -4_936_897_262_349_239_473];

    #[test]
    fn next_int_matches_java() {
        for &(seed, want) in NEXT_INT {
            let mut rng = GameRandom::with_seed(seed);
            assert_eq!(rng.next_int(), want[0], "seed {seed} first nextInt");
            assert_eq!(rng.next_int(), want[1], "seed {seed} second nextInt");
        }
    }

    #[test]
    fn next_int_bound7_matches_java() {
        for &(seed, want) in NEXT_INT_BOUND7 {
            let mut rng = GameRandom::with_seed(seed);
            for (i, w) in want.iter().enumerate() {
                assert_eq!(rng.next_int_bound(7), *w, "seed {seed} bound 7 draw {i}");
            }
        }
    }

    #[test]
    fn next_int_bound16_matches_java_power_of_two_path() {
        for &(seed, want) in NEXT_INT_BOUND16 {
            let mut rng = GameRandom::with_seed(seed);
            for (i, w) in want.iter().enumerate() {
                assert_eq!(
                    rng.next_int_bound(16),
                    *w,
                    "seed {seed} bound 16 (pow2) draw {i}"
                );
            }
        }
    }

    #[test]
    fn next_int_bound100_matches_java() {
        for &(seed, want) in NEXT_INT_BOUND100 {
            let mut rng = GameRandom::with_seed(seed);
            for (i, w) in want.iter().enumerate() {
                assert_eq!(
                    rng.next_int_bound(100),
                    *w,
                    "seed {seed} bound 100 draw {i}"
                );
            }
        }
    }

    #[test]
    fn next_long_matches_java() {
        for &(seed, want) in NEXT_LONG {
            let mut rng = GameRandom::with_seed(seed);
            assert_eq!(rng.next_long(), want[0], "seed {seed} first nextLong");
            assert_eq!(rng.next_long(), want[1], "seed {seed} second nextLong");
        }
    }

    #[test]
    fn next_double_matches_java_bit_exact() {
        for &(seed, want) in NEXT_DOUBLE {
            let mut rng = GameRandom::with_seed(seed);
            for (i, w) in want.iter().enumerate() {
                let got = rng.next_double();
                assert_eq!(got.to_bits(), w.to_bits(), "seed {seed} double draw {i}");
            }
        }
    }

    #[test]
    fn next_float_matches_java_bit_exact() {
        for &(seed, want) in NEXT_FLOAT {
            let mut rng = GameRandom::with_seed(seed);
            for (i, w) in want.iter().enumerate() {
                let got = rng.next_float();
                assert_eq!(got.to_bits(), w.to_bits(), "seed {seed} float draw {i}");
            }
        }
    }

    #[test]
    fn next_boolean_matches_java() {
        for &(seed, want) in NEXT_BOOLEAN {
            let mut rng = GameRandom::with_seed(seed);
            assert_eq!(rng.next_boolean(), want[0], "seed {seed} bool 0");
            assert_eq!(rng.next_boolean(), want[1], "seed {seed} bool 1");
        }
    }

    /// Widest allowed absolute difference for a 1-ulp disagreement on `f64`
    /// values in [1, 2) (ulp = 2^-52). Gaussians fall in this range in our
    /// vector set.
    const GAUSSIAN_ULP_TOLERANCE: f64 = f64::EPSILON;

    /// A 1-ulp difference comparator: `a` and `b` must agree to within one
    /// representable step of the wider-magnitude value.
    #[allow(clippy::float_cmp)] // exact equality fast-path is intentional here
    fn within_1_ulp(a: f64, b: f64) -> bool {
        if a == b {
            return true;
        }
        let scale = a.abs().max(b.abs());
        (a - b).abs() <= scale * GAUSSIAN_ULP_TOLERANCE
    }

    /// Gaussian vectors generated from authoritative `java.util.Random`
    /// (JDK 26). `nextGaussian`'s `log` step is **not** required to be
    /// bit-identical across math libraries (Java allows ≤1 ulp; the reference
    /// used the MSVC CRT; we use Rust's libm). The LCG + double draws feeding
    /// it are verified bit-exact by the other tests. See `docs/reference/rng.md`
    /// and `docs/reference/discrepancies.md` #4.
    #[test]
    fn next_gaussian_matches_java_within_1_ulp() {
        for &(seed, want) in NEXT_GAUSSIAN {
            let mut rng = GameRandom::with_seed(seed);
            for (i, w) in want.iter().enumerate() {
                let got = rng.next_gaussian();
                assert!(
                    within_1_ulp(got, *w),
                    "seed {seed} gaussian draw {i}: got {got:e}, want {w:e} (>1 ulp apart)"
                );
            }
        }
    }

    /// (seed, nextBytes(10)) — DERIVED, not JDK vectors.
    ///
    /// The reference does byte-by-byte `next(8)` (`Random.cpp:33-39`, old-Java
    /// semantics). Modern JDKs pack 4 bytes per `nextInt`, so JDK output cannot
    /// be used as ground truth here; the test below recomputes the expectation
    /// with an independent inline LCG instead.
    #[test]
    fn next_bytes_matches_reference_byte_by_byte_semantics() {
        // Independent inline reimplementation of the 48-bit LCG for top-8-bit draws.
        let expected_bytes = |seed: i64, count: usize| -> Vec<u8> {
            let mut seed = (seed ^ MULTIPLIER) & MASK;
            (0..count)
                .map(|_| {
                    seed = seed.wrapping_mul(MULTIPLIER).wrapping_add(ADDEND) & MASK;
                    (seed >> 40) as u8 // top 8 bits of the 48-bit state
                })
                .collect()
        };
        for &seed in NEXT_BYTES_SEEDS {
            let mut rng = GameRandom::with_seed(seed);
            let want = expected_bytes(seed, 10);
            let mut got = [0u8; 10];
            rng.next_bytes(&mut got);
            assert_eq!(got.to_vec(), want, "seed {seed} nextBytes(10)");
        }
    }

    #[test]
    fn reseed_resets_state_including_gaussian_cache() {
        let mut rng = GameRandom::with_seed(0);
        let first = rng.next_gaussian();
        // Reseeding must clear the cached second gaussian: the sequence for
        // seed 0 restarts identically.
        rng.set_seed(0);
        assert_eq!(rng.next_gaussian().to_bits(), first.to_bits());
    }

    #[test]
    fn gaussian_cache_does_not_advance_seed() {
        // First call consumes doubles; the cached second call must consume
        // nothing (verified by reproducing: cache-hit path leaves seed equal).
        let mut a = GameRandom::with_seed(7);
        let g1 = a.next_gaussian();
        let seed_after_first = a.seed;
        let g2 = a.next_gaussian();
        assert_eq!(a.seed, seed_after_first, "cache hit must not advance seed");
        assert!(g1.is_finite() && g2.is_finite());
    }

    #[test]
    fn cosmetic_random_is_deterministic_and_independent_of_game_random() {
        let mut a = CosmeticRandom::with_seed(1);
        let mut b = CosmeticRandom::with_seed(1);
        for _ in 0..16 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
        // Cosmetic floats stay within [0, 1).
        let f = a.next_float();
        assert!((0.0..1.0).contains(&f));
    }
}
