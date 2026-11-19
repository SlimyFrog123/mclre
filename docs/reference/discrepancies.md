# Discrepancies Log

> Records every observed place where reference behavior disagrees with (a) this repo's assumptions, (b) wiki/community documentation, or (c) another reference snapshot. Precedence: reference source > owner captures > wiki > assumptions (SPEC 3.5).

| # | Date | Area | Discrepancy | Resolution / Owner note |
|---|---|---|---|---|
| 1 | 2026-11-17 | Versioning | Spec guessed baseline ≈ "TU19"; the source carries no TU string — it shows display version `"1.6"`, product build 560, and 1.8.2-era forward-ports. | Record profile as `lce:141217` (date-based) until owner confirms TU/CU number externally. See `baseline.md`. |
| 2 | 2026-11-17 | World sizes | Spec guessed Nether scale as uniform; source shows **per-preset** Nether scales (classic/small=3, medium=6, large=8) and a **fixed 18-chunk End** for all sizes. | Adopt source values as hard data in `WorldSizePreset`. |
| 3 | 2026-11-17 | Java `Random.nextInt` | Spec assumed classic Java semantics; source uses the modern power-of-two path (`(next(31) * n) >> 31` in 64-bit) — same as current Java spec, not the old buggy wraparound. | Port the modern path exactly; note for parity vectors. |
| 4 | 2026-11-17 | Java `Random.nextBytes` | Modern JDKs pack 4 bytes per `nextInt`; the reference (`Random.cpp:33-39`) does one `next(8)` per byte (old-Java ≤1.4 semantics). | Port the reference byte-by-byte behavior. JDK vectors unusable as ground truth; test computes expectations from an independent inline LCG. |
| 5 | 2026-11-17 | `nextGaussian` math library | Java `Math.log` allows ≤1 ulp; MSVC CRT `log` (reference) differs again; Rust `f64::ln` differs from both on some inputs. Verified: seed-42 first gaussian differs from JDK by exactly 1 ulp; all `next_double` inputs bit-identical. | Tolerance ≤1 ulp in tests; flagged for any future parity-critical gaussian consumer (baseline worldgen does not use gaussian — confirm at M6). |
| 6 | 2026-11-18 | Save format compression | LCE (Xbox 360/PS3) uses **LZX** compression (type 3, Xbox codec) in `.mca` files; Java Edition uses **zlib** (type 2, 4096-byte sectors, 1024-entry location/timestamp tables). Our Anvil implementation writes standard Java Edition format (zlib, type 2) so external tools (NbtExplorer, Amulet, MCA Selector) can inspect saves. | **Deviation logged, not fixed** — LCE saves are imported via a converter (`tools/assetc` → `game-assets/`), not read directly. The converter will handle LZX→zlib translation at the edge (ADR 0004 scope). This is a save-format boundary concern, not a gameplay parity issue. |

<!-- Template row:
| N | YYYY-MM-DD | Area | What differs from expectation | How resolved + evidence |
-->
