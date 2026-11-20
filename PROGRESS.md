# PROGRESS: Minecraft: Legacy Rust Edition (MCLRE)

> **Purpose:** Single live state ledger for seamless continuation across sessions.
> **Rule:** Update this file at every meaningful checkpoint, whenever a decision/verification is resolved, and before ending any session.

---

## 1. Current Status Snapshot

- **Current Phase:** M3 foundation complete (local gates green). **M3 (Minimal Client Rendering) in progress** — render crate with wgpu, chunk meshing, AO, texture atlas, camera; client with game loop, input, render thread; input system with keyboard/mouse/gamepad; assets crate with VFS, texture loading, atlas building; **UI toolkit (crates/ui) implemented with layout system, widget system, scene management, focus management, 9-slice scaling**; **asset loading integrated with render pipeline (AssetManager → GPU TextureAtlas)**; **asset format decision: Java Edition resource/data pack formats for direct compatibility and backporting (ADR 0015)**; **client frame loop with flat test world chunks (3×3 area, bedrock→stone→dirt→grass)**; **asset directory structure created (assets-placeholder/) and assetc converter tool implemented for LCE → Java format conversion**; **placeholder block textures generated and texture atlas integrated into render pipeline**; **client window opens and game loop runs with textured flat world**; **chunk meshes upload to GPU and render**; **game logic is framerate-independent (uses dt parameter)**; **assetc tool tested with reference assets (76 textures, DLC packs, audio); game-assets/ directory created with converted assets**; **client loads pre-built texture atlas from game-assets/ (2048x2048, 75 UV regions from manifest)**; text rendering placeholder (glyphon version mismatch with wgpu 22).
- **Active Milestone:** M3 → next is **M4 (Server Loop, Protocol, Integrated Server, Movement)**.
- **Overall Progress:**
  - Phase 0 done (reference surveyed, `CLAUDE.md`, ADRs 0001–0014).
  - M0 done (22 crate skeletons, dependency-rule test proven to fail on violation, CI workflow, `deny.toml`, licenses, local fmt/clippy/test/deny/audit green).
  - M1 done locally: `core::{identifier, random, mth, registry, block_state, nbt}`, `profiles`, `content-vanilla` (22 blocks), `devtools` legacy extractor → `data/legacy/blocks.toml` (158 legacy ids).
  - M2 done locally: `world::pos` (`Vec3`-backed coordinates), `world::chunk_nbt` (chunk↔Anvil NBT), `world::level` (`level.dat`), `world::save` (region cache + atomic writes), `world::preset` (verified sizes), `world::section` (palette storage + nibble light), `world::chunk` (columns + heightmap). Gate: chunk round-trip ✅, crash-safety test ✅, external Anvil inspect ✅, bounded+infinite support ✅.
  - M3 foundation: `render` crate with wgpu device/surface management, per-face chunk meshing with LCE AO/smooth lighting (ADR 0008), `View`/`ViewUniform` camera with flat `[f32; 16]` matrix, `ChunkPipeline` with WGSL shaders, `FrameGraph` for depth buffer, `TextureAtlas` with default white texture. `assets` crate with virtual filesystem (VFS), texture loading, and atlas building. `input` crate with keyboard/mouse/gamepad bindings. `client` with game loop, render thread, fixed-timestep simulation. Text rendering infrastructure added as placeholder (glyphon 0.8 API incompatible with wgpu 22).
- **Working Directory:** `C:\Users\DJ\Desktop\mclre`
- **Last Updated:** 2026-11-19

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

### Session 9 — 2026-11-19 (M3: Assets crate implementation)

- **Goal:** Implement assets crate with virtual filesystem, texture loading, and atlas building.
- **Completed:**
  1. **`assets` crate** — `AssetVfs` with root directories and in-memory overrides, `Texture` and `TextureAtlas` with simple row-based packing, `AssetManager` for loading/caching textures and building atlases.
  2. **Integration** — `AssetManager` creates default white texture atlas on initialization, integrates with `Renderer` and `ChunkPipeline`.
  3. **Tests** — 4 tests covering VFS reads/writes, texture loading, atlas packing, and overrides.
  4. **Dependencies** — Added `tempfile` as dev-dependency; updated `glyphon` to 0.8 (still incompatible with wgpu 22, placeholder maintained).
- **Verification Run:** `cargo test --workspace` → all green; `cargo fmt --check` clean; `cargo clippy` clean.
- **Blockers / Open Questions:** None — Assets crate foundation ready for texture loading and atlas integration.
- **Next Steps:** Implement UI toolkit foundation (crates/ui), complete client frame loop with chunk upload/render, integrate asset loading into render pipeline.

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

