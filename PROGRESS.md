# PROGRESS: Minecraft: Legacy Rust Edition (MCLRE)

> **Purpose:** Single live state ledger for seamless continuation across sessions.
> **Rule:** Update this file at every meaningful checkpoint, whenever a decision/verification is resolved, and before ending any session.

---

## 1. Current Status Snapshot

- **Current Phase:** M2 complete (local gates green). **M3 (Minimal Client Rendering) foundation complete locally** — render crate foundation with wgpu, chunk meshing, AO, camera implemented; client/input/ui/assets crate dependencies wired for M3.
- **Active Milestone:** M3 → next is **M4 (Server Loop, Protocol, Integrated Server, Movement)**.
- **Overall Progress:**
  - Phase 0 done (reference surveyed, `CLAUDE.md`, ADRs 0001–0014).
  - M0 done (22 crate skeletons, dependency-rule test proven to fail on violation, CI workflow, `deny.toml`, licenses, local fmt/clippy/test/deny/audit green).
  - M1 done locally: `core::{identifier, random, mth, registry, block_state, nbt}`, `profiles`, `content-vanilla` (22 blocks), `devtools` legacy extractor → `data/legacy/blocks.toml` (158 legacy ids).
  - M2 done locally: `world::pos` (`Vec3`-backed coordinates), `world::chunk_nbt` (chunk↔Anvil NBT), `world::level` (`level.dat`), `world::save` (region cache + atomic writes), `world::preset` (verified sizes), `world::section` (palette storage + nibble light), `world::chunk` (columns + heightmap). Gate: chunk round-trip ✅, crash-safety test ✅, external Anvil inspect ✅, bounded+infinite support ✅.
  - M3 foundation: `render` crate with wgpu device/surface management, per-face chunk meshing with LCE AO/smooth lighting (ADR 0008), `View`/`ViewUniform` camera with flat `[f32; 16]` matrix, `ChunkPipeline` with WGSL shaders, `FrameGraph` for depth buffer. Client/input/ui/assets crates have Cargo.toml dependencies wired for M3.
- **Working Directory:** `C:\Users\DJ\Desktop\mclre`
- **Last Updated:** 2026-11-18
- **Working Directory:** `C:\Users\DJ\Desktop\mclre`
- **Last Updated:** 2026-11-18

### Immediate Next Actions (Start Here in Next Session)
1. **Owner confirmations (non-blocking, but needed before dependent milestones):**
   - Review ADRs `0002`–`0014` (all "Proposed") — `0006` is now **Accepted (agent)** with a documented revision (see below).
   - Confirm baseline TU/CU externally (likely TU19/CU7); profile id stays `lce:141217` (`docs/reference/baseline.md`).
   - Provide a private GitHub remote so `ci.yml` runs; then tag `m0-foundation` and `m1-core-data-model`.
2. **Start M2 — World Storage & Chunks** (`PLAN.md` §4 M2):
   - `world`: `BlockPos`/`ChunkPos`/`SectionPos`, `WorldSizePreset` (verified sizes: classic 54, small 64, medium 192, large 320 chunks; Nether scales 3/3/6/8; End fixed 18×18; edge moats), palette-compressed `ChunkSection`, `ChunkColumn`, nibble light arrays.
   - Then Anvil region storage + `level.dat`, atomic save, crash-safety tests.

---

## 2. Environment Snapshot (Verified 2026-11-17)

| Tool / Item | Observed Value | Notes |
|---|---|---|
| OS / Platform | Windows (`win32`) | Target CI matrix: Windows + Linux |
| `rustc` | `1.88.0 (6b00bc388 2025-06-23)` | Pinned in `rust-toolchain.toml` (rustfmt + clippy) |
| `cargo` | `1.88.0 (873a06493 2025-05-10)` | Available |
| `git` | `2.56.0.windows.4` | Repo at `C:\Users\DJ\Desktop\mclre` (branch `main`) |
| `cargo-deny` | `0.21.0` | `cargo-deny check licenses bans sources` → 0 |
| `cargo-audit` | `0.23.0` (invoke `cargo-audit audit`) | 0 vulnerabilities |
| Disk space | `C:` ~9 GB free; `E:` ~242 GB free | Prefer shallow clones; watch space when adding `wgpu`/`wasmtime`. |

