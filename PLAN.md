# PLAN: Minecraft: Legacy Rust Edition (MCLRE)

> **Source of truth:** `SPEC.md` (v0.1 draft). If anything in this plan conflicts with `SPEC.md`, `SPEC.md` wins.
> **Live status & session handoff:** See `PROGRESS.md`. Always update `PROGRESS.md` at meaningful checkpoints and before ending a session.

---

## 1. Execution Rules & Architectural Guardrails

Every phase and milestone must uphold these non-negotiable constraints from `SPEC.md` Sections 0, 2, 3, 16, and 18:

1. **Strict milestone gating:** Do not start milestone `M(N+1)` until `M(N)` satisfies all of its acceptance criteria.
2. **Strict client/server separation:** `core`, `world`, `worldgen`, `sim`, `protocol`, `server`, `profiles`, `content-vanilla` must have **zero** dependencies on `wgpu`, `winit`, audio, or UI crates. `client` must not depend on `sim` internals or `server`. Only `app` wires `client` + `server` together (integrated server using the exact same protocol over `InMemoryTransport`).
3. **Vanilla is a mod:** Base content registers through the same public registry/event API exposed to mods.
4. **Namespaced identifiers & block states:** No numeric IDs in game logic, saves, or wire payloads; `Identifier` (`namespace:path`) and `(BlockId, typed properties) -> BlockStateId` from day one. Legacy `(id, meta)` mappings live only in generated `data/legacy/*.toml` for importers/parity tools.
5. **Profiles, not scattered version checks:** Version-specific rules live in `profiles` (`FeatureFlag`, content `since`/`until`, pinned worldgen modules, protocol version, data version). Mechanics check `ctx.profile().has(Feature::X)`.
6. **Determinism & 1:1 seed parity:** All gameplay/worldgen randomness uses `GameRandom` (exact Java `Random` LCG + reference seed derivation + exact call order + deterministic container iteration + exact `f32`/`f64`/lookup math). Cosmetic visuals use `CosmeticRandom`. Gameplay crates must **never** depend on `rand`.
7. **Definition of Done for every gameplay feature:**
   - Reference files/functions cited in `docs/reference/<feature>.md` (pointers, never pasted code).
   - RNG call order, count, and seeding verified against `reference/lce-141217/`.
   - Parity test (golden data, YAML scenario in `parity/`, or seed hash comparison) passes.
   - Any intentional deviation is recorded in `docs/reference/discrepancies.md` and mapped to an allowed improvement (`SPEC.md` §3.3) or a toggleable flag (`faithful_bugs`).
8. **ADR discipline:** Never guess silently on `[DECIDE]` items or spec ambiguities. Write `docs/adr/NNNN-title.md` (context, options, decision, consequences), flag it to the owner, and use the least-irreversible option until confirmed.
9. **Code quality & safety:** `#![forbid(unsafe_code)]` default across crates (only explicitly exempted where required, with `// SAFETY:` + tests); `#![warn(missing_docs)]` on library crates; `rustfmt`; `clippy -D warnings`; zero `unwrap()`/`expect()` on untrusted input; `cargo-deny` + `cargo-audit` in CI.
10. **Asset & reference isolation:** Reference source lives read-only at `reference/lce-141217/` (never edited, never compiled into engine, never copied into `crates/`). Proprietary converted assets live only in `game-assets/` (no `include_bytes!` of proprietary assets). Open CC0/placeholder assets live in `assets-placeholder/` so CI runs cleanly without proprietary files.

---

## 2. Repository & Workspace Blueprint

Target layout inside `mclre/`:

```text
mclre/
├─ Cargo.toml                 # Workspace manifest + shared lints/dependencies
├─ Cargo.lock
├─ rust-toolchain.toml        # Pinned stable MSRV (currently tested on Rust 1.88.0)
├─ deny.toml                  # cargo-deny license & advisory policy
├─ CLAUDE.md                  # Condensed architectural & fidelity rules (SPEC §0.5)
├─ SPEC.md                    # Master specification
├─ PLAN.md                    # This detailed development plan
├─ PROGRESS.md                # Live progress tracker & seamless session handoff
├─ .github/workflows/ci.yml   # Windows + Linux CI (fmt, clippy, tests, cargo-deny, cargo-audit, dep-graph)
├─ reference/
│  └─ lce-141217/             # Read-only reference tree (MinecraftConsoles-141217)
├─ docs/
│  ├─ adr/                    # 0000-template.md, 0001-record-architecture-decisions.md, ...
│  ├─ reference/              # baseline.md, reference-tree.md, rng.md, discrepancies.md, feature-flags.md, <feature>.md
│  ├─ modding/                # content-packs.md, modding guide, hello-block tutorial
│  └─ perf.md                 # Low/mid hardware profiles & benchmark targets
├─ data/
│  └─ legacy/                 # Generated legacy ID/meta -> state mappings (TOML)
├─ assets-placeholder/        # CC0/minimal assets for CI & missing-assets testing
├─ game-assets/               # Output of assetc (isolated proprietary assets; gitignored or LFS per owner)
├─ parity/                    # YAML parity scenarios & golden-data comparison harness
├─ examples/                  # Example WASM mod(s)
└─ crates/
   ├─ core/                   # Identifier, interning, registries, tags, math, GameRandom/CosmeticRandom, NBT, errors, tick types
   ├─ profiles/               # ProfileId, FeatureFlag, GameProfile definitions
   ├─ content-vanilla/        # Vanilla blocks/items/entities/recipes via public mod/registry API
   ├─ world/                  # BlockState, palette sections, lighting, Anvil region storage, dimensions, WorldSizePreset
   ├─ worldgen/               # Versioned terrain/biome/cave/structure generators + deterministic noise
   ├─ sim/                    # Fixed 20 TPS server simulation, physics/AABB, entities, AI, fluids, redstone, explosions
   ├─ protocol/               # Transport-agnostic typed packets (Encode/Decode), v{N} modules, handshake/config/play
   ├─ net/                    # Transport trait, InMemoryTransport, QuicTransport, mDNS/UDP LAN discovery
   ├─ server/                 # Server library (integrated + dedicated), ClientConnection adapter, host privileges
   ├─ server-bin/             # `mclre-server` headless CLI binary
   ├─ client/                 # ClientWorld mirror, prediction, interpolation, local player(s), UI state machine
   ├─ render/                 # wgpu frame graph (Iris-mapped pass structure), chunk meshing, AO/smooth lighting, viewports
   ├─ ui/                     # Retained-mode data-driven UI toolkit (RON/JSON layouts, 9-slice, gamepad focus nav)
   ├─ audio/                  # 3D positional audio, multi-listener splitscreen mixing, music selection
   ├─ input/                  # winit + gilrs action mapping, per-player device bindings
   ├─ assets/                 # Asset VFS, resource/content packs, texture atlas/array, localization
   ├─ mod-api/                # Guest-side SDK crate for WASM mods
   ├─ modding/                # Host-side mod manifest, dependency resolver, WASM sandbox host, event bus
   ├─ identity/               # IdentityProvider (LaunchArgIdentity, guest UUIDs) & EntitlementProvider
   ├─ app/                    # `mclre` client binary, winit loop, integrated server wiring
   └─ tools/
      ├─ assetc/              # Offline LCE asset/archive/audio/texture/lang converter
      ├─ devtools/            # Reference data extractors, registry dumpers, worldgen/packet inspectors
      └─ lce-import/          # LCE world save importer (post-v1.0)
```

