# Baseline Reference — Version & World Facts

> **Status:** Verified from source where noted; a few external confirmations still pending (marked below).
> **Reference:** `reference/lce-141217/MinecraftConsoles-141217/` (see `reference-tree.md`).

## 1. Version identity (V-02)

| Fact | Value | Evidence |
|---|---|---|
| Build date | **December 17, 2014** (repo dir `MinecraftConsoles-141217`) | repo structure [external confirmation of exact TU below still pending] |
| Display string | `"Minecraft Xbox " + "1.6"` | `Minecraft.Client/ClientConstants.cpp:4`, `Common/BuildVer.h:15,17` (`VER_FILEVERSION_STRING "1.6"`) |
| Product build | `VER_PRODUCTBUILD = 560`, `VER_NETWORK = 560`, QFE 0 | `Common/BuildVer.h:10-13` |
| PC parity era | **1.8.2-era code** (many "brought forward from 1.8.2" changes: `AnimatePacket.h:13` EAT=5, `Chicken.cpp:33` hitbox 0.4→0.7 in 1.8.2, `GameRenderer.cpp` lighting-layer changes, `Attribute.h:16` "1.8+") | many 4J comments |
| Console TU/CU | **Likely TU19 (Xbox 360/PS3) / CU7 (Xbox One) — released Dec 17, 2014.** Source itself doesn't carry a "TU19" string; confirm against community sources before relying on the number. | PENDING EXTERNAL CONFIRMATION |

Decision for now: the profile is **`lce:141217`** (date-based, unambiguous). If the owner wants `lce:tu19` naming after confirming, rename is trivial (data, not code).

## 2. World sizes & edge behavior (V-03) — VERIFIED

Evidence: `Minecraft.World/ChunkSource.h:8-49`, `Minecraft.Client/LevelRenderer.cpp:92-96`.

| Preset | Chunk width | Blocks | Notes |
|---|---|---|---|
| `classic` | 54 | **864** | `LEVEL_WIDTH_CLASSIC 54` (PS3/360 default) |
| `small` | 64 | **1024** | `LEVEL_WIDTH_SMALL 64` ("Our default map (at zoom level 3) is 1024x1024") |
| `medium` | 192 | **3072** | `LEVEL_WIDTH_MEDIUM (3*64)` |
| `large` | 320 | **5120** | `LEVEL_WIDTH_LARGE (5*64)`; `LEVEL_MAX_WIDTH (5*64)` |
| End | 18 | **288** | `END_LEVEL_MAX_WIDTH 18` — fixed for ALL sizes ("Fix the size of the end for all platforms") |

**Nether coordinate scale** (`HELL_LEVEL_SCALE_*`, per preset — 1 block in Nether = scale blocks in Overworld):

| Preset | Nether scale | Nether chunk width |
|---|---|---|
| `classic` | 3 | 18 |
| `small` | 3 | 21* |
| `medium` | 6 | 32 |
| `large` | 8 | 40 |

\* `HELL_LEVEL_MAX_WIDTH = LEVEL_MAX_WIDTH / HELL_LEVEL_MAX_SCALE` where MAX scale is 8 for large worlds; the per-preset scale selects which the save uses. Exact Nether width per preset needs a follow-up pass through `LevelRenderer`/`ChunkSource` construction — the scales above are verified from `ChunkSource.h:30-33`.

**Edge behavior:** finite worlds have **edge moats** — `ChunkSource.h:57-59` declares `m_classicEdgeMoat`, `m_smallEdgeMoat`, `m_mediumEdgeMoat` (large worlds presumably unmoated/ocean edge). Confirms the "moat" treatment at world boundaries. Exact edge visuals (barrier vs ocean) to be pinned down when porting chunk-edge rendering (M6/M13).

**Vertical dimensions (VERIFIED, added 2026-11-18):**

| Fact | Value | Evidence |
|---|---|---|
| Minimum build height | **0** | `Level.h:69` `minBuildHeight = 0` |
| Maximum build height | **256** (valid Y `0..=255`) | `Level.h:70` `maxBuildHeight = 256` ("brought forward from 1.2.3") |
| Terrain generation depth | **128** | `Level.h:71-73` `genDepthBits = 7`, `genDepth = 1 << 7` |
| Sea level | **63** | `Level.h:74` `constSeaLevel = genDepth / 2 - 1` |
| Chunk storage split | lower half compressed, upper half sparse | `LevelChunk.h:50` `upperBlocks; // 128 - 255` |

So the baseline profile exposes **256 blocks of build height** while worldgen only fills the **bottom 128**; the upper half is (normally) air and stored sparsely. This is encoded in `crates/profiles` (`build_height`, `terrain_height`, `sea_level`).


**Dimension chunk-pool layout:** `LevelRenderer.cpp:92-96` computes per-dimension render grid sizes and `DIMENSION_OFFSETS` — confirms all three dimensions live in one bounded coordinate system sized by the preset.

## 3. RNG & math (V-04) — VERIFIED

See `rng.md` for full details. Summary: `Random.cpp` is a line-by-line `java.util.Random` port; `Mth` uses a 65536-entry float sine table.

## 4. Implications for our implementation
- `WorldSizePreset` values are now **hard data** (write them into `profiles` + `world`, cross-check golden tests with these constants).
- The End is fixed at 18×18 chunks for every preset — encode in `WorldSizePreset::end_chunks`.
- Edge moats are a real feature; implement per-preset `edge_moat: bool` (+ visuals) from M6 onward.
- Profile ID: `lce:141217` until owner confirms TU numbering.