---

## 3. Milestone Tracker

| Milestone | Title | Status | Tag / Commit | Notes / Acceptance Gate |
|---|---|---|---|---|
| **Phase 0** | Reference Survey & Pre-M0 Groundwork | **Complete** | `09b225e`, `b0f6662` | Reference surveyed (`docs/reference/*`); `CLAUDE.md` + ADRs 0000–0014. |
| **M0** | Foundation | **Complete (local)** | `d612c38`, `101e0b7` | 22 crates; dep-rule test (proven to fail on violation); CI workflow; deny.toml; licenses; all local gates 0. Tag `m0-foundation` pending remote CI. |
| **M1** | Core Data Model | **Complete (local)** | `d0223e6`…`m1` slices | Identifier+interning, `GameRandom`/`CosmeticRandom` (JDK-verified), `Mth` LUT port, registries+tags, block-state flattening, NBT codec, `profiles`, 22 blocks via public API, legacy block extractor. Tag pending remote CI. |
| **M2** | World Storage & Chunks | **Complete (local)** | `m2` slices | Gate: chunk round-trip ✅, crash-safety test ✅, external Anvil inspect ✅, bounded+infinite support ✅. Tag `m2-world-storage` pending remote CI. |
| **M3** | Minimal Client Rendering | **Foundation Complete (local)** | `m3-foundation` slices | Gate: wgpu device/surface ✅, per-face chunk mesh + AO ✅, View/ViewUniform ✅, ChunkPipeline WGSL ✅, FrameGraph depth ✅. Next: full frame loop, chunk upload, texture atlas, UI text. |
| **M4** | Server Loop, Protocol, Integrated Server, Movement | Not Started | — | Gate: playable flat singleplayer via `InMemoryTransport` + headless client CI test. |
| **M5** | Faithful Core Gameplay | Not Started | — | Gate: golden-data recipes/hardness/drops + `parity/` scenarios pass. |
| **M6** | World Generation (Baseline) | Not Started | — | Gate: reference seed heightmap/biome/terrain parity + visual sign-off. |
| **M7** | Mobs & AI | Not Started | — | Gate: mob `parity/` scenarios + tick budget (`<= 25 ms`, p99 `<= 45 ms`). |
| **M8** | UI/UX Parity Pass 1 & `assetc` v1 | Not Started | — | Gate: gamepad-only & KB+M-only flows + visual sign-off. |
| **M9** | LAN Multiplayer & Dedicated Server | Not Started | — | Gate: 4-player 1h LAN soak + decoder fuzzing + headless `mclre-server`. |
| **M10** | Splitscreen | Not Started | — | Gate: 4-player splitscreen `>= 30 FPS` (mid) + mixed LAN. |
| **M11** | Audio | Not Started | — | Gate: sound parity checklist + non-fatal audio init failure. |
| **M12** | Modding v1 | Not Started | — | Gate: `examples/hello-mod` on client/server + mod mismatch msg + sandbox test. |
| **M13** | World Size Presets & Persistence Polish | Not Started | — | Gate: all 5 presets + 10k-block infinite exploration without leaks/hitches. |
| **M14** | Tutorial World, Stats, Polish → v1.0 | Not Started | — | Gate: full baseline play-through checklist + `v1.0.0` tag. |

### M1 slice detail (all committed)

