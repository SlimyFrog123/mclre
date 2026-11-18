# RNG & Math Reference (V-04)

> **Status:** Verified from source (2026-11-17). All claims below cite files in `reference/lce-141217/MinecraftConsoles-141217/`.
> **Rule (SPEC 3.6):** port exactly — same algorithm, same call order, same arithmetic. Unit-test against reference vectors.

## 1. `java.util.Random` port — `Minecraft.World/Random.{h,cpp}` (VERIFIED)

This is an exact, complete port of Java `Random` (4J comment at line 7 confirms intent: "jave now uses the system nanosecond counter..."). Constants and semantics:

- **State:** 48-bit `seed`, `haveNextNextGaussian`, `nextNextGaussian` (`Random.h:6-8`).
- **`setSeed(s)`:** `seed = (s ^ 0x5DEECE66D) & ((1<<48)-1)`; clears gaussian cache (`Random.cpp:21-25`).
- **`next(bits)`:** `seed = (seed * 0x5DEECE66D + 0xB) & ((1<<48)-1); return (int)(seed >> (48 - bits))` (`Random.cpp:27-31`).
- **`nextInt(bound)`** (`Random.cpp:76-91`):
  - Power-of-two fast path: `(int)(((int64)next(31) * n) >> 31)` — 4J comment: "Made __int64 instead of long" (same as modern Java; NOT the buggy old-JDK wraparound path — matches Java's current spec).
  - Rejection loop: `do { bits = next(31); val = bits % n; } while (bits - val + (n-1) < 0); return val;`
- **`nextLong()`:** `((int64)next(32) << 32) + next(32)` — note `+`, not `|` (Java semantic, sign-carrying).
- **`nextDouble()`:** `(((int64)next(26) << 27) + next(27)) / (double)(1<<53)`.
- **`nextFloat()`:** `next(24) / ((float)(1<<24))`.
- **`nextBoolean()`:** `next(1) != 0`.
- **`nextGaussian()`:** Box-Muller polar with cache, loop `while (s >= 1 || s == 0)`, `multiplier = sqrt(-2 * log(s)/s)` (`Random.cpp:48-69`).
- **`nextBytes(bytes, count)`:** successive `next(8)` per byte (`Random.cpp:33-39`).
- **Default ctor seed:** `QueryPerformanceCounter() + 8682522807148012` (platform-specific; only relevant for random-unseeded dev behavior, never for parity).

**Rust requirements for `GameRandom`:**
- `i64` wrapping arithmetic (`wrapping_mul`, `wrapping_add`), mask `(1u64<<48)-1`.
- `next_int(bound)` fast path must compute `(i64)next(31) * n >> 31` in 64-bit then cast to `i32`.
- Gaussian cache must be part of serialized state where the reference serializes it (Java doesn't serialize the gaussian flag; Random's state is just the seed — but call-order can leave a cached value; keep the cache in-memory and reproduce call order).
- Test vectors: derive from known Java outputs (e.g. `new Random(0).nextInt()`, seeded sequences of `nextInt(n)`, `nextLong`, `nextGaussian`) + reference-compiled vectors via devtools later.
- **`nextBytes` divergence note (verified 2026-11-17):** the reference fills bytes via one `next(8)` per byte (old-Java ≤1.4 semantics). Modern JDKs (≥1.6) pack 4 bytes per `nextInt`, so JDK output is NOT ground truth for `nextBytes`. Our port matches the reference (byte-by-byte); test vectors are computed from an independent inline LCG (`crates/core/src/random.rs`).
- **`nextGaussian` precision note (verified 2026-11-17):** the Box–Muller `log` step is not bit-identical across math libraries — Java allows ≤1 ulp, the reference used the MSVC CRT `log`, Rust uses `f64::ln`. The LCG and `next_double` draws feeding the gaussian ARE bit-exact (verified vs JDK 26, seeds 0/42/123456789/-1/-4936897262349239473). Tests tolerate ≤1 ulp. Baseline worldgen must not depend on gaussian output for parity-critical paths — confirm per-system at M6.

## 2. `Mth` — `Minecraft.World/Mth.{h,cpp}` (VERIFIED)

- **Sine table:** `float _sin[65536]`, built once: `_sin[i] = (float)sin(i * PI * 2 / 65536.0f)` (`Mth.cpp:17-24`).
- **`sin(x)`:** `_sin[(int)(x * sinScale) & 65535]` where `sinScale = 65536.0f / (float)(PI * 2)` (`Mth.cpp:26-30`, `Mth.h:20`).
- **`cos(x)`:** same table, index offset `+ 65536/4` (`Mth.cpp:32-36`).
- **Rust port:** 65536-entry `[f32; 65536]` LUT built with `(i as f64 * PI * 2.0 / 65536.0) as f32` — verify exact float conversion order (`f64` sin → cast `f32`). Index math: `(x * sinScale) as i32 & 0xFFFF`.
- Note: this table exists in *both* worldgen-irrelevant and gameplay paths; anything gameplay-affecting (entity motion, worldgen noise) must use this port, not `f32::sin`.

## 3. Noise classes present (for M6)

Files verified in `Minecraft.World/`: `ImprovedNoise.*`, `PerlinNoise.*`, `PerlinSimplexNoise.*`, `SimplexNoise.*`, `FastNoise.*`, `IntCache.*`, `BiomeSource.*`, `FixedBiomeSource.*`, `RandomLevelSource.*`, `HellRandomLevelSource.*` (Nether), `TheEndLevelRandomLevelSource.*` (End), `RandomScatteredLargeFeature.*`.

- `IntCache` = Java-style `IntCache` reuse pool (allocation only; port for determinism is unnecessary, but *call order* through it must be preserved).
- Detailed analysis (octaves, seeds per layer, `HellRandomLevelSource` Nether gen, biome layer stack) → `docs/reference/worldgen.md` at M6.

## 4. Seed derivation (to be completed at M6 kickoff)
- Text seed → numeric hash, numeric strings, negatives, 64-bit wrap: locate `Level`/`LevelData` seed parsing (next pass).
- Chunk/layer seed mixing: locate in `RandomLevelSource.cpp` / `BiomeSource.cpp` (next pass).