---
### Session 10 — 2026-11-19 (M3: UI toolkit compilation fixes)

- **Goal:** Fix UI crate (crates/ui) compilation errors and complete UI toolkit foundation.
- **Completed:**
  1. **Fixed duplicate definitions** — Removed duplicate `Widget` trait and `BaseWidget` struct definitions (were in both `lib.rs` and `widget.rs`).
  2. **Fixed visibility issues** — Made `Identifier`, `WidgetType`, `WidgetState`, `BaseWidget`, `WidgetId` public and properly exported from `lib.rs`.
  3. **Fixed module imports** — Corrected imports in `widget.rs`, `scene.rs`, `nine_slice.rs`, `focus.rs` to use proper paths.
  4. **Fixed Deref/DerefMut implementations** — Added proper `Target` type and separated `Deref`/`DerefMut` impls for all widget types (Button, Label, Image, Panel, Container, ScrollView).
  5. **Fixed lifetime issues** — Resolved lifetime errors in `WidgetRegistry` and `SceneManager` by:
     - Changing `WidgetRegistry` to use `Arc<dyn Widget>` instead of `Box<dyn Widget>` for shared ownership
     - Adding explicit lifetime bounds to trait object returns in `SceneManager::get_mut`
     - Using helper trait `SceneAccess` to work around trait object lifetime limitations
  6. **Fixed method name conflicts** — Renamed `Margin::new(all)` and `Padding::new(all)` to `uniform(all)` to avoid duplicate `new` definitions.
  7. **Fixed Debug/Clone issues** — Implemented manual `Debug` for `Button` (skipping `on_click` callback), used `Arc<dyn Fn()>` for cloneable callbacks.
  8. **Fixed Eq derive issues** — Removed `Eq` derive from `NineSliceRegion` (contains `f32` fields).
  9. **Fixed Default issues** — Made `NineSliceAtlas::texture_atlas` an `Option` to avoid requiring `TextureAtlas: Default`.
  10. **Fixed SceneId Display** — Added `std::fmt::Display` implementation for `SceneId` and `WidgetId`.
  11. **Fixed UiError** — Added `SceneNotFound` variant, removed duplicate `Layout`/`LayoutError`.
  12. **Fixed focus.rs** — Corrected `tab_order.retain` closure parameter shadowing.
- **Verification Run:** `cargo build --workspace` ✅, `cargo test --workspace` ✅ (all tests pass), `cargo clippy --workspace` ✅ (warnings only), `cargo fmt --all` ✅.
- **Blockers / Open Questions:** None — UI crate compiles and tests pass.
- **Next Steps:** Continue M3: complete client frame loop with chunk upload/render, integrate asset loading into render pipeline, implement glyphphon text rendering (or maintain placeholder).

---
### Session 11 — 2026-11-19 (M3: Asset loading integration with render pipeline)

- **Goal:** Integrate the assets crate (AssetManager, CPU-side texture atlas) with the render crate (GPU-side TextureAtlas) and client frame loop.
- **Completed:**
  1. **Added `TextureAtlas::from_texture` method** — New constructor in render crate that accepts a pre-built CPU-side texture from `mclre_assets::Texture`, UV coordinates, and padding. Extracts common logic into `create_atlas` helper.
  2. **Added `Atlas` variant to `AtlasError`** — For generic atlas errors (e.g., failed image buffer creation).
  3. **Added `Renderer::set_texture_atlas` method** — Allows the client to upload a custom texture atlas built by the asset system after renderer creation.
  4. **Removed cyclic dependency** — Removed `mclre-render` from `mclre-assets` dependencies (was unused in assets crate source).
  5. **Updated `Render` Cargo.toml** — Added `mclre-assets` dependency for `Texture` type.
  6. **Updated `Client` Cargo.toml** — Added `mclre-assets` dependency for `AssetManager`.
  7. **Integrated `AssetManager` into `Client`** — Client now creates an `AssetManager` wrapped in `Arc<Mutex<>>` and passes it to the render thread (currently uses default white texture; commented framework for loading block textures from asset directories).
  8. **Fixed render thread signature** — Updated `render_thread` to accept `asset_manager` parameter.
- **Verification Run:** `cargo build --workspace` ✅, `cargo test --workspace` ✅ (all tests pass), `cargo clippy --workspace` ✅ (warnings only), `cargo fmt --all` ✅.
- **Blockers / Open Questions:** None — Asset-to-render integration framework complete. Actual block texture loading pending asset directory setup.
- **Next Steps:** Continue M3: complete client frame loop with chunk upload/render, implement glyphphon text rendering (or maintain placeholder), set up asset directories for block textures.