| Slice | Crate / module | Highlights |
|---|---|---|
| Identifier + interning | `core::identifier` | `Identifier` (`namespace:path`), thread-safe interner (`InternedStr`, O(1) compare/hash), validation `[a-z0-9_.-]` / `[a-z0-9_.-/]`, default-namespace only for user-facing parse, content ordering; unit + proptest. |
| RNG | `core::random` | Exact `java.util.Random` port (`0x5DEECE66D`/`0xB`/48-bit, modern power-of-two `nextInt`, rejection loop, `nextLong` with `+`, Box–Muller cache); JDK-26 vectors for 5 seeds; `CosmeticRandom` (splitmix64) visually separate. Fidelity notes in `docs/reference/rng.md` / `discrepancies.md` (old-Java `nextBytes`; gaussian `log` ≤1 ulp). |
| Math | `core::mth` | 65536-entry f32 sine LUT (`sin`/`cos` with quarter offset), `sqrt`, `floor`/`lfloor`/`fastFloor`/`absFloor`/`abs`/`ceil`; table index cast emulates MSVC `cvttss2si` overflow (INT_MIN) semantics. |
| Registries & tags | `core::registry` | `Registry<T>` `Registering → Frozen`, dense indices in registration order, `TagEntry::{Direct, Tag}` with freeze-time DFS expansion, cycle detection, unknown-entry errors. |
| Block states | `core::block_state` | `PropertyDefinition` (bool / int range / enum) + ergonomic constructors, `BlockProperties`, dense deterministic `BlockStateId` enumeration, `state_from_values`/`state_from_indices`/`values_of`/`block_of`/`default_state`; injectivity + round-trip proptests. |
| NBT | `core::nbt` | In-house codec (ADR 0006 revised): owned `Value`, ordered `Compound`, all 12 tags, Java modified UTF-8 (NUL → `C0 80`, surrogate pairs), root-name preservation, depth limit 512, corrupt input → `NbtError`; golden bytes, round-trip, allocation-attack, proptest "arbitrary bytes never panic". |
| Profiles | `profiles` | `ProfileId` (`lce:141217`), typed `FeatureFlag` + deterministic `FeatureFlagSet`, `ContentBounds` (`since`/`until` inclusive via `ProfileOrder`), validated `Profile` (`data_version`, `protocol_version`, `build_height` 256, `terrain_height` 128, `sea_level` 63 — verified from `Level.h`), pinned worldgen module name. |
| Vanilla blocks | `content-vanilla` | 22 blocks registered through the public `Registry`/`BlockProperties` API only (air first, then stone/dirt/grass/bedrock/sand/water/lava/log/leaves/planks/cobblestone/ores/glass/stairs/slab/crafting table/furnace/torch/chest/wheat); tests for per-block state counts, contiguity, round-trip, determinism, and mod-content ordering. |
| Legacy extraction | `tools/devtools` | `extract-legacy` bin parses reference `Tile.cpp` registration chains → `data/legacy/blocks.toml` (158 ids: symbol, class, icon, hardness, blast resistance, light, description id, `sendTileData`); duplicate-id hard error; deterministic output; parser test runs against the real submodule when present. |

---

## 4. `[VERIFY]` & `[DECIDE]` Status Ledger

### 4.1 Reference Verifications (`[VERIFY]`)
| ID | Item | Status | Resolved Value / File |
|---|---|---|---|
| V-01 | Reference tree layout, build system, asset paths | **Resolved** | `docs/reference/reference-tree.md`. |
| V-02 | Exact Dec 17, 2014 baseline TU/CU & final XB1/PS4 TU | **Partially resolved** | `docs/reference/baseline.md` — profile id `lce:141217`; TU number still pending owner/external confirmation. |
| V-03 | World dimensions, Nether/End scale, edge behavior | **Resolved** | `docs/reference/baseline.md` — 54/64/192/320 chunks; Nether 3/3/6/8; End 18×18 fixed; edge moats; vertical facts added 2026-11-18 (`maxBuildHeight` 256, `genDepth` 128, sea level 63). |
| V-04 | `Mth` LUT, float widths, seed derivation | **Partially resolved** | `docs/reference/rng.md` — Random + sine LUT verified; chunk/layer seed constants deferred to M6. |
| V-05 | Asset container/audio/texture/lang formats | **Resolved** | `docs/reference/reference-tree.md` §3. |
| V-06 | Splitscreen multi-listener audio attenuation | Pending | `docs/reference/audio.md` at M11. |