---

## 3. Open Decisions (`[DECIDE]`) & Reference Verifications (`[VERIFY]`)

### 3.1 `[VERIFY]` Items (Must resolve in Phase 0 / M0 from `reference/lce-141217/`)

| ID | Topic | Question to Resolve | Target Doc | Blocks |
|---|---|---|---|---|
| V-01 | Reference tree layout | Exact layout of `Minecraft.World`, `Minecraft.Client`, build system, and asset paths | `docs/reference/reference-tree.md` | M0/M1 |
| V-02 | Baseline TU/CU | Confirm Dec 17, 2014 build TU/CU number (expected TU19 / CU7) & final XB1/PS4 TU | `docs/reference/baseline.md` | M1 |
| V-03 | World sizes & edge | Exact block/chunk dimensions for `classic` (864×864?), `small`/`medium`/`large` (1024/3072/5120?), Nether/End scales, and world-edge barrier/ocean behavior | `docs/reference/baseline.md` | M2/M6/M13 |
| V-04 | Math & RNG details | Sine lookup tables (`Mth`), float/double widths, `java.util.Random` / chunk seed XOR constants | `docs/reference/rng.md` | M1/M6 |
| V-05 | Asset formats | Exact archive formats (`.arc`, `.pck`, `.loc`, `.binka`, `.dds`, UI SWF/assets) | `docs/reference/reference-tree.md` | M3/M11 |
| V-06 | Splitscreen audio | Reference behavior for multi-listener sound spatialization/attenuation | `docs/reference/audio.md` | M11 |

### 3.2 `[DECIDE]` Items (Record ADRs in `docs/adr/`, flag to owner before dependent milestones)

| ID | Topic | Options & Recommended Default | Target ADR | Must Confirm Before |
|---|---|---|---|---|
| D-01 | Source & asset policy | Formalize private repo derivative-reference policy + `game-assets/` isolation + Git LFS vs plain gitignore | `0002-reference-and-asset-policy.md` | M0 |
| D-02 | Namespace policy | `minecraft:` for Java-equivalent IDs, `lce:` only for LCE-exclusives | `0003-identifier-namespaces.md` | M1 |
| D-03 | Infinite world inside finite bounds | Whether `infinite` matches finite-preset terrain inside bounds or uses unbounded biome scale | `0004-infinite-world-bounds-behavior.md` | M6 |
| D-04 | Server ECS | **`bevy_ecs` standalone** behind `sim::entity` wrapper vs `hecs`/custom | `0005-ecs-selection.md` | M4/M7 |
| D-05 | NBT crate | **`simdnbt`** vs **`fastnbt`** (Java-compatible NBT read/write) | `0006-nbt-crate.md` | M1/M2 |
| D-06 | Light propagation authority | **Server computes & sends light** vs client recomputes | `0007-lighting-authority.md` | M2/M4 |
| D-07 | Text rendering crate | **`glyphon` (cosmic-text)** + custom bitmap pixel font path vs `fontdue` | `0008-text-rendering.md` | M3/M8 |
| D-08 | Chunk meshing strategy | Per-face meshing first; **greedy meshing disabled by default** if AO/smooth lighting diverges | `0009-chunk-meshing-ao.md` | M3 |
| D-09 | Identity & splitscreen guests | Missing `--username` behavior (refuse in prod/MP, fallback `Player` in `--dev-defaults`) + deterministic guest UUIDs (UUIDv5 from primary + slot vs persisted UUIDv7) | `0010-identity-and-guests.md` | M4/M10 |
| D-10 | Networking transport & discovery | **`quinn` (QUIC)** with TOFU/LAN self-signed certs + **`mdns-sd`** (plus optional UDP broadcast) | `0011-networking-transport-discovery.md` | M9 |
| D-11 | Audio crate & splitscreen mix | **`kira`** (or `rodio` + `symphonia`) + per-listener attenuation mixing; external `vgmstream` vs bundled decoder in `assetc` | `0012-audio-and-assetc-decoders.md` | M3/M11 |
| D-12 | Modding WASM & native mods | **`wasmtime`** (WIT component model) + optional dev-only native Rust mods flag | `0013-modding-runtime.md` | M12 |
| D-13 | Perf targets, finite→infinite migration, shaders/license | Confirm low/mid hardware budgets (`docs/perf.md`), finite→infinite migration support, `faithful_bugs` defaults, clean-room vs LGPL Iris policy, and own code license | `0014-perf-licensing-and-scope.md` | M3 / Post-1.0 |
| D-14 | Asset formats: Java Edition parity | Use Java Edition resource/data pack formats (JSON models, blockstates, sounds.json, pack.mcmeta) for all modern assets | `0015-asset-format-java-parity.md` | M3/M8 |