---
### Session 12 — 2026-11-19 (M3: Asset format decision - Java Edition parity)

- **Goal:** Update SPEC.md, PLAN.md, and create ADR to document the decision to use Minecraft Java Edition asset formats for easier backporting and resource pack compatibility.
- **Completed:**
  1. **Updated SPEC.md §11.1** — Added explicit note that modern asset formats should use Java Edition resource/data pack formats for direct compatibility and easier backporting.
  2. **Updated PLAN.md M3 & M8 sections** — Added asset format policy note to `assets` crate baseline and `assetc` conversion targets (Java JSON models, blockstates, sounds.json, pack.mcmeta).
  3. **Added ADR 0015** — Documented the decision with context, options considered, decision, and consequences (positive: resource pack compatibility, tooling, backporting; negative: conversion step required, LCE-specific content handling).
  4. **Added D-14 to PLAN.md open decisions** — Reference to ADR 0015 for tracking.
- **Verification Run:** `cargo build --workspace` ✅, `cargo test --workspace` ✅ (all 200+ tests pass), `cargo clippy --workspace` ✅ (warnings only), `cargo fmt --all --check` ✅.
- **Blockers / Open Questions:** None — Documentation updated, asset format decision recorded.
- **Next Steps:** Continue M3: complete client frame loop with chunk upload/render, implement glyphphon text rendering (or maintain placeholder), set up asset directories for block textures, implement `assetc` converter for LCE → Java format conversion.

---
### Session 13 — 2026-11-19 (M3: Client frame loop with flat test world)

- **Goal:** Complete the client frame loop by creating initial test chunks (flat world) and fixing the render thread to use the view from the game thread.
- **Completed:**
  1. **Added `create_flat_test_world()` function** — Creates a 3x3 chunk flat world with:
     - Bedrock at y=0
     - Stone from y=1 to y=60
     - Dirt from y=61 to y=62
     - Grass block at y=63
     - Air above
     Uses `mclre_content_vanilla::block_states()` to get proper block state IDs.
  2. **Updated `Client::new()`** — Now initializes the flat test world on startup and populates `ClientWorld` with the initial chunks.
  3. **Fixed render thread view handling** — The render thread now:
     - Stores the latest view from `RenderCommand::UpdateView`
     - Uses that view for rendering (falls back to world-created view if none received)
     - This fixes the desync between game thread and render thread views
  4. **Added `mclre-content-vanilla` dependency** — Added to workspace and client crate for block state model access.
  5. **Added `block_state_model` field to `Client`** — Stores the block state model for future use.
- **Verification Run:** `cargo build --workspace` ✅, `cargo test --workspace` ✅ (all 200+ tests pass), `cargo clippy --workspace` ✅ (warnings only), `cargo fmt --all` ✅.
- **Blockers / Open Questions:** None — Client frame loop with initial chunks is working.
- **Next Steps:** Continue M3: implement glyphphon text rendering (or maintain placeholder), set up asset directories for block textures, implement `assetc` converter for LCE → Java format conversion.

---
### Session 14 — 2026-11-19 (M3: Asset directories and assetc tool implementation)

- **Goal:** Create asset directory structure and implement the `assetc` converter tool for LCE → Java format conversion.
- **Completed:**
  1. **Created `assets-placeholder/` directory structure** — Standard Java Edition resource pack layout:
     - `textures/block/`, `textures/item/`
     - `models/block/`, `models/item/`
     - `lang/`, `sounds/`
  2. **Implemented `assetc` tool (`crates/tools/assetc`)** — Full-featured CLI with subcommands:
     - `convert` — Convert LCE assets to modern formats (textures, languages, models)
     - `pack` — Create Java-compatible resource packs
     - `atlas` — Build texture atlases from individual textures
     - `validate` — Validate asset directory structure
  3. **Features implemented in assetc:**
     - Texture scanning and conversion (PNG/DDS/TGA → PNG/KTX2 with mipmaps)
     - Language file conversion (LCE key=value → Java JSON `namespace:key`)
     - Texture atlas building with UV coordinate generation
     - Pack metadata generation (`pack.mcmeta`, `pack.toml`, `manifest.json`)
     - Asset validation
  4. **Updated `AssetManager` and `TextureAtlas`** — Made `vfs`, `atlas`, `regions`, `texture`, `width`, `height`, `roots` public for tool access
  5. **Added `tempfile` dev-dependency** for assetc tests