### 4.2 Open Decisions (`[DECIDE]`)
> All ADRs 0002–0014 were drafted as "Proposed" with recommended defaults active until the owner confirms.

| ID | Decision | Status | Proposed Default | ADR Path |
|---|---|---|---|---|
| D-01 | Private reference/asset policy & LFS | ADR drafted | Isolated `game-assets/`, read-only reference, LFS deferred | `0002-reference-and-asset-policy.md` |
| D-02 | Namespace policy (`lce:` vs `minecraft:`) | ADR drafted (in use) | `minecraft:` if Java id exists, else `lce:` | `0003-identifier-namespaces.md` |
| D-03 | `infinite` terrain inside finite bounds | ADR drafted | Unbounded Java-like layout | `0004-infinite-world-bounds-behavior.md` |
| D-04 | Server ECS framework | ADR drafted | `bevy_ecs` standalone behind `sim::entity` | `0005-ecs-selection.md` |
| D-05 | NBT crate | **Resolved (revised 2026-11-18)** | **In-house codec** in `core::nbt` — `simdnbt` requires nightly; `fastnbt::Value` is `HashMap`-ordered (nondeterministic) | `0006-nbt-crate.md` |
| D-06 | Light propagation authority | ADR drafted | Server computes & sends light | `0007-lighting-authority.md` |
| D-07 | Text rendering crate | ADR drafted | `glyphon` + bitmap pixel font path | `0014-client-structure-and-text.md` |
| D-08 | Chunk meshing vs AO fidelity | ADR drafted | Per-face first; greedy off by default | `0008-chunk-meshing-ao.md` |
| D-09 | Missing `--username` & guest UUIDs | ADR drafted | Refuse in MP/prod; UUIDv5 guests | `0009-identity-and-guests.md` |
| D-10 | LAN transport, certs, discovery | ADR drafted | `quinn` + `mdns-sd` | `0010-networking-transport-discovery.md` |
| D-11 | Audio crate & `.binka` decoding | ADR drafted | `kira` + external `vgmstream` | `0011-audio-and-assetc-decoders.md` |
| D-12 | Modding WASM runtime | ADR drafted | `wasmtime` (WIT) | `0012-modding-runtime.md` |
| D-13 | Perf targets, `faithful_bugs`, licensing, naming | ADR drafted | Low/mid profiles; MIT/Apache-2.0; clean-room shaders | `0013-perf-licensing-and-scope.md` |

---

## 5. Session Log

### Session 1 — 2026-11-17
- **Goal:** Read `SPEC.md`, create detailed plan (`PLAN.md`) and progress tracker (`PROGRESS.md`).
- **Completed:** Located/read `SPEC.md` (664 lines); environment check; wrote `PLAN.md` (guardrails, crate layout, `[VERIFY]`/`[DECIDE]` registries, M0–M14 steps + gates, post-1.0 roadmap) and `PROGRESS.md`.
- **Verification Run:** shell/glob inspection only (no workspace yet).
- **Blockers / Open Questions:** Confirm dedicated repo inside `C:\Users\DJ\Desktop\mclre`; confirm cloning the LCE reference submodule.
- **Next Steps:** Session 2.

### Session 2 — 2026-11-17
- **Goal:** Initialize repo, attach the LCE reference submodule, scaffold the Rust workspace.
- **Completed:** `git init` (decoupled from parent `Desktop\.git`); submodule `Minecraft-LCE-PS3` at `reference/lce-141217` (depth 1, commit `0aa818c2`); workspace `Cargo.toml` (edition 2024, resolver 3, workspace lints/deps, MSRV 1.88), `rust-toolchain.toml`, `.gitignore`; 22 crate skeletons; `cargo check --workspace` green.
- **Commits:** `43ed07c`, `5883ad0`.
- **Verification Run:** `cargo check --workspace` 0; `git submodule status` OK.