---

## 4. Detailed Phase & Milestone Plan

### Phase 0 — Reference Survey & Pre-M0 Groundwork
*Goal:* Establish exact facts from the reference source and clean repository boundaries before creating workspace crates.

1. **Git repository boundary check:**
   - Note that `C:\Users\DJ\Desktop` currently has an uninitialized `.git` directory. Initialize a dedicated git repository inside `C:\Users\DJ\Desktop\mclre\` (confirm with owner so parent Desktop `.git` doesn't interfere) with `.gitignore` covering `target/`, temporary files, and configurable `game-assets/` policy.
2. **Attach reference tree:**
   - Add `https://github.com/furax124/Minecraft-LCE-PS3` (specifically `MinecraftConsoles-141217`) as a read-only submodule (or git-ignored read-only clone) at `reference/lce-141217/`.
3. **Reference inventory & `[VERIFY]` resolution:**
   - Inspect `Minecraft.World` and `Minecraft.Client` directory layouts, C++/header structure, data tables, RNG/noise/math classes (`Random`, `Mth`, `PerlinNoise`, `BiomeSource`, `IntCache`, `Layer`), and asset directories.
   - Resolve `V-01` through `V-05` and record findings in:
     - `docs/reference/reference-tree.md` (verified layout + crate mapping table)
     - `docs/reference/baseline.md` (exact TU/CU, world sizes, Nether/End scale, edge behavior)
     - `docs/reference/rng.md` (RNG, sine table, seed derivation formulas)
     - `docs/reference/discrepancies.md` (template + any early findings)
     - `docs/reference/feature-flags.md` (template)
   - Update `SPEC.md` `[VERIFY]` markers if any spec assumption needs correction (or record in `baseline.md` + flag to owner).
4. **Governance & ADR scaffolding:**
   - Create `CLAUDE.md` (condensed `SPEC.md` §§2, 3.5, 3.6, 4, 15, 16, 18).
   - Create `docs/adr/0000-template.md` and `docs/adr/0001-record-architecture-decisions.md`.
   - Draft initial ADRs for `D-01` through `D-13` (especially `D-01`, `D-02`, `D-04`, `D-05`, `D-06`, `D-07`, `D-10`, `D-11`) and present open decisions to owner.
5. **Exit criteria:** Reference tree attached read-only, `reference-tree.md` & `baseline.md` written, `CLAUDE.md` & ADR template/initial ADRs created.

---

### M0 — Foundation
*Goal:* Reproducible Cargo workspace, strict dependency enforcement, CI pipeline, logging/error conventions.

1. **Toolchain & workspace setup:**
   - Create `rust-toolchain.toml` (pin stable MSRV matching environment, e.g., `1.88.0`, with `rustfmt` & `clippy` components).
   - Create root `Cargo.toml` with `[workspace]`, `resolver = "2"`, `edition = "2024"`, shared `[workspace.package]`, `[workspace.lints]` (`unsafe_code = "forbid"`, `missing_docs = "warn"` on libs, clippy pedantic selection), and `[workspace.dependencies]`.
   - Create skeleton crates (`Cargo.toml` + `src/lib.rs` or `src/main.rs`) for all crates listed in `SPEC.md` §4 (`core`, `profiles`, `content-vanilla`, `world`, `worldgen`, `sim`, `protocol`, `net`, `server`, `server-bin`, `client`, `render`, `ui`, `audio`, `input`, `assets`, `mod-api`, `modding`, `identity`, `app`, `tools/assetc`, `tools/devtools`), plus `parity/` harness crate.
2. **Conventions & observability:**
   - Standardize `thiserror` error types in libs, `anyhow` in bins/tools, and `tracing` + `tracing-subscriber` init helper in `core` / `app` / `server-bin`.
   - Implement an automated **dependency-graph test** (using `cargo metadata` in an integration test) verifying:
     - `core`, `world`, `worldgen`, `sim`, `protocol`, `server`, `profiles`, `content-vanilla` never transitively depend on `wgpu`, `winit`, `audio`, `ui`, or `client`.
     - `core`, `world`, `worldgen`, `sim`, `content-vanilla` never depend on `rand`.
     - `client` never depends on `sim` or `server`.
     - `server-bin` has zero GPU/windowing/audio dependencies.
3. **Security, license & CI:**
   - Configure `deny.toml` (`cargo-deny`: allow MIT/Apache-2.0/BSD/ISC/Zlib/MPL-2.0, flag GPL/LGPL, deny known advisories).
   - Install/verify `cargo-deny` and `cargo-audit` locally (note: currently not installed in environment; add setup notes).
   - Create `.github/workflows/ci.yml` running on `windows-latest` and `ubuntu-latest`:
     - `cargo fmt --all -- --check`
     - `cargo clippy --workspace --all-targets -- -D warnings`
     - `cargo test --workspace`
     - Dependency-graph check
     - `cargo deny check` & `cargo audit`
4. **Acceptance criteria (`SPEC.md` §17):**
   - `cargo test --workspace` passes on Windows + Linux.
   - Deliberately adding `wgpu` to `server` causes the dependency-rule test to fail.
   - CI green; tagged commit `m0-foundation`.

---

### M1 — Core Data Model
*Goal:* Identifiers, freeze-able registries & tags, block-state flattening, exact `GameRandom` + deterministic noise, NBT, and `profiles` skeleton.

1. **`core` — Identifiers & interning:**
   - `Identifier` (`namespace`, `path`) with validation (`[a-z0-9_.-]` / `[a-z0-9_.-/]`), default `minecraft` only for user-facing parse helpers, never in storage.
   - Thread-safe string interner yielding compact `Copy` handles (`InternedId` / `IdentifierId`) for hot paths.
   - Property-based tests (`proptest`) for valid/invalid identifier parsing and canonical round-trips.
2. **`core` — Registries & Tags:**
   - `Registry<T>` lifecycle: `Registering -> Frozen`; dense `RegistryId<T>` numeric indexing assigned deterministically on freeze; iteration preserves deterministic registration order.
   - `TagKey<T>` and `TagRegistry` supporting JSON tag loading (`#minecraft:logs` style) and nested tag resolution with cycle detection.
   - Registry snapshot structure for save compatibility and server→client registry sync.