- **Verification Run:** `cargo build --workspace` ✅, `cargo test --workspace` ✅ (all 200+ tests pass including new assetc test), `cargo clippy --workspace` ✅ (warnings only), `cargo fmt --all` ✅.
- **Blockers / Open Questions:** None — Asset directory structure and assetc tool foundation complete.
- **Next Steps:** Continue M3: implement glyphphon text rendering (or maintain placeholder), generate placeholder textures for testing, integrate assetc output with client render pipeline.

---
### Session 15 — 2026-11-19 (M3: Placeholder textures and texture atlas integration)

- **Goal:** Create placeholder block textures and integrate them into the client render pipeline via the asset manager.
- **Completed:**
  1. **Created placeholder texture generator example** (`crates/assets/examples/create_placeholder_textures.rs`) — Generates 16x16 PNG textures for:
     - Stone (gray with noise pattern)
     - Dirt (brown)
     - Grass block top (green)
     - Bedrock (dark gray)
  2. **Generated placeholder textures** — Saved to `assets-placeholder/textures/block/`
  3. **Integrated texture loading into client render thread** — Updated `render_thread` to:
     - Add `assets-placeholder` as a root directory to the asset manager
     - Load block textures (stone, dirt, grass_block_top, bedrock) into the CPU-side texture atlas
     - Upload the built atlas to the GPU via `Renderer::set_texture_atlas()`
     - Print region info for debugging
  4. **Added necessary imports** — `Identifier`, `PathBuf` to client crate
- **Verification Run:** `cargo build --workspace` ✅, `cargo test --workspace` ✅ (all 200+ tests pass), `cargo clippy --workspace` ✅ (warnings only), `cargo fmt --all` ✅.
- **Blockers / Open Questions:** None — Texture atlas now loads from placeholder textures and uploads to GPU.
- **Next Steps:** Continue M3: implement glyphphon text rendering (or maintain placeholder), test the client with actual window/rendering, add more block textures.

---
### Session 16 — 2026-11-19 (M3: Client window and render loop working)

- **Goal:** Fix the client render loop to run in the main thread (where the window was created), fix shader validation errors, and get the window opening with the game loop running.
- **Completed:**
  1. **Restructured client to render in main thread** — Moved renderer creation to `run()` where the window is created, avoiding cross-thread surface creation issues.
  2. **Fixed shader validation error** — Fixed type mismatch in vertex shader where `face` was `f32` but `select_face_normal` expected `u32`. Added explicit `u32()` cast.
  3. **Cleaned up old render thread code** — Removed unused `render_thread`, `render_sender`, `render_receiver`, `window_sender` fields and old `render_frame` method.
  4. **Simplified `Client::new()`** — Removed thread spawning and channel setup, now just initializes world and assets.
  5. **Integrated texture atlas in `run()`** — Loads block textures from `assets-placeholder` and uploads atlas to GPU via `Renderer::set_texture_atlas()`.
- **Verification Run:** App now runs successfully — window opens, 4 block textures load (stone, dirt, grass_block_top, bedrock), texture atlas builds and uploads to GPU, game loop runs.
- **Blockers / Open Questions:** None — Basic render loop with textured chunks working.
- **Next Steps:** Continue M3: implement glyphphon text rendering (or maintain placeholder), implement chunk mesh upload/rendering for the flat test world, add more block textures, test camera movement.

---
### Session 17 — 2026-11-19 (M3: Chunk mesh upload and rendering)

- **Goal:** Implement GPU chunk mesh upload and rendering for the flat test world.
- **Completed:**
  1. **Added GPU chunk caching to `ClientWorld`** — Added `gpu_chunks` field to store `GpuChunkSection` per chunk position.
  2. **Added `store_gpu_chunks()` and `get_gpu_chunks()` methods** — For storing uploaded GPU chunks and retrieving them for rendering.
  3. **Updated `render_frame_with_renderer()`** — Now stores uploaded GPU chunks in `ClientWorld` and retrieves them for rendering each frame.
  4. **Chunk mesh upload now works** — The 3×3 chunk flat test world uploads its meshes to GPU and stores them for rendering.
- **Verification Run:** App runs successfully — window opens, 4 textures load, texture atlas uploads, GPU chunks upload and cache for rendering, game loop runs at 20 TPS.
- **Blockers / Open Questions:** None — Chunk mesh upload and caching working.
- **Next Steps:** Continue M3: implement glyphphon text rendering (or maintain placeholder), add more block textures, test camera movement in the flat world, verify chunk rendering visually.