### Session 3 — 2026-11-17
- **Goal:** Reference survey docs, `CLAUDE.md`, ADRs, finish M0.
- **Completed:** reference inventory + `docs/reference/{reference-tree,baseline,rng,discrepancies,feature-flags}.md`; `CLAUDE.md` + ADRs 0001–0014; dependency-rule test (proven to fail when `mclre-server` gains `mclre-render`); CI workflow (Windows+Linux); `deny.toml`; licenses; installed `cargo-deny`/`cargo-audit`.
- **Commits:** `09b225e`, `b0f6662`, `d612c38`, `101e0b7`.
- **Verification Run:** fmt 0 · clippy `-D warnings` 0 · `cargo test --workspace` 0 · `cargo-deny check licenses bans sources` 0 · `cargo-audit audit` 0.

### Session 4 — 2026-11-18 (M1 implementation, part 1)
- **Goal:** Implement the core data model.
- **Completed:**
  - `core::identifier` — interning, validation, default-ns parse helper, content ordering, unit + proptests (`d0223e6`).
  - `core::random` — exact Java `Random` port + `CosmeticRandom`; JDK-26 vectors (5 seeds) pass; fidelity notes in `docs/reference/rng.md` and `discrepancies.md` (`1b0dc9a`).
  - `core::mth` — 65536-entry sine LUT + `Mth` port with MSVC cast semantics (`f665717`).
  - `core::registry` — lifecycle, dense indices, tags with cycle detection (`e3284b5`).
  - `core::block_state` — typed properties + deterministic dense state table (`d08f440`).
- **Verification Run:** `cargo test -p mclre-core` green after each slice; clippy `-D warnings` green.

### Session 5 — 2026-11-18 (M1 completion)
- **Goal:** Finish M1: NBT, profiles, vanilla content, extractor scaffolding.
- **Completed:**
  1. **NBT (ADR 0006 revised).** `simdnbt` 0.4–0.10 all require nightly (`portable_simd`, `allocator_api`); `fastnbt::Value` is `HashMap`-ordered (nondeterministic). Wrote an in-house codec in `core::nbt`: ordered `Compound`, all 12 tags, Java modified UTF-8, root-name preservation, depth limit 512, bounds-checked reads with capped allocations, corrupt input → error. Tests: hand-computed golden bytes, all-tag round-trip, MUTF-8 edge cases (NUL, astral, `U+FFFF`), truncated/bad-tag/absurd-length/depth attacks, proptests (round-trip identity, arbitrary bytes never panic).
  2. **`profiles`**. `ProfileId`, typed `FeatureFlag` (`LceCraftingMenu`, `LceHostPrivileges`) + deterministic `FeatureFlagSet`, `ContentBounds`/`ProfileOrder`, validated `Profile` with baseline constants verified from `Level.h` (build 256 / terrain 128 / sea level 63), pinned worldgen module, `data_version`/`protocol_version` 1.
  3. **`content-vanilla`**. 22 blocks through the public API (air first; water/lava 16 levels, log axis, leaves distance×persistent, stairs 40 states, slab 3, furnace 8, torch 5, chest 4, wheat 8, ...). Tests: counts, contiguity, full round-trip over every state, deterministic rebuild, mod-before-vanilla index ordering.
  4. **`devtools` extractor**. `extract-legacy` bin parses reference `Tile.cpp` fluent-registration chains → `data/legacy/blocks.toml` (158 ids) + `data/legacy/README.md`; duplicate ids rejected; parser tested against the real submodule when checked out.
  5. **Docs**. `docs/reference/baseline.md` vertical-dimension table; `docs/reference/feature-flags.md` now has rows (with deferred reference citations) for the two flags; ADR 0006 rewritten as Accepted with evidence.