3. **`core` / `world` — Flattened Block & BlockState model:**
   - `Block` definition with typed `Property` declarations (`bool`, `int(min..=max)`, `enum`).
   - Global dense `BlockStateId` table computed deterministically at registry freeze; bidirectional lookup `BlockStateId <-> (BlockId, PropertyMap)`.
   - Property-based tests ensuring all state indices are contiguous, injective, and order-deterministic across platforms.
4. **`core` — `GameRandom` & `CosmeticRandom`:**
   - Exact port of Java `java.util.Random` LCG (`multiplier = 0x5DEECE66D`, `addend = 0xB`, `mask = (1 << 48) - 1`, `next(bits)`, `nextInt`, `nextInt(bound)` rejection loop, `nextLong`, `nextFloat`, `nextDouble`, `nextGaussian` Box-Muller cache, `setSeed`).
   - Reference seed-derivation helpers (string hash to 64-bit seed, chunk coordinate mixing, biome layer seeds) verified against `reference/lce-141217/` and documented in `docs/reference/rng.md`.
   - `CosmeticRandom` wrapper (can wrap `rand`) isolated to client/cosmetic use; gameplay crates strictly use `GameRandom`.
   - Unit tests against known Java `Random` and reference LCE test vectors; CI cross-OS bit-identity test.
5. **`worldgen` / `core` — Deterministic math & noise primitives:**
   - Exact port of reference `Mth` (including sine/cosine lookup table if used by reference), `ImprovedNoise` / `PerlinNoise` / `SimplexNoise` matching reference `f32`/`f64` operations and operation order.
6. **`core` — NBT & `profiles` skeleton:**
   - Java-compatible NBT reader/writer in `core` (using `simdnbt` or `fastnbt` per ADR `D-05`, wrapped in a clean internal API) with fuzz/proptest coverage.
   - `profiles` crate: `ProfileId` (`lce:tu19` or verified baseline ID), `FeatureFlag` enum, `Profile` struct (`id`, `data_version`, `protocol_version`, `world_height`, `feature_flags`, `since`/`until` content queries).
7. **`content-vanilla` & `tools/devtools` initial proof:**
   - Register ~20 representative test blocks (including multi-property blocks like logs, stairs, slabs, water) through the public registration API.
   - Start `tools/devtools` extractor scaffolding to parse reference block/item/recipe tables into `data/legacy/blocks.toml` and golden test JSON/TOML.
8. **Acceptance criteria (`SPEC.md` §17):**
   - ~20 test blocks registered via public API; property-based tests for state indexing pass.
   - `GameRandom` & seed-derivation vectors match reference outputs bit-for-bit across Windows and Linux.
   - `GameRandom`/`CosmeticRandom` split enforced by dependency test (`rand` forbidden in gameplay crates).
   - Tagged commit `m1-core-data-model`.

---

### M2 — World Storage & Chunks
*Goal:* Modern section-based chunk representation, lighting arrays, Anvil region storage, DataFixer pipeline, async chunk I/O, and bounded + unbounded world coordinates.

1. **`world` — Chunk & Section model:**
   - `ChunkPos(i32, i32)`, `SectionPos(i32, i32, i32)`, `BlockPos(i32, i32, i32)`.
   - `WorldSizePreset` (`Classic`, `Small`, `Medium`, `Large`, `Infinite`) carrying exact bounds, Nether scale, End layout, and height from verified `baseline.md`.
   - `ChunkSection` (16×16×16): bit-packed palette-compressed `BlockStateId` storage (single-value, indirect palette, direct global palette) + biome palette + 4-bit sky light & block light arrays (`NibbleArray`).
   - `ChunkColumn`: configurable vertical section count (default matching baseline profile height, e.g., 8 sections = 128 blocks or 16 sections = 256 blocks per `baseline.md` verification), heightmaps, block entities map (deterministic ordering), chunk status (`Unloaded -> Loading -> Generated -> Lit -> Ready -> Ticking -> Unloading`).
2. **`world` — Anvil region storage & crash safety:**
   - Custom Anvil `.mca` reader/writer (`region/r.X.Z.mca`) using zlib/lz4 compression and namespaced block-state palettes in NBT.
   - Top-level `level.dat` (NBT): `data_version`, `profile_id`, `world_size_preset`, `seed`, dimension data, game rules, registry snapshot.
   - Atomic save path: write temp file → flush/sync → atomic rename, plus corrupt-chunk recovery (log + skip/backup, never panic or silent data loss).
   - Works identically for both bounded legacy presets (coordinate bounds enforcement) and `Infinite` worlds (streaming region files across `±30,000,000`).
3. **`world` — DataFixer & Async I/O:**
   - `DataFixer` trait & versioned migration chain keyed by monotonic `data_version` + `ProfileId`.
   - Background chunk I/O worker pool (`crossbeam` / dedicated I/O threads) with request prioritization and clean shutdown flush.
4. **Testing:**
   - Random chunk write → read round-trip property tests across single-value, indirect, and direct palettes.
   - Crash-safety test simulating interrupted temp writes / truncated region sectors.
   - Manual verification: open a generated `.mca` region in an external Java Anvil viewer/tool.
5. **Acceptance criteria (`SPEC.md` §17):**
   - Round-trip, crash-safety, and bounded+infinite storage tests pass; external Anvil tool inspection succeeds.
   - Tagged commit `m2-world-storage`.

---

### M3 — Minimal Client Rendering
*Goal:* `wgpu` window & backend portability, `GpuCaps` negotiation, staged Iris-compatible frame graph, placeholder atlas, chunk meshing with AO/smooth lighting, and debug overlay.

1. **`assets` (baseline):**
   - Virtual filesystem (`AssetVfs`) loading `assets-placeholder/` (and optional `--assets` directory) with clean "assets missing" fallback behavior.
   - **Asset format policy:** Use **Minecraft: Java Edition resource/data pack formats** for all modern assets (JSON block/item models, blockstates, language files, pack manifests). This enables direct Java resource pack compatibility and easier backporting.
   - Texture loader (`image`) + block texture atlas / texture array builder + mipmap generation + animated texture frame scheduler stub.