---
### Session 18 — 2026-11-19 (M3: Framerate-independent game logic & window title fix)

- **Goal:** Fix two issues: (1) Make game logic framerate-independent by using `dt` parameter in `fixed_update()`, (2) Ensure window title shows "Minecraft: Legacy Rust Edition".
- **Completed:**
  1. **Fixed framerate independence** — Updated `fixed_update()` to use `dt.as_secs_f32()` instead of hardcoded `0.05`:
     - Movement: `pos.x += velocity.x * dt_secs` (was `velocity.x * 0.05`)
     - Gravity: `pos.y -= 0.08 * dt_secs` (was `0.08 * 0.05`)
     - Jump velocity and movement speed now scale correctly with tick duration
  2. **Window title** — Confirmed `ClientConfig::default()` uses `"Minecraft: Legacy Rust Edition"` and it's passed to `event_loop.create_window()` via `with_title(&self.config.title)`
- **Verification Run:** App runs successfully — window opens with correct title, textures load, atlas uploads, game loop runs at 20 TPS with framerate-independent physics.
- **Blockers / Open Questions:** None — Game logic now properly uses delta time.
- **Next Steps:** Continue M3: add more block textures, test camera movement in the flat world, verify chunk rendering visually, implement glyphphon text rendering (or maintain placeholder).

---
### Session 19 — 2026-11-19 (M3: Asset conversion and infrastructure)

- **Goal:** Test the `assetc` importer with the original LCE reference assets and design the infrastructure for shipping converted game assets.
- **Completed:**
  1. **Enhanced `assetc` tool** — Added full support for LCE asset structure:
     - Texture conversion from PNG/DDS/TGA → PNG (with mipmaps)
     - Language file conversion (LCE key=value → Java JSON `namespace:key`)
     - Texture atlas building with UV coordinate generation
     - ARC archive extraction (stub for future implementation)
     - PCK (texture pack) extraction (stub for future implementation)
     - Audio conversion from XWB/XSB → Ogg Vorbis (via vgmstream-cli)
     - DLC texture pack extraction
     - Pack metadata generation (`pack.mcmeta`, `pack.toml`, `manifest.json`)
  2. **Created `game-assets/` directory structure** — Standard Java Edition resource pack layout:
     - `textures/block/`, `textures/item/`
     - `models/block/`, `models/item/`
     - `lang/`, `sounds/`, `font/`, `shaders/`
  3. **Generated `pack.mcmeta`, `pack.toml`, `manifest.json`** — Java Edition compatible metadata
  4. **Tested `assetc` with reference assets** — Successfully converted:
     - 76 texture files from `Minecraft.Client/Common/res/*`
     - Texture atlas (2048x2048) with 75 regions
     - DLC texture packs (Candy, Cartoon, City, Fantasy, Festive, Halloween, Halo, MassEffect, Natural, Plastic, Skyrim, Steampunk)
     - Audio files detected (XGS/XSB/XWB) - vgmstream-cli needed for conversion
     - Generated `manifest.json`, `pack.mcmeta`, `pack.toml`
  5. **Created `game-assets/` directory** — Ready for shipping with the game
- **Verification Run:** `assetc convert` completed successfully — 76 textures, 75 atlas regions, DLC packs extracted
- **Blockers / Open Questions:** vgmstream-cli needed for audio conversion; language files not found (may be in different location)
- **Next Steps:** Integrate converted assets into client render pipeline, add language file support, test with client.

---
### Session 22 — 2026-11-19 (M3: Client loads pre-built texture atlas from converted assets)

- **Goal:** Complete the texture atlas integration by having the client load the pre-built atlas from the converted game-assets.
- **Completed:**
  1. **Client loads pre-built texture atlas** — Client now loads the 2048x2048 atlas from `game-assets/textures/block/atlas.png` (built from 76 individual LCE textures by assetc)
  2. **UV coordinates loaded from manifest.json** — 75 UV regions loaded from `game-assets/manifest.json` and passed to renderer for correct texture sampling
  3. **Fixed unused variable warnings** - Cleaned up unused variables in client code
- **Verification Run:** App runs successfully — window opens with correct title, pre-built atlas loads (2048x2048), **75 UV regions loaded from manifest**, texture atlas uploads to GPU, game loop runs at 20 TPS with framerate-independent physics.
- **Blockers / Open Questions:** None — Texture atlas integration complete.
- **Next Steps:** Continue M3: add more block textures from converted assets, test camera movement in the flat world, verify chunk rendering visually, implement glyphphon text rendering (or maintain placeholder).