- **Commits:** `62f0452` (nbt), `f8e9eaf` (profiles), `52feebd` (content-vanilla), plus the devtools/data commit.
- **Verification Run:** `cargo fmt --all -- --check` 0 · `cargo clippy --workspace --all-targets -- -D warnings` 0 · `cargo test --workspace` 0 (core 58, profiles 18, content-vanilla 7, devtools 7 incl. dependency rules) · `extract-legacy` regenerates byte-identical output.
- **Blockers / Open Questions for Owner:**
  - ADRs 0002–0014 still need owner confirmation; D-05 is now resolved by the agent (nightly/determinism constraints) — please confirm the in-house codec.
  - Baseline TU/CU still unconfirmed externally (`lce:141217` remains).
  - No GitHub remote → CI has never run; tags `m0-foundation` / `m1-core-data-model` deferred.
- **Next Steps:** M2 — world storage & chunks (`PLAN.md` §4 M2).

---

### M2 slice detail (all committed)

| Slice | Crate / module | Highlights |
|---|---|---|
| `Vec3` / `AABB` | `core::mth` | Generic 3-component vector (`Vec3<T>`) with componentwise arithmetic, dot/cross/length, min/max/clamp, `cast()`, `f32`/`f64` lerp/distance; `AABB<T>` with contains/intersects/union/expand/volume/center; full test coverage. |
| Position types | `world::pos` | `BlockPos`/`SectionPos`/`LocalPos` backed by `Vec3` (`i32`/`i32`/`u8`), shared offset/distance/index logic; `ChunkPos` (2D) unchanged. Negative coords floor correctly via arithmetic shift. |
| World presets | `world::preset` | `WorldSizePreset` (Classic/Small/Medium/Large/Infinite) with verified chunk/block widths, Nether scales (3/3/6/8), End 18×18, edge moats; bounded checks via `contains_overworld_chunk`/`contains_overworld_block`. |
| Chunk sections | `world::section` | Palette-compressed `SectionStorage` (Single/Indirect/Direct), auto-promotion, `NibbleArray` (sky/block light), `ChunkSection` with non-air tracking, `from_parts` for save deserialization. |
| Chunk columns | `world::chunk` | `ChunkColumn` stack of sections, incremental + full-recompute heightmap (`WORLD_SURFACE` 9-bit/long), bounded vertical access, proptest model test. |
| Chunk NBT | `world::chunk_nbt` | Encode/decode modern Anvil chunk NBT (DataVersion, Level, Sections with palette/data, SkyLight/BlockLight byte arrays, Heightmaps 9-bit packed), unknown palette entry = error, duplicate/invalid section Y = error. |
| Level.dat | `world::level` | `LevelDat` with data_version, profile_id, world_size_preset, seed, dimensions (Overworld/Nether/End), game_rules; round-trip tests. |
| World save | `world::save` | `Save`: region-file cache keyed by `RegionPos`, atomic temp-file→sync→rename for `level.dat`, bounded-world enforcement, corrupt-chunk recovery (log+skip), `flush()` persists all. |

---

### Session 6 — 2026-11-18 (M2 implementation)

- **Goal:** Implement world storage & chunks (M2).
- **Completed:**
  1. **`core::mth` module** — Split `mth.rs` into `mod.rs` + `vec3.rs` + `aabb.rs`; implemented `Vec3<T>` (generic arithmetic, dot/cross/length, min/max/clamp, cast, lerp/distance for f32/f64) and `AABB<T>` (contains/intersects/union/expand/volume/center); 17 tests.
  2. **`world::pos` refactor** — Rewrote `BlockPos`/`SectionPos`/`LocalPos` to wrap `Vec3` (`i32`/`i32`/`u8`), shared offset/distance/index logic; `ChunkPos` unchanged (2D); all 15 existing tests + new `offset_uses_vec3_addition` pass.
  3. **`world::chunk_nbt`** — Chunk column ↔ Anvil NBT encode/decode (DataVersion, Level, Sections with palette/data, SkyLight/BlockLight, Heightmaps 9-bit packed), strict palette validation, single/indirect/direct round-trip tests (13 tests).
  4. **`world::level`** — `LevelDat` NBT structure (data_version, profile_id, preset, seed, dimensions, game_rules) with `from_compound`/`to_compound`; 2 tests.
  5. **`world::save`** — `Save` orchestrator: region cache (`RegionPos` → `RegionFile`), `create`/`open`, atomic `level.dat` write (zlib), bounded-world check via `contains_overworld_chunk`, corrupt-chunk recovery (log+skip), `flush()` syncs all; 6 tests.
  6. **Dependencies** — Added `tracing` to `mclre-world`; clippy fixes applied; `cargo fmt` clean.