2. **`input` (baseline) & `app` shell:**
   - `winit` event loop + CLI parsing (`clap`) implementing `SPEC.md` §19 flags (`--gpu-backend`, `--assets`, `--dev-defaults`, `--log`, etc.).
   - Basic keyboard/mouse camera controller (fly camera for flat-world benchmark).
3. **`render` — wgpu core & Render Graph:**
   - Backend selection order: Windows (`Vulkan -> DX12 -> GL`), Linux (`Vulkan -> GL`), macOS (`Metal`), selectable via `--gpu-backend`.
   - `GpuCaps` probing (limits, BCn/ASTC support, texture arrays, MDI/compute capability flags) with simple fallback paths.
   - Explicit pass-based frame graph structured for future shaderpack mapping: `Shadow (optional) -> GBufferOpaque -> GBufferCutout -> Sky/Clouds -> GBufferTranslucent -> Composite/Post -> UI`.
   - WGSL vanilla shaders mirroring `gbuffers_terrain` style, with vertex attributes already structured to carry position, face/normal, atlas UV, per-vertex AO/light, biome tint, and future block/midblock IDs.
4. **`render` — Chunk meshing & culling:**
   - Worker-thread (`rayon`) meshing per 16³ section using neighbor borders.
   - Face culling, per-vertex AO / smooth lighting, separate opaque / cutout / translucent vertex buffers, compact packed vertex format.
   - Frustum culling per section + upload budget (MB/frame) + priority queue (closest to camera first).
5. **Dev UI (`egui` behind `dev-ui` feature):**
   - FPS, frame time breakdown, draw calls, meshed sections, active GPU backend & `GpuCaps`.
6. **Performance verification & `docs/perf.md`:**
   - Record baseline low/mid hardware definitions in `docs/perf.md`.
   - Scripted flat-world flythrough benchmark (`criterion` + headless/render timing).
7. **Acceptance criteria (`SPEC.md` §17):**
   - Renders a 32-chunk-radius flat world at `>= 60 FPS` on mid-range hardware.
   - Vulkan, DX12, and GL backends all initialize and render cleanly on Windows (plus Vulkan/GL on Linux).
   - Tagged commit `m3-minimal-render`.

---

### M4 — Server Loop, Protocol, Integrated Server & Player Movement
*Goal:* Authoritative 20 TPS server, transport-agnostic protocol v1, integrated server via `InMemoryTransport`, headless client for CI, and shared deterministic player physics.

1. **`identity`:**
   - `IdentityProvider` trait + `LaunchArgIdentity` (`--username`, `--uuid` validated strictly as UUID v7, `--dev-defaults` fallback).
   - `EntitlementProvider` trait (accept-all stub).
   - Deterministic splitscreen guest UUID derivation per ADR `D-09`.
2. **`protocol` & `net`:**
   - Custom `Encode` / `Decode` traits (explicit binary layout, `bytes` crate, bounded lengths/strings, zero panics on malformed input).
   - `protocol::v1` packets covering states: `Handshake -> Login -> Configuration (registry sync, profile, feature flags, world preset) -> Play (keepalive, chunk data + light + delta updates, player position/look, block dig/place, disconnect)`.
   - `Transport` trait (`reliable_ordered`, `reliable_unordered`, `unreliable`) + `InMemoryTransport` (crossbeam/tokio channels).
   - `cargo-fuzz` target (or property fuzz test) for `protocol::v1` decoder.
3. **`world` / `core` shared physics:**
   - Pure deterministic physics module (AABB collision, gravity, drag, step-up, sneaking edge-clamp, jumping, water/lava buoyancy, flying) shared by `sim` (server authority) and `client` (prediction).
4. **`server` & `sim` (foundation):**
   - `Server::new(config, profile, world_storage, transport)` running fixed 50 ms (20 TPS) simulation loop on dedicated OS thread; `ClientConnection` abstraction isolating game logic from wire encoding.
   - Fixed tick order: inbound packets → player tick → block actions → chunk loading/streaming → outbound flush → autosave scheduling; "can't keep up" lag logging.
   - Server-side validation: movement plausibility, block reach/line-of-sight, rate limits.
5. **`client` & `app` integration:**
   - `ClientWorld` chunk mirror, client-side prediction + server reconciliation, block breaking/placing interaction, hotbar stub.
   - Headless client mode in `client` (no window/GPU) for automated integration tests.
   - `app` launches integrated `Server` + `Client` over `InMemoryTransport` (no shortcuts).
6. **Acceptance criteria (`SPEC.md` §17):**
   - Singleplayer flat world playable: walk, jump, swim, fly, place/break blocks.
   - Headless client ↔ server integration test exercises the exact same protocol path and passes in CI.
   - Tagged commit `m4-server-protocol-movement`.

---

### M5 — Faithful Core Gameplay
*Goal:* Complete baseline-profile core mechanics, vanilla content via public API, LCE crafting UX logic, lighting, fluids, fire, TNT, and basic redstone.

1. **Reference extraction & golden data (`tools/devtools`):**
   - Extract baseline profile tables from `reference/lce-141217/`: blocks (hardness, blast resistance, tool tier, flammability, light emission/opacity), items, tools, food, crafting recipes + LCE recipe groups/categories, smelting, drops.
   - Generate `data/legacy/blocks.toml` and golden-data test fixtures; document each system in `docs/reference/<feature>.md`.
2. **`content-vanilla` (via public API):**
   - Register all baseline-profile blocks, items, tools, armor, food, dyes, and recipes through the same registration API mods use.
3. **`sim` — Core gameplay mechanics (profile-flagged):**
   - Block breaking progress, tool effectiveness, fortune/silk touch/harvest drops.
   - Inventories & containers (player inventory, chest, double chest, crafting table, furnace smelting + fuel, dispenser/dropper if in baseline).
   - **LCE-style crafting system:** console-style recipe groups/tabs, 2×2 and 3×3 recipe matching, ingredient availability checking, plus classic grid crafting option if applicable.
   - Health, hunger/saturation, natural regeneration, starvation, fall/drowning/lava/fire/void damage, armor reduction, death & respawn, XP levels & curves.
   - Day/night cycle (20-minute cycle), sky/block lighting propagation (`0..=15`) matching reference semantics, incremental updates on block changes.
   - Fluids (water/lava spread, infinite water sources, obsidian/cobblestone formation), fire spread & burn-out, TNT & reference explosion raycasting/damage/knockback, basic redstone (dust, torch, lever, button, pressure plate, repeater, piston, door, trapdoor, TNT ignition).
   - `faithful_bugs` toggle integration for documented baseline quirks.
4. **Parity & golden-data tests:**
   - Golden-data tests for every recipe, hardness, blast resistance, and drop table.
   - YAML scenarios in `parity/` for crafting, furnace, fluids, lighting, explosion, and redstone behavior.
5. **Acceptance criteria (`SPEC.md` §17):**
   - All golden-data tests and `parity/` core gameplay scenarios pass; every gameplay feature has its `docs/reference/<feature>.md` entry.
   - Tagged commit `m5-faithful-core-gameplay`.

---

### M6 — World Generation (Baseline)
*Goal:* 1:1 seed-compatible Overworld, Nether, and The End generators for the baseline profile across legacy presets and infinite mode.

1. **Reference worldgen analysis (`docs/reference/worldgen.md`):**
   - Document exact biome layer stack (`GenLayer` chain + world-size biome scaling), terrain noise octaves, density sampling, surface replacement, caves/ravines, structure placement regions/spacing, decoration order, ore distribution, Nether, and End generation from `reference/lce-141217/`.
2. **Reference seed-parity harness & differential debugger (`parity/` + `tools/devtools`):**
   - Establish reference dumps (per owner choice: standalone reference worldgen runner or captured dumps from original) for fixed seeds × presets (`Classic`, `Small`, `Medium`, `Large`) × dimensions: biome maps, heightmaps, per-chunk block hashes, structure coordinates, spawn point.
   - Build differential debugger reporting first mismatching chunk → first differing stage (biomes → heightmap → caves → structures → decoration) → exact `GameRandom` state.
3. **`worldgen` — Frozen baseline modules (`worldgen::v_tu19` or verified TU):**
   - Seed input parser (text string hashCode, numeric strings, negative 64-bit, empty seed random pick).
   - Biome registry + scaled biome layer stack per `WorldSizePreset`.
   - Overworld terrain, surface rules, caves, ravines, lakes, dungeons, mineshafts, villages, strongholds, temples/witch huts, decoration/trees/ores.
   - Nether (`minecraft:the_nether`) + Nether fortresses + glowstone/quartz/lava oceans.
   - The End (`minecraft:the_end`) + obsidian pillars + exit portal structure.
   - World-edge behavior for bounded presets (ocean/barrier per verified `baseline.md`).
   - Nether portal linking & coordinate scaling per `WorldSizePreset`.
   - Parallel chunk generation (`rayon`) preserving exact per-chunk & cross-border RNG determinism.
4. **Acceptance criteria (`SPEC.md` §17):**
   - Reference seed tests match dumps for biome maps, heightmaps, structures, and terrain within parity region (target: exact match) on both Windows and Linux.
   - Visual side-by-side signoff with owner captures.
   - Tagged commit `m6-worldgen-baseline`.

---

### M7 — Mobs & AI
*Goal:* Finalized entity framework, baseline passive/hostile/utility mobs, spawning, AI goals, combat, breeding, trading, and bosses.

1. **`sim::entity` architecture:**
   - Implement `bevy_ecs` (or ADR-confirmed ECS) behind `sim::entity` abstraction; register entity types via public API.
   - Entity components: transform, velocity, AABB, health, status effects, equipment, NBT serialization, deterministic iteration order for AI/RNG.
2. **Navigation & AI goal selector:**
   - Reference-faithful goal priority/mutex bitmask system, chunk-cached pathfinding with faithful limits, look/random-stroll/tempt/panic/melee/ranged/avoid goals.
3. **Baseline entities:**
   - Passive: Pig, Cow, Mooshroom, Sheep, Chicken, Squid, Wolf, Ocelot, Horse (if in baseline TU), Bat, Villager (baseline career/trading model).
   - Hostile: Zombie, Skeleton, Creeper, Spider, Cave Spider, Enderman, Slime, Magma Cube, Ghast, Zombie Pigman, Blaze, Witch, Silverfish.
   - Bosses: Ender Dragon (and Wither if in baseline TU).
   - Non-living entities: Item entity, XP orb, Arrow, Snowball, Egg, Ender Pearl, Potion, Falling Block, Primed TNT, Minecart, Boat, Painting/Item Frame.
4. **Spawning & combat:**
   - Natural mob spawning cycle, mob caps (configurable above console defaults), light/biome spawn rules, spawners.
   - Baseline combat model (no Java 1.9 attack cooldown, sword blocking if in baseline, knockback, critical hits, enchantments/potions).
5. **Acceptance criteria (`SPEC.md` §17):**
   - Entity behavior `parity/` scenarios pass (AI, spawning, breeding, combat, drops).
   - Server meets tick budget (`<= 25 ms`, p99 `<= 45 ms`) under target entity load.
   - Tagged commit `m7-mobs-ai`.

---

### M8 — UI/UX Parity Pass 1 & `assetc` v1
*Goal:* Retained-mode data-driven UI reproducing LCE menus/HUD/crafting UX, full gamepad + KB/M parity, and `assetc` importing textures/lang/UI assets.

1. **`tools/assetc` (v1 — textures, lang, models):**
   - Extract LCE archives (`.arc` / pack files per verified `V-05`).
   - Convert textures (DDS/PNG → PNG/KTX2) and language files → **Java Edition JSON format** into `game-assets/`, emitting source-hash manifest (idempotent & resumable).
   - Convert LCE block/item models → **Java Edition JSON model format** (parent models, elements, faces, variants/multipart).
   - Convert LCE pack manifests → **Java Edition `pack.mcmeta` compatible format** with `pack.toml`/`pack.json`.
   - Keep `assets-placeholder/` functional for CI.