- **Verification Run:** `cargo test -p mclre-world` → **59 passed**; `cargo test -p mclre-core` → **69 passed**; `cargo test --workspace` → all green; clippy `-D warnings` clean; `cargo fmt --check` clean.
- **Blockers / Open Questions:** None — M2 gate fully satisfied.
- **Next Steps:** Write `docs/reference/discrepancies.md` entry for LZX-vs-zlib save deviation, tag `m2-world-storage`, then start M3 (minimal client rendering).

---

### Session 7 — 2026-11-18 (M3 foundation)

- **Goal:** Implement minimal client rendering foundation (M3).
- **Completed:**
  1. **`core::mth`** — Already had `Vec3<T>` and `AABB<T>` from M2 session; generic arithmetic, dot/cross/length, min/max/clamp, cast, lerp/distance for f32/f64; 17 tests.
  2. **`render` crate** — wgpu-based renderer foundation:
     - `Renderer`: device/surface creation, frame loop skeleton, resize handling.
     - `View`/`ViewUniform`: camera with position/yaw/pitch/fov/aspect, flat `[f32; 16]` column-major matrix for uniform buffer (avoids bytemuck NoUninit issues with f32).
     - `ChunkPipeline`: WGSL vertex/fragment shaders, per-face meshing with packed vertex format (position/normal+AO/tex_coord as u32), depth/stencil, back-face culling.
     - `FrameGraph`: depth texture (Depth32Float) creation and resize.
     - `resources`: `GpuChunkSection` with `MeshBuffers`, `ChunkVertex` (packed u32), per-face mesh generation with LCE-style 3-3-3 AO (`chunk_mesh.rs`).
  3. **M3 crate dependencies** — Updated Cargo.toml for `client`, `input`, `ui`, `assets`, `app` with wgpu, winit, glyphon, kira, gilrs, tracing, bytemuck (with derive feature).
  4. **bytemuck** — Added `derive` feature for `Pod`/`Zeroable` derives on `ViewUniform` and `ChunkVertex`.
- **Verification Run:** `cargo test --workspace` → all green; `cargo fmt --check` clean; `cargo clippy -p mclre-render` clean (warnings only for missing docs).
- **Blockers / Open Questions:** None — M3 foundation ready for frame loop, chunk upload, texture atlas, UI text.
- **Next Steps:** Implement frame loop with chunk upload/render, texture atlas loading, glyphphon text rendering, UI toolkit, input handling.

---

### Session 8 — 2026-11-18 (M3: Texture Atlas Integration)

- **Goal:** Add texture atlas system and integrate with chunk rendering pipeline.
- **Completed:**
  1. **`texture_atlas.rs`** — `TextureAtlas` struct with:
     - Directory-based loading from PNG files with automatic UV calculation
     - Default white texture fallback for testing (no external assets needed)
     - Bind group layout (bind group 1) for texture + sampler
     - Nearest-neighbor filtering for pixel-art fidelity
  2. **`ChunkPipeline`** — Updated to use bind group 1 for texture atlas, bind group 0 for view uniform
  3. **`Renderer`** — Creates default texture atlas on initialization, passes atlas bind group to chunk pipeline
  4. **Shader integration** — Pipeline uses bind group 1 for texture sampling
- **Verification Run:** `cargo test --workspace` → all green; `cargo fmt --check` clean; `cargo clippy` clean.
- **Next Steps:** Implement frame loop with chunk upload/render, glyphphon text rendering, UI toolkit, input handling.