2. **`input` (full):**
   - `winit` + `gilrs` action system, per-player device assignment, LCE default gamepad bindings (LB/RB tabs, X/Y quick-move, pointerless inventory cursor/grid nav) and KB+M bindings, control remapping persisted in `options.toml`.
3. **`render` — Sky, atmosphere & visual parity:**
   - Sky dome, sun/moon phases, stars, fog, clouds, rain/snow weather, particles (`CosmeticRandom`), skeletal entity/block-entity models (chests, signs), 3D/2D item rendering, first-person hand.
4. **`ui` — Retained-mode toolkit:**
   - wgpu UI renderer + text rendering (`glyphon` / bitmap pixel font per ADR `D-07`) + 9-slice panel styling + focus graph navigation + scene stack.
   - Data-driven layout/style definitions (RON/JSON) with scale + safe-area support.
5. **Screens & HUD:**
   - Title screen, Load/Create World (profile selector, `WorldSizePreset`, seed, host options, superflat/bonus chest), Pause, Settings (`options.toml`), Controls remap, Help & Options, Texture Pack menu, Host Options overlay.
   - HUD (hearts, armor, hunger, air, XP bar, hotbar, tooltips/item names, vignette, crosshair, paper doll if in baseline).
   - In-game containers: Inventory, LCE Crafting Menu (tabbed groups + quick craft), Furnace, Chest, Enchanting Table, Anvil, Brewing Stand, Villager Trading, Sign edit.
6. **Acceptance criteria (`SPEC.md` §17):**
   - Entire menu & gameplay flow operable exclusively with gamepad AND exclusively with KB+M.
   - Visual comparison sign-off against owner reference screenshots/videos.
   - Tagged commit `m8-ui-parity`.

---

### M9 — LAN Multiplayer & Dedicated Server
*Goal:* Production-ready LAN multiplayer (`quinn` QUIC + `mdns-sd` discovery), dedicated server binary `mclre-server`, host privileges, chat, and entity interpolation.

1. **`net` — QUIC transport & LAN discovery:**
   - `QuicTransport` (`quinn`) supporting reliable ordered streams, unreliable datagrams, TLS self-signed cert (TOFU/LAN mode per ADR `D-10`), compression (`zstd`/`lz4_flex`) above threshold.
   - `mdns-sd` (+ optional UDP broadcast) advertising MOTD, profile ID, protocol version, player count, world size preset, host display name.
2. **`protocol` & `client`/`server` multiplayer:**
   - Entity spawn/despawn/move/teleport/metadata/equipment/animation packets, client-side entity interpolation, player list, chat/system messages, host privilege enforcement & UI toggling.
   - Anti-cheat/rate-limiting baseline & packet decode fuzzing (`cargo-fuzz`).
3. **`server-bin` (`mclre-server`):**
   - Headless dedicated server reading `server.toml` + CLI overrides (`SPEC.md` §19), interactive `--headless-console` command input, graceful Ctrl+C / SIGTERM shutdown with world save.
   - Verify zero GPU/windowing/audio dependencies in CI.
4. **UI additions:**
   - Play Game / Join LAN server browser (auto-discovered + direct IP:port connect), in-game player list & host privilege toggles.
5. **Acceptance criteria (`SPEC.md` §17):**
   - 4 remote players on LAN complete a 1-hour soak test without crashes, desyncs, or memory leaks.
   - Packet decoder fuzz target runs cleanly; `mclre-server` passes headless dependency gate.
   - Tagged commit `m9-lan-multiplayer`.

---

### M10 — Splitscreen
*Goal:* Up to 4 local players on one client process with independent viewports, cameras, inputs, HUDs, inventories, and server connections, usable solo or alongside LAN players.

1. **`identity` & `input`:**
   - Assign up to 4 local devices (e.g., 4 gamepads or 1 KB+M + 3 gamepads) and generate deterministic guest identities (`LocalIdentity` slot 0..3).
2. **`client` & `server`:**
   - Open N independent `InMemoryTransport` connections to the integrated server (each local player is a full server-side player entity with independent inventory, crafting screen, reach, and host privileges).
   - Dynamic join/leave of local players 2–4 (press Start/A to join).
3. **`render` & `ui`:**
   - Multi-viewport render graph pass execution: shared chunk meshes & texture atlas, per-viewport camera/frustum culling/uniforms, horizontal, vertical, and 4-way quad layouts, per-viewport UI scale & safe areas.
4. **Acceptance criteria (`SPEC.md` §17):**
   - 4-player splitscreen runs smoothly at `>= 30 FPS` on the "mid" hardware profile, including mixed with remote LAN players.
   - Per-player inventories, crafting menus, and input bindings operate independently without focus bleed.
   - Tagged commit `m10-splitscreen`.

---

### M11 — Audio
*Goal:* Full sound & music engine with LCE asset conversion, 3D spatial attenuation, multi-listener splitscreen support, and non-fatal failure isolation.

1. **`tools/assetc` (audio extension):**
   - Convert LCE `.binka` audio to `.ogg` (Ogg Vorbis) / `.wav` (delegating to `vgmstream` or selected decoder per ADR `D-12`) and generate **Java Edition `sounds.json` format** mappings.
2. **`audio` crate:**
   - Isolated audio thread (`kira` or `rodio` + `symphonia` per ADR `D-12`); initialization failure logs a warning and degrades to silent mode without crashing.
   - `SoundEvent` registry + `sounds.json` pitch/volume/variant selection.
   - 3D positional audio with LCE-matching attenuation curve; multi-listener splitscreen spatialization policy (ADR `D-12` / `V-06`).
   - Background music scheduler matching LCE's track selection & cooldown timings (menu, creative, survival, Nether, End).
3. **Acceptance criteria (`SPEC.md` §17):**
   - Parity checklist for key sound events (blocks, mobs, UI, weather, music) verified.
   - Simulated missing/broken audio device test confirms non-fatal fallback.
   - Tagged commit `m11-audio`.

---

### M12 — Modding v1
*Goal:* Sandboxed WASM mods, data/resource packs, mod channel networking, dependency resolution, and developer documentation.

1. **`mod-api` (guest SDK) & `modding` (host):**
   - `mod.toml` manifest parser + semver dependency resolver (topological load order, cycle/missing-dep diagnostics).
   - WASM host (`wasmtime` component model / WIT per ADR `D-13`) with strict sandboxing: tick fuel budgets, memory caps, declared permissions, deterministic host functions (no wall-clock in simulation callbacks).
   - Typed cancellable Event Bus (`BlockBreak`, `BlockPlace`, `EntityDamage`, `PlayerJoin`, `Tick`, `ChunkGen`, `ItemUse`, `Explosion`, etc.).
   - Server/client side enforcement + profile/feature-flag query API.
2. **Data/resource packs & networking:**
   - Data packs (JSON recipes, loot, tags, structures) and resource packs mounted cleanly in `assets` VFS; dev hot-reload.
   - Mod registry sync & custom namespaced mod channels; clear disconnect screen listing exact missing/mismatched mods.
3. **Examples & docs:**
   - `docs/modding/` ("Hello Block" tutorial, API reference, `content-packs.md`).
   - `examples/hello-mod/` adding a custom block + recipe + event handler.
4. **Acceptance criteria (`SPEC.md` §17):**
   - `examples/hello-mod` loads and works across integrated and LAN client/server.
   - Mismatched mod client receives readable disconnect screen; hostile infinite-loop/OOM mod test is cleanly contained by sandbox limits.
   - Tagged commit `m12-modding-v1`.

---

### M13 — World Size Presets & Persistence Polish
*Goal:* Complete fidelity for all world sizes (`classic`, `small`, `medium`, `large`, `infinite`), edge behavior, Nether/End scaling, and rock-solid autosave/backups.

1. **World size polish:**
   - Verify every preset (`Classic`, `Small`, `Medium`, `Large`, `Infinite`) end-to-end across world creation, Overworld/Nether/End generation, portal scaling, and world-edge rendering/collision.
2. **Persistence & memory management:**
   - Periodic background autosave (configurable interval, atomic write, crash-safe), manual/upgrade world backup snapshots, region file eviction & chunk cache memory limits.
   - World upgrade tooling skeleton (`DataFixer` CLI/UI flow with automatic pre-upgrade backup).
3. **Acceptance criteria (`SPEC.md` §17):**
   - All 5 presets create/save/reload cleanly.
   - `Infinite` world stress test exploring 10,000+ blocks in all cardinal directions shows bounded memory (`<= 2 GB` client) and no tick/frame hitches beyond budget.
   - Tagged commit `m13-world-sizes-persistence`.

---

### M14 — Tutorial World, Stats, Polish → v1.0
*Goal:* Complete remaining baseline-profile features, tutorial world, achievements/trophies, crash reporting, packaging, and v1.0 release readiness.

1. **Tutorial world & progression:**
   - Baseline tutorial world template/generator + guided tutorial prompts/progression matching reference.
   - Statistics tracking & LCE-style Achievements / Trophies screen (local persistence).
2. **Remaining baseline features & polish:**
   - Audit `docs/reference/baseline.md` & `feature-flags.md` for any remaining baseline-profile blocks, items, UI screens, or host options.
   - Performance profiling & optimization pass against `docs/perf.md` low/mid budgets.
3. **Reliability & packaging:**
   - Local-only panic hook writing crash report files (zero telemetry).
   - First-run `assetc` import flow when `game-assets/` is absent or user supplies an LCE dump path.
   - Windows packaging (zip/installer) & Linux build verification.
4. **Acceptance criteria (`SPEC.md` §17):**
   - Full baseline-profile play-through checklist passes (Overworld → Nether → End → Ender Dragon → splitscreen → LAN → dedicated server).
   - Zero known save-corrupting bugs; CI green.
   - Tagged commit `v1.0.0`.

---

## 5. Post-1.0 Roadmap (Tracked for Continuity)

1. **Java-style Skins:** Classic (4px) & Slim (3px) player models, `SkinProvider` trait, skin library & picker UI, handshake skin transfer/caching.
2. **LCE World Importer (`crates/tools/lce-import`):** Legacy console save format decoder using `data/legacy/blocks.toml`, entity/block-entity NBT conversion, size-preset detection.
3. **Content Packs, Auth & NAT Traversal:** LCE DLC/mash-up converter in `assetc`, `EntitlementProvider` & `IdentityProvider` launcher integrations, UPnP (`igd-next`), `RelayTransport`.
4. **Shaderpacks (Iris/OptiFine compatibility):**
   - **M-Shader-0 Spike:** Preprocess `#include`/options, lower compatibility-profile GLSL (`gl_ModelViewMatrix`, `attribute/varying`) → SPIR-V → `wgpu`/`naga` cross-compile check across Vulkan/DX12/GL/Metal.
   - Data-driven `colortex0–15`/`depthtex0–2`/`shadow`/uniforms/buffer-flipping pipeline + `block.properties` `namespace:path` → `mc_Entity` mapping.
5. **TU-by-TU Upgrade Train:** Incremental `lce:tuNN` profiles up to final Xbox One / PS4 TU, preserving all earlier profiles playable.
6. **Java Parity Track:** `java:1.x.y` profiles + `protocol-java` adapter crate implementing `ProtocolAdapter` over `ClientConnection`.

---

## 6. Cross-Session Continuity Protocol

To ensure any future session resumes with zero lost context:
1. **Start of every session:** Read `PROGRESS.md` first (then `CLAUDE.md` once created, `PLAN.md`, and relevant `docs/adr/` or `docs/reference/` files).
2. **During work:** Update `PROGRESS.md` whenever a task is completed, a `[VERIFY]`/`[DECIDE]` is resolved, a blocker appears, or before ending the session.
3. **Never leave silent state:** Record exact file paths created/modified, exact test commands & outcomes, and the single clearest next step in `PROGRESS.md`.
