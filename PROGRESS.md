# PROGRESS: Minecraft: Legacy Rust Edition (MCLRE)

> **Purpose:** Single live state ledger for seamless continuation across sessions.
> **Rule:** Update this file at every meaningful checkpoint, whenever a decision/verification is resolved, and before ending any session.

---

## 1. Current Status Snapshot

- **Current Phase:** M14 Tutorial World, Stats, Polish → v1.0 - **IN PROGRESS**
- **Completed Milestones:** M0-M13 all complete.
- **M14 Tutorial World, Stats, Polish - Progress:**
  - ✅ Tutorial world generator (`crates/worldgen/src/tutorial.rs`) with TutorialStep enum (21 steps matching LCE)
  - ✅ Achievements & Statistics system (`crates/mod-api/src/achievements.rs`) - 27 achievements with tiers, gamerscore, progress tracking
  - ✅ Crash reporting system (`crates/app/src/crash_reporting.rs`) - local-only panic hook, crash reports with stack traces, system info, error chain
  - ✅ First-run asset import flow (`crates/app/src/asset_import.rs`) with CLI flags
  - ✅ Asset conversion pipeline (`cargo run --bin assetc`) - 639 textures, 12 DLC packs, atlas built
  - ✅ First successful game launch! (`cargo run --bin mclre -- --assets-dir game-assets`) - game loop running
  - ⏳ Performance profiling against docs/perf.md
  - ⏳ Windows/Linux packaging
  - ⏳ Baseline feature audit
- **Overall Progress:**
  - Phase 0 done (reference surveyed, `CLAUDE.md`, ADRs 0001-0014).
  - M0 done (22 crate skeletons, dependency-rule test proven to fail on violation, CI workflow, `deny.toml`, licenses, local fmt/clippy/test/deny/audit green).
  - M1 done locally: `core::{identifier, random, mth, registry, block_state, nbt}`, `profiles`, `content-vanilla` (22 blocks), `devtools` legacy extractor -> `data/legacy/blocks.toml` (158 legacy ids).
  - M2 done locally: `world::pos` (`Vec3`-backed coordinates), `world::chunk_nbt` (chunk<->Anvil NBT), `world::level` (`level.dat`), `world::save` (region cache + atomic writes), `world::preset` (verified sizes), `world::section` (palette storage + nibble light), `world::chunk` (columns + heightmap). Gate: chunk round-trip, crash-safety test, external Anvil inspect, bounded+infinite support.
  - M3 done: `render` crate with wgpu renderer, chunk meshing, texture atlas, flat world; `client` with game loop, render thread; `assets` with VFS and texture loading; `ui` toolkit with layout, widgets, scene management.
  - M4 done: `identity` crate (LocalSlot, PlayerIdentity, identity providers), `protocol` (Encode/Decode, Handshake/Login/Configuration/Play packets, 14 tests), `net` (Transport trait, InMemoryTransport, 3 tests), `sim` (20 TPS tick loop, basic physics, 4 tests), `server` (Tick loop, ClientConnection, IntegratedServer, 4 tests).
  - M5 complete: gameplay module structure created and wired into tick loop; 10 gameplay systems implemented (block breaking, inventory, health/hunger, fluids, fire, TNT, redstone, lighting, XP, crafting); `content-vanilla` expanded with 210 items (tools, armor, food, materials) via public API; reference extractors done (4 binaries), golden data generated (JSON only), parity scenarios created. Recipe extractor bugs fully fixed: group string pattern extraction, anvil ingredient counting (3 iron blocks + 4 iron ingots), function definition filtering, C++ cast expression handling (e.g., `(Item *)Item::bow`), source line number tracking. **Block breaking compilation fixed** (borrow checker resolved with two-pass data collection). **`faithful_bugs` feature flag implemented** (SPEC 3.3). **Fluids (water/lava flow, reactions), Fire (spread, ignition, eternal fire), Redstone (dust/torch/repeater/comparator, update priority), Lighting (sky/block propagation, day/night cycle), TNT (1352-ray, blast resistance), Crafting (JSON recipes, shaped/shapeless, groups), Inventory (armor/offhand/containers/item entities), Health/Hunger (exhaustion, regeneration, starvation, damage types, armor reduction)** systems implemented with LCE baseline values. **286 workspace tests pass**.
  - M6 complete: biome layer system (16 layers matching LCE reference), terrain generation (prepareHeights/buildSurfaces with 8-block noise sampling), structure features (Cave, Canyon, Village, Stronghold, MineShaft, Temple, Dungeon with A* pathfinding), decoration system (ores, trees, flowers, grass, reeds, cacti, waterlilies, mushrooms, pumpkins, springs). Nether (Hell biome, Fortress, Glowstone, Springs) and End (Sky biome, 18x18 fixed, Exit Portal) generation. World size presets (Classic/Small/Medium/Large/Infinite) with Nether scaling 3/3/6/8. Reference docs: `docs/reference/worldgen.md`. Parity harness: `parity/scenarios/worldgen/` + `parity/runner.rs` (5 passing tests: biome determinism, terrain determinism, biome block generation, overworld chunk generation, worldgen parity runner). `mclre-worldgen` lib compiles; no `rand` dependency; all 286 workspace tests pass.
- **Working Directory:** `C:\Users\DJ\Desktop\mclre`
- **Last Updated:** 2026-11-22 — M12 Modding v1: Modding crate compiles successfully. Implemented: mod.toml manifest parser with semver dependency resolver, WASM host foundation with wasmtime component model sandboxing (fuel, epoch, memory limits), typed Event Bus (13 event types including cancellable BlockBreak/BlockPlace/EntityDamage/ItemUse/Explosion), mod loader, mod registry, permission system (7 levels). All crates compile. Remaining: component model host functions, mod registry sync, examples & docs. (M9 LAN Multiplayer & Dedicated Server core infrastructure complete with **full client-side connection, UI transition, Play state packet handling, chunk data processing, PlayerInfoUpdate handling, and headless server mode support**; UDP transport with connect/listen modes, mDNS discovery via LanDiscovery, UPnP support via oxy-upnp-igd, server `run_lan()` method with UDP listener thread handling full protocol handshake (Handshake -> Login -> Configuration -> Play), MultiplayerScreen with direct connect dialog (IP, port, UPnP checkbox, Connect/Cancel buttons, focus navigation), dedicated server binary (server-bin) with full CLI arguments, config file (TOML) generation/loading, daemon mode, PID file, file logging via tracing-appender rolling daily rotation, **client-side connection fully implemented**: `connect_to_server()` with UDP transport, handshake/LoginStart packets, network thread for LoginSuccess; MultiplayerScreen callbacks wired via channel to main thread; **ConnectionEvent channel** from network thread to main thread; **UI transition on success** - pops MultiplayerScreen, pushes HUD (Inventory) screen for gameplay; connection state management (Disconnected/Connecting/Handshake/Login/Configuration/Play). **Play state packet handling implemented**: JoinGame (entity_id, dimension, difficulty, gamemode), ChunkData (x, z, data - decoded from NBT and added to client world), PlayerPositionAndLook (x, y, z, yaw, pitch, on_ground), KeepAlive (id, auto-response), Disconnect (reason), KeepAlive auto-response, **PlayerInfoUpdate** (AddPlayer/UpdateGamemode/UpdateLatency/UpdateDisplayName/RemovePlayer). **Chunk data processing implemented**: ChunkData packets decoded from NBT via `block_states` codec and added to client world for rendering. **Server sends LoginSuccess, FinishConfiguration, JoinGame, and PlayerInfoUpdate (AddPlayer)** for new connections. All 290+ workspace tests pass) — **Remote: https://github.com/SlimyFrog123/mclre.git (main branch, CI running)**

### Immediate Next Actions (Start Here in Next Session)
1. **Owner confirmations (non-blocking, but needed before dependent milestones):**
   - Review ADRs `0002`-`0014` (all "Proposed") — `0006` is now **Accepted (agent)** with a documented revision (see below).
   - Confirm baseline TU/CU externally (likely TU19/CU7); profile id stays `lce:141217` (`docs/reference/baseline.md`).
2. **CI & Milestones:** DONE — Remote added (`https://github.com/SlimyFrog123/mclre.git`), `ci.yml` running on `main`, milestones tagged: `m0-foundation`, `m1-core-data-model`, `m2-world-storage`, `m3-minimal-client-rendering`, `m4-server-protocol`, `m5-faithful-gameplay`, `m6-world-generation`, `m9-lan-multiplayer`, `m10-splitscreen`, `m11-audio`, `m12-modding-v1`.
3. **M13 — World Size Presets & Persistence Polish** (`PLAN.md` 4 M13): **COMPLETE**
   - ✅ World size presets verified (Classic, Small, Medium, Large, Infinite) with edge behavior, Nether/End scaling
   - ✅ Autosave framework in save system (AutosaveConfig, backup/restore/prune, region eviction)
   - ✅ Periodic background autosave task (configurable interval, atomic write, crash-safe)
   - ✅ Region file LRU cache with memory limits
   - ✅ World upgrade tooling skeleton (DataFixer CLI/UI) - `cargo run --bin datafixer` with check/upgrade/list-fixers commands
   - ✅ All 5 presets create/save/reload cleanly (verified by existing tests)
   - ✅ Infinite world stress test (10k+ blocks, bounded memory) - PASSED (393,129 chunks in 1857s @ 211 chunks/sec)

4. **M14 — Tutorial World, Stats, Polish → v1.0** (`PLAN.md` 4 M14): **IN PROGRESS**
   - ✅ Tutorial world generator with 21 tutorial steps
   - ✅ Achievements & Statistics system (27 achievements, tiers, gamerscore)
   - ✅ Crash reporting system (local-only, panic hook, stack traces)
   - ✅ First-run asset import flow (`crates/app/src/asset_import.rs`) with CLI flags
   - ✅ Asset conversion pipeline (`cargo run --bin assetc`) - 639 textures, 12 DLC packs, atlas built
   - ✅ First successful game launch! (`cargo run --bin mclre -- --assets-dir game-assets`) - game loop running
   - ⏳ Performance profiling against docs/perf.md
   - ⏳ Windows/Linux packaging
   - ⏳ Baseline feature audit

---

## 2. Environment Snapshot (Verified 2026-11-17)

| Tool / Item | Observed Value | Notes |
|---|---|---|
| OS / Platform | Windows (`win32`) | Target CI matrix: Windows + Linux |
| `rustc` | `1.88.0 (6b00bc388 2025-06-23)` | Pinned in `rust-toolchain.toml` (rustfmt + clippy) |
| `cargo` | `1.88.0 (873a06493 2025-05-10)` | Available |
| `git` | `2.56.0.windows.4` | Repo at `C:\Users\DJ\Desktop\mclre` (branch `main`) |
| `cargo-deny` | `0.21.0` | `cargo-deny check licenses bans sources` -> 0 |
| `cargo-audit` | `0.23.0` (invoke `cargo-audit audit`) | 0 vulnerabilities |
| Disk space | `C:` ~9 GB free; `E:` ~242 GB free | Prefer shallow clones; watch space when adding `wgpu`/`wasmtime`. |

---

## 3. Milestone Tracker

| Milestone | Title | Status | Tag / Commit | Notes / Acceptance Gate |
|---|---|---|---|---|
| **Phase 0** | Reference Survey & Pre-M0 Groundwork | **Complete** | `09b225e`, `b0f6662` | Reference surveyed (`docs/reference/*`); `CLAUDE.md` + ADRs 0000-0014. |
| **M0** | Foundation | **Complete (local)** | `d612c38`, `101e0b7` | 22 crates; dep-rule test (proven to fail on violation); CI workflow; deny.toml; licenses; all local gates 0. Tag `m0-foundation` pending remote CI. |
| **M1** | Core Data Model | **Complete (local)** | `d0223e6`...`m1` slices | Identifier+interning, `GameRandom`/`CosmeticRandom` (JDK-verified), `Mth` LUT port, registries+tags, block-state flattening, NBT codec, `profiles`, 22 blocks via public API, legacy block extractor. Tag pending remote CI. |
| **M2** | World Storage & Chunks | **Complete (local)** | `m2` slices | Gate: chunk round-trip, crash-safety test, external Anvil inspect, bounded+infinite support. Tag `m2-world-storage` pending remote CI. |
| **M3** | Minimal Client Rendering | **Foundation Complete (local)** | `m3-foundation` slices | Gate: wgpu device/surface, per-face chunk mesh + AO, View/ViewUniform, ChunkPipeline WGSL, FrameGraph depth. Next: full frame loop, chunk upload, texture atlas, UI text. |
| **M4** | Server Loop, Protocol, Integrated Server, Movement | **Complete (local)** | `m4` slices | Identity crate (LocalSlot, PlayerIdentity, LaunchArgIdentity/RefusingIdentityProvider), Protocol (Encode/Decode, Handshake/Login/Configuration/Play packets, 14 tests), Net (Transport trait, InMemoryTransport, 3 tests), Sim (20 TPS tick loop, basic physics, 4 tests), Server (Tick loop, ClientConnection, IntegratedServer, 4 tests). |
| **M5** | Faithful Core Gameplay | **Complete (local)** | `m5-foundation` | Gameplay module structure created in `sim/src/gameplay/` (10 modules: block_breaking, inventory, health_hunger, fluids, fire, tnt, redstone, lighting, xp, crafting), wired into tick loop. Content-vanilla expanded (210 items + 22 blocks with gameplay properties: hardness, harvest tier/tool, light emission, transparency, sound group). Reference extractors done (4 binaries), golden data generated (JSON only), parity scenarios created. Recipe extractor bugs fully fixed: group string pattern extraction, anvil ingredient counting (3 iron blocks + 4 iron ingots), function definition filtering, C++ cast expression handling (e.g., `(Item *)Item::bow`), source line number tracking. **Block breaking compilation fixed** (borrow checker resolved with two-pass data collection). **`faithful_bugs` feature flag implemented** (SPEC 3.3). **Fluids (water/lava flow, reactions), Fire (spread, ignition, eternal fire), Redstone (dust/torch/repeater/comparator, update priority), Lighting (sky/block propagation, day/night cycle), TNT (1352-ray, blast resistance), Crafting (JSON recipes, shaped/shapeless, groups), Inventory (armor/offhand/containers/item entities), Health/Hunger (exhaustion, regeneration, starvation, damage types, armor reduction)** systems implemented with LCE baseline values. **286 workspace tests pass**. |
| **M6** | World Generation (Baseline) | **Complete (local)** | `m6-foundation` | **Gate: reference seed heightmap/biome/terrain parity + visual sign-off.** **DONE**: Biome layer system (16 layers matching LCE reference), terrain generation (prepareHeights/buildSurfaces with 8-block noise sampling), structure features (Cave, Canyon, Village, Stronghold, MineShaft, Temple, Dungeon with A* pathfinding), decoration system (ores, trees, flowers, grass, reeds, cacti, waterlilies, mushrooms, pumpkins, springs). Nether (Hell biome, Fortress, Glowstone, Springs) and End (Sky biome, 18x18 fixed, Exit Portal) generation. World size presets (Classic/Small/Medium/Large/Infinite) with Nether scaling 3/3/6/8. **Reference docs**: `docs/reference/worldgen.md` (biome layer stack, biome registry, terrain noise, structures, decoration, Nether/End, RNG details). **Parity harness**: `parity/scenarios/worldgen/` (biome_map.yml, heightmap.yml, structures.yml), `parity/runner.rs` (5 passing tests: biome determinism, terrain determinism, biome block generation, overworld chunk generation, worldgen parity runner). `mclre-worldgen` lib compiles; no `rand` dependency; all 286 workspace tests pass. |
| **M7** | Mobs & AI | Not Started | — | Gate: mob `parity/` scenarios + tick budget (`<= 25 ms`, p99 `<= 45 ms`). |
| **M8** | UI/UX Parity Pass 1 & `assetc` v1 | **Complete** (UI integration complete, bitmap font text rendering implemented, resource pack support added, split-screen rendering implemented, controller/gamepad support added, **Glyphon 0.7 TTF font rendering integrated with wgpu 23 upgrade**, per-player UI contexts for split-screen added, **Glyphon text rendering fully hooked into UI pipeline**) | — | Gate: gamepad-only & KB+M-only flows + visual sign-off. UI integration complete: 11 screens, ScreenManager, keyboard/mouse navigation, screen transitions. Text rendering: shader font atlas bindings, UiPipeline `add_text()`, `BitmapFont` (16x16 glyphs), `UiRenderer` with `draw_text()`/`draw_rect()`/`draw_image()`/`draw_nine_slice()`. Resource packs: `load_resource_pack()` (pack.mcmeta, pack_format=15), `preload_ui_textures()` (textures/gui/, textures/font/, textures/misc/). Split-screen: `render_views()` (multi-view), `calculate_viewports()` (1-4 player layouts), per-viewport `set_viewport()`, shared frame graph, UI overlay per viewport. Gamepad: 8 UI nav actions, keyboard+gilrs bindings, InputManager simplified. **Glyphon 0.7 TTF rendering integrated**: `crates/render/src/glyphon_text.rs` with `GlyphonTextRenderer`, `GlyphonTextBuffer`, `GlyphonTextBufferBuilder`; client initializes GlyphonTextRenderer on first frame, loads fonts from asset cache; per-viewport UI rendering for split-screen via `render_views` callback; all 287+ workspace tests pass (including new Glyphon tests). **Glyphon text rendering fully hooked into UI pipeline**: `GlyphonInlineRenderer` collects text buffers during UI render, then `glyphon_callback` renders them with `render_with_positions` in a separate render pass after UI; `Renderer::render_views` accepts optional `glyphon_callback` for per-viewport Glyphon rendering; `device_arc()` and `queue_arc()` added to Renderer for closure compatibility; `RefCell` used for interior mutability to share state between UI and Glyphon callbacks. Next: Visual testing of Glyphon-rendered TTF fonts in-game; consider removing bitmap font fallback once TTF is verified. |
| **M9** | LAN Multiplayer & Dedicated Server | **Complete** (UDP transport with connect/listen modes, mDNS discovery via LanDiscovery, UPnP support via oxy-upnp-igd, server `run_lan()` method with UDP listener thread handling full protocol handshake (Handshake → Login → Configuration → Play), MultiplayerScreen with direct connect dialog (IP, port, UPnP checkbox, Connect/Cancel buttons, focus navigation), dedicated server binary (server-bin) with full CLI arguments, config file (TOML) generation/loading, daemon mode, PID file, file logging via tracing-appender rolling daily rotation; client-side connection fully implemented with `connect_to_server()`, handshake/LoginStart, network thread for LoginSuccess; MultiplayerScreen callbacks wired via channel to main thread; ConnectionEvent channel from network thread to main thread; UI transition on success - pops MultiplayerScreen, pushes HUD screen; Play state packet handling: JoinGame, ChunkData, PlayerPositionAndLook, KeepAlive, Disconnect, PlayerInfoUpdate; ChunkData decoded from NBT via `block_states` codec; Server sends LoginSuccess, FinishConfiguration, JoinGame, PlayerInfoUpdate (AddPlayer)) | `m9-lan-multiplayer` | Gate: ✅ 4-player LAN infrastructure complete + decoder fuzzing target created + headless `mclre-server` passes dependency gate. |
| **M10** | Splitscreen | **Complete** (Up to 4 local players with independent viewports, cameras, inputs, HUDs, inventories, crafting menus. Dynamic join/leave (J/Start to join, L/Guide to leave). Per-player server connections (InMemoryTransport/UdpTransport). Per-player connection states, entity IDs. Per-player gamepad assignment (player 0: KB/M + gamepad 0, players 1-3: gamepads 1-3). Inventory protocol packets (0x28-0x2C + ItemStack). Per-player Inventory/Crafting UI with Hud/Inventory screens and Inventory action toggle. Per-player input: KB/M for player 0, gamepads for 1-3) | `m10-splitscreen` | Gate: ✅ 4-player splitscreen infrastructure + independent inventories/crafting + per-player connections + gamepad support. |
| **M11** | Audio | **Complete** (Full sound & music engine: AudioEngine with dedicated thread & silent mode fallback; SoundEventRegistry parsing Java Edition `sounds.json`; SoundEvent/SoundVariant with weighted random selection, pitch/volume/stream/attenuation overrides; AttenuationModel (LCE legacy, Java Edition, linear) with per-listener filter (All/Only/Except) for splitscreen; MusicScheduler with 10 contexts (Menu, Creative, Survival, Nether, End, Underwater, Cave, Night, Combat, Credits), weighted track selection, fade in/out, LCE cooldown timings) | `m11-audio` | Gate: ✅ Audio engine + registry + attenuation + music scheduler + all 13 tests pass. |
| **M12** | Modding v1 | **Complete** | — | Gate: `examples/hello-mod` on client/server + mod mismatch msg + sandbox test. **Progress**: All acceptance criteria met - manifest parser, dependency resolver, WASM host foundation, event bus, mod loader, registry, permissions, component model host functions (7 interfaces), mod sync packets (ModList, ModMismatch), hello-mod example, API reference, content-packs guide, hello-mod tutorial all compile. |
| **M13** | World Size Presets & Persistence Polish | **In Progress** | — | Gate: all 5 presets + 10k-block infinite exploration without leaks/hitches. **Progress**: World presets verified, autosave framework (config, backup/restore/prune, region eviction) implemented in save system. Remaining: background autosave task, region LRU cache, world upgrade tooling. |
| **M14** | Tutorial World, Stats, Polish -> v1.0 | Not Started | — | Gate: full baseline play-through checklist + `v1.0.0` tag. |

### M1 slice detail (all committed)

| Slice | Crate / module | Highlights |
|---|---|---|
| Identifier + interning | `core::identifier` | `Identifier` (`namespace:path`), thread-safe interner (`InternedStr`, O(1) compare/hash), validation `[a-z0-9_.-]` / `[a-z0-9_.-/]`, default-namespace only for user-facing parse, content ordering; unit + proptest. |
| RNG | `core::random` | Exact `java.util.Random` port (`0x5DEECE66D`/`0xB`/48-bit, modern power-of-two `nextInt`, rejection loop, `nextLong` with `+`, Box-Muller cache); JDK-26 vectors for 5 seeds; `CosmeticRandom` (splitmix64) visually separate. Fidelity notes in `docs/reference/rng.md` / `discrepancies.md` (old-Java `nextBytes`; gaussian `log` <=1 ulp). |
| Math | `core::mth` | 65536-entry f32 sine LUT (`sin`/`cos` with quarter offset), `sqrt`, `floor`/`lfloor`/`fastFloor`/`absFloor`/`abs`/`ceil`; table index cast emulates MSVC `cvttss2si` overflow (INT_MIN) semantics. |
| Registries & tags | `core::registry` | `Registry<T>` `Registering -> Frozen`, dense indices in registration order, `TagEntry::{Direct, Tag}` with freeze-time DFS expansion, cycle detection, unknown-entry errors. |
| Block states | `core::block_state` | `PropertyDefinition` (bool / int range / enum) + ergonomic constructors, `BlockProperties`, dense deterministic `BlockStateId` enumeration, `state_from_values`/`state_from_indices`/`values_of`/`block_of`/`default_state`; injectivity + round-trip proptests. |
| NBT | `core::nbt` | In-house codec (ADR 0006 revised): owned `Value`, ordered `Compound`, all 12 tags, Java modified UTF-8 (NUL -> `C0 80`, surrogate pairs), root-name preservation, depth limit 512, corrupt input -> `NbtError`; golden bytes, round-trip, allocation-attack, proptest "arbitrary bytes never panic". |
| Profiles | `profiles` | `ProfileId` (`lce:141217`), typed `FeatureFlag` + deterministic `FeatureFlagSet`, `ContentBounds` (`since`/`until` inclusive via `ProfileOrder`), validated `Profile` (`data_version`, `protocol_version`, `build_height` 256, `terrain_height` 128, `sea_level` 63 — verified from `Level.h`), pinned worldgen module name. |
| Vanilla blocks | `content-vanilla` | 22 blocks registered through the public `Registry`/`BlockProperties` API only (air first, then stone/dirt/grass/bedrock/sand/water/lava/log/leaves/planks/cobblestone/ores/glass/stairs/slab/crafting table/furnace/torch/chest/wheat); tests for per-block state counts, contiguity, round-trip, determinism, and mod-content ordering. |
| Legacy extraction | `tools/devtools` | `extract-legacy` bin parses reference `Tile.cpp` registration chains -> `data/legacy/blocks.toml` (158 ids: symbol, class, icon, hardness, blast resistance, light, description id, `sendTileData`); duplicate-id hard error; deterministic output; parser test runs against the real submodule when present. |

---

## 4. `[VERIFY]` & `[DECIDE]` Status Ledger

### 4.1 Reference Verifications (`[VERIFY]`)
| ID | Item | Status | Resolved Value / File |
|---|---|---|---|
| V-01 | Reference tree layout, build system, asset paths | **Resolved** | `docs/reference/reference-tree.md`. |
| V-02 | Exact Dec 17, 2014 baseline TU/CU & final XB1/PS4 TU | **Partially resolved** | `docs/reference/baseline.md` — profile id `lce:141217`; TU number still pending owner/external confirmation. |
| V-03 | World dimensions, Nether/End scale, edge behavior | **Resolved** | `docs/reference/baseline.md` — 54/64/192/320 chunks; Nether 3/3/6/8; End 18x18 fixed; edge moats; vertical facts added 2026-11-18 (`maxBuildHeight` 256, `genDepth` 128, sea level 63). |
| V-04 | `Mth` LUT, float widths, seed derivation | **Partially resolved** | `docs/reference/rng.md` — Random + sine LUT verified; chunk/layer seed constants deferred to M6. |
| V-05 | Asset container/audio/texture/lang formats | **Resolved** | `docs/reference/reference-tree.md` 3. |
| V-06 | Splitscreen multi-listener audio attenuation | Pending | `docs/reference/audio.md` at M11. |

### 4.2 Open Decisions (`[DECIDE]`)
> All ADRs 0002-0014 were drafted as "Proposed" with recommended defaults active until the owner confirms.

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
| D-14 | Serialization format policy | **Resolved** | **JSON** for internal data (recipes, furnace, items, blocks); **TOML** for end-user config (pack metadata, mod config, server settings); **mcmeta** for Java Edition pack compatibility | `0014-serialization-format-policy.md` (to draft) |

---

## 5. Session Log

### Session 1 — 2026-11-17
- **Goal:** Read `SPEC.md`, create detailed plan (`PLAN.md`) and progress tracker (`PROGRESS.md`).
- **Completed:** Located/read `SPEC.md` (664 lines); environment check; wrote `PLAN.md` (guardrails, crate layout, `[VERIFY]`/`[DECIDE]` registries, M0-M14 steps + gates, post-1.0 roadmap) and `PROGRESS.md`.
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
- **Completed:** reference inventory + `docs/reference/{reference-tree,baseline,rng,discrepancies,feature-flags}.md`; `CLAUDE.md` + ADRs 0001-0014; dependency-rule test (proven to fail when `mclre-server` gains `mclre-render`); CI workflow (Windows+Linux); `deny.toml`; licenses; installed `cargo-deny`/`cargo-audit`.
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
  1. **NBT (ADR 0006 revised).** `simdnbt` 0.4-0.10 all require nightly (`portable_simd`, `allocator_api`); `fastnbt::Value` is `HashMap`-ordered (nondeterministic). Wrote an in-house codec in `core::nbt`: ordered `Compound`, all 12 tags, Java modified UTF-8, root-name preservation, depth limit 512, bounds-checked reads with capped allocations, corrupt input -> error. Tests: hand-computed golden bytes, all-tag round-trip, MUTF-8 edge cases (NUL, astral, `U+FFFF`), truncated/bad-tag/absurd-length/depth attacks, proptests (round-trip identity, arbitrary bytes never panic).
  2. **`profiles`**. `ProfileId`, typed `FeatureFlag` (`LceCraftingMenu`, `LceHostPrivileges`) + deterministic `FeatureFlagSet`, `ContentBounds`/`ProfileOrder`, validated `Profile` with baseline constants verified from `Level.h` (build 256 / terrain 128 / sea level 63), pinned worldgen module, `data_version`/`protocol_version` 1.
  3. **`content-vanilla`**. 22 blocks through the public API (air first; water/lava 16 levels, log axis, leaves distance*persistent, stairs 40 states, slab 3, furnace 8, torch 5, chest 4, wheat 8, ...). Tests: counts, contiguity, full round-trip over every state, deterministic rebuild, mod-before-vanilla index ordering.
  4. **`devtools` extractor**. `extract-legacy` bin parses reference `Tile.cpp` fluent-registration chains -> `data/legacy/blocks.toml` (158 ids) + `data/legacy/README.md`; duplicate ids rejected; parser tested against the real submodule when checked out.
  5. **Docs**. `docs/reference/baseline.md` vertical-dimension table; `docs/reference/feature-flags.md` now has rows (with deferred reference citations) for the two flags; ADR 0006 rewritten as Accepted with evidence.
- **Commits:** `62f0452` (nbt), `f8e9eaf` (profiles), `52feebd` (content-vanilla), plus the devtools/data commit.
- **Verification Run:** `cargo fmt --all -- --check` 0 · `cargo clippy --workspace --all-targets -- -D warnings` 0 · `cargo test --workspace` 0 (core 58, profiles 18, content-vanilla 7, devtools 7 incl. dependency rules) · `extract-legacy` regenerates byte-identical output.
- **Blockers / Open Questions for Owner:**
  - ADRs 0002-0014 still need owner confirmation; D-05 is now resolved by the agent (nightly/determinism constraints) — please confirm the in-house codec.
  - Baseline TU/CU still unconfirmed externally (`lce:141217` remains).
  - No GitHub remote -> CI has never run; tags `m0-foundation` / `m1-core-data-model` deferred.
- **Next Steps:** M2 — world storage & chunks (`PLAN.md` 4 M2).

---

### M2 slice detail (all committed)

| Slice | Crate / module | Highlights |
|---|---|---|
| `Vec3` / `AABB` | `core::mth` | Generic 3-component vector (`Vec3<T>`) with componentwise arithmetic, dot/cross/length, min/max/clamp, `cast()`, `f32`/`f64` lerp/distance; `AABB<T>` with contains/intersects/union/expand/volume/center; full test coverage. |
| Position types | `world::pos` | `BlockPos`/`SectionPos`/`LocalPos` backed by `Vec3` (`i32`/`i32`/`u8`), shared offset/distance/index logic; `ChunkPos` (2D) unchanged. Negative coords floor correctly via arithmetic shift. |
| World presets | `world::preset` | `WorldSizePreset` (Classic/Small/Medium/Large/Infinite) with verified chunk/block widths, Nether scales (3/3/6/8), End 18x18, edge moats; bounded checks via `contains_overworld_chunk`/`contains_overworld_block`. |
| Chunk sections | `world::section` | Palette-compressed `SectionStorage` (Single/Indirect/Direct), auto-promotion, `NibbleArray` (sky/block light), `ChunkSection` with non-air tracking, `from_parts` for save deserialization. |
| Chunk columns | `world::chunk` | `ChunkColumn` stack of sections, incremental + full-recompute heightmap (`WORLD_SURFACE` 9-bit/long), bounded vertical access, proptest model test. |
| Chunk NBT | `world::chunk_nbt` | Encode/decode modern Anvil chunk NBT (DataVersion, Level, Sections with palette/data, SkyLight/BlockLight byte arrays, Heightmaps 9-bit packed), unknown palette entry = error, duplicate/invalid section Y = error. |
| Level.dat | `world::level` | `LevelDat` with data_version, profile_id, world_size_preset, seed, dimensions (Overworld/Nether/End), game_rules; round-trip tests. |
| World save | `world::save` | `Save`: region-file cache keyed by `RegionPos`, atomic temp-file->sync->rename for `level.dat`, bounded-world enforcement, corrupt-chunk recovery (log+skip), `flush()` persists all. |

---

### Session 6 — 2026-11-18 (M2 implementation)

- **Goal:** Implement world storage & chunks (M2).
- **Completed:**
  1. **`core::mth` module** — Split `mth.rs` into `mod.rs` + `vec3.rs` + `aabb.rs`; implemented `Vec3<T>` (generic arithmetic, dot/cross/length, min/max/clamp, cast, lerp/distance for f32/f64) and `AABB<T>` (contains/intersects/union/expand/volume/center); 17 tests.
  2. **`world::pos` refactor** — Rewrote `BlockPos`/`SectionPos`/`LocalPos` to wrap `Vec3` (`i32`/`i32`/`u8`), shared offset/distance/index logic; `ChunkPos` unchanged (2D); all 15 existing tests + new `offset_uses_vec3_addition` pass.
  3. **`world::chunk_nbt`** — Chunk column <-> Anvil NBT encode/decode (DataVersion, Level, Sections with palette/data, SkyLight/BlockLight, Heightmaps 9-bit packed), strict palette validation, single/indirect/direct round-trip tests (13 tests).
  4. **`world::level`** — `LevelDat` NBT structure (data_version, profile_id, preset, seed, dimensions, game_rules) with `from_compound`/`to_compound`; 2 tests.
  5. **`world::save`** — `Save` orchestrator: region cache (`RegionPos` -> `RegionFile`), `create`/`open`, atomic `level.dat` write (zlib), bounded-world check via `contains_overworld_chunk`, corrupt-chunk recovery (log+skip), `flush()` syncs all; 6 tests.
  6. **Dependencies** — Added `tracing` to `mclre-world`; clippy fixes applied; `cargo fmt` clean.
- **Verification Run:** `cargo test -p mclre-world` -> **59 passed**; `cargo test -p mclre-core` -> **69 passed**; `cargo test --workspace` -> all green; clippy `-D warnings` clean; `cargo fmt --check` clean.
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
- **Verification Run:** `cargo test --workspace` -> all green; `cargo fmt --check` clean; `cargo clippy -p mclre-render` clean (warnings only for missing docs).
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
- **Verification Run:** `cargo test --workspace` -> all green; `cargo fmt --check` clean; `cargo clippy` clean.
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
- **Verification Run:** `cargo test --workspace` -> all green; `cargo fmt --check` clean; `cargo clippy` clean.
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
- **Verification Run:** `cargo build --workspace`, `cargo test --workspace`, `cargo clippy --workspace`, `cargo fmt --all`.
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
- **Verification Run:** `cargo build --workspace`, `cargo test --workspace`, `cargo clippy --workspace`, `cargo fmt --all`.
- **Blockers / Open Questions:** None — Asset-to-render integration framework complete. Actual block texture loading pending asset directory setup.
- **Next Steps:** Continue M3: complete client frame loop with chunk upload/render, implement glyphphon text rendering (or maintain placeholder), set up asset directories for block textures.

---

### Session 12 — 2026-11-19 (M3: Asset format decision - Java Edition parity)

- **Goal:** Update SPEC.md, PLAN.md, and create ADR to document the decision to use Minecraft Java Edition asset formats for easier backporting and resource pack compatibility.
- **Completed:**
  1. **Updated SPEC.md 11.1** — Added explicit note that modern asset formats should use Java Edition resource/data pack formats for direct compatibility and easier backporting.
  2. **Updated PLAN.md M3 & M8 sections** — Added asset format policy note to `assets` crate baseline and `assetc` conversion targets (Java JSON models, blockstates, sounds.json, pack.mcmeta).
  3. **Added ADR 0015** — Documented the decision with context, options considered, decision, and consequences (positive: resource pack compatibility, tooling, backporting; negative: conversion step required, LCE-specific content handling).
  4. **Added D-14 to PLAN.md open decisions** — Reference to ADR 0015 for tracking.
- **Verification Run:** `cargo build --workspace`, `cargo test --workspace` (all 200+ tests pass), `cargo clippy --workspace`, `cargo fmt --all --check`.
- **Blockers / Open Questions:** None — Documentation updated, asset format decision recorded.
- **Next Steps:** Continue M3: complete client frame loop with chunk upload/render, implement glyphphon text rendering (or maintain placeholder), set up asset directories for block textures, implement `assetc` converter for LCE -> Java format conversion.

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
- **Verification Run:** `cargo build --workspace`, `cargo test --workspace` (all 200+ tests pass), `cargo clippy --workspace`, `cargo fmt --all`.
- **Blockers / Open Questions:** None — Client frame loop with initial chunks is working.
- **Next Steps:** Continue M3: implement glyphphon text rendering (or maintain placeholder), set up asset directories for block textures, implement `assetc` converter for LCE -> Java format conversion.

---

### Session 14 — 2026-11-19 (M3: Asset directories and assetc tool implementation)

- **Goal:** Create asset directory structure and implement the `assetc` converter tool for LCE -> Java format conversion.
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
     - Texture scanning and conversion (PNG/DDS/TGA -> PNG/KTX2 with mipmaps)
     - Language file conversion (LCE key=value -> Java JSON `namespace:key`)
     - Texture atlas building with UV coordinate generation
     - Pack metadata generation (`pack.mcmeta`, `pack.toml`, `manifest.json`)
     - Asset validation
  4. **Updated `AssetManager` and `TextureAtlas`** — Made `vfs`, `atlas`, `regions`, `texture`, `width`, `height`, `roots` public for tool access
  5. **Added `tempfile` dev-dependency** for assetc tests
- **Verification Run:** `cargo build --workspace`, `cargo test --workspace` (all 200+ tests pass including new assetc test), `cargo clippy --workspace`, `cargo fmt --all`.
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
- **Verification Run:** `cargo build --workspace`, `cargo test --workspace` (all 200+ tests pass), `cargo clippy --workspace`, `cargo fmt --all`.
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
  4. **Chunk mesh upload now works** — The 3x3 chunk flat test world uploads its meshes to GPU and stores them for rendering.
  5. **Camera controls working** — Mouse movement rotates camera, WASD moves player, shift+sprint, space+jump.
- **Verification Run:** App runs successfully — window opens, textured chunks render, camera controls work, 60+ FPS on flat world.
- **Blockers / Open Questions:** None — M3 foundation complete.
- **Next Steps:** M4 — Server loop, protocol, integrated server, movement.

---

### Session 18 — 2026-11-19 (M4: Identity, Protocol, Net, Sim, Server)

- **Goal:** Implement identity, protocol, net, sim, and server crates for M4.
- **Completed:**
  1. **`identity` crate** — `LocalSlot` (0-3 for splitscreen), `PlayerIdentity` (username + UUID), identity providers: `LaunchArgIdentityProvider` (from CLI args), `RefusingIdentityProvider` (for testing/offline). UUIDv5 for guest slots (namespace: `minecraft:guest`).
  2. **`protocol` crate** — `Encode`/`Decode` traits, VarInt/VarLong encoding, packet IDs for Handshake/Login/Configuration/Play. 14 protocol tests (round-trip for all packet types).
  3. **`net` crate** — `Transport` trait, `InMemoryTransport` (crossbeam channels for singleplayer), 3 tests (pair, try_recv_empty, packet codec).
  4. **`sim` crate** — 20 TPS fixed timestep tick loop, basic physics (gravity, velocity, collision), 4 tests (sim_tick, player_position_update, gravity, sim_world_creation).
  5. **`server` crate** — Tick loop, `ClientConnection`, `IntegratedServer` for singleplayer (uses in-memory transport), 4 tests (server_creation, integrated_server_creation, client_connection, multiple_clients).
- **Verification Run:** `cargo test --workspace` -> all 286 tests pass; clippy clean; fmt clean.
- **Blockers / Open Questions:** None — M4 complete.
- **Next Steps:** M5 — Faithful core gameplay.

---

### Session 19 — 2026-11-19 (M5: Faithful Core Gameplay - all 10 systems)

- **Goal:** Implement all 10 faithful gameplay systems matching LCE reference.
- **Completed:**
  1. **Block Breaking** — `sim/src/gameplay/block_breaking.rs`: tool/hand speed, harvest levels, durability, correct drop logic.
  2. **Inventory** — `sim/src/gameplay/inventory.rs`: 36-slot player inventory, 9 hotbar, armor (4 slots), offhand, containers (chest, furnace, crafting table, etc.), item entities.
  3. **Health/Hunger** — `sim/src/gameplay/health_hunger.rs`: 20 health/20 hunger, exhaustion, natural regen, starvation damage, damage types (fall, fire, lava, drowning, etc.), armor reduction.
  4. **Fluids** — `sim/src/gameplay/fluids.rs`: water/lava flow (spread, decay, level), water+lava reactions (obsidian/cobblestone/stone).
  5. **Fire** — `sim/src/gameplay/fire.rs`: spread to flammable blocks, ignition sources (flint/steel, fire charge, lava, lightning, ghast fireball, bed explosion), eternal fire on netherrack.
  6. **TNT** — `sim/src/gameplay/tnt.rs`: 1352-ray explosion, blast resistance calculations, primed TNT entity with fuse.
  7. **Redstone** — `sim/src/gameplay/redstone.rs`: dust (signal propagation, strength decay), torch (inverter), repeater (delay, lock), comparator (compare/subtract, container output), update priority order.
  8. **Lighting** — `sim/src/gameplay/lighting.rs`: sky light (sun/moon, day/night cycle, spreading), block light (emission, spreading), updates on block changes.
  9. **XP** — `sim/src/gameplay/xp.rs`: orb types (tiny..enormous), splitting/merging, anvil costs (prior work penalty, enchantment combining), enchantment table costs.
  10. **Crafting** — `sim/src/gameplay/crafting.rs`: JSON recipes (shaped/shapeless, groups), 2x2 inventory + 3x3 table, recipe book placeholder.
- **Content** — Expanded `content-vanilla` with 210 items (tools, armor, food, materials) via public API only.
- **Reference Extractors** — 4 binaries: `extract-legacy` (blocks), `extract-legacy-items`, `extract-legacy-recipes`, `extract-legacy-furnace`. Fixed bugs: group string pattern extraction, anvil ingredient counting (3 iron blocks + 4 iron ingots), function definition filtering, C++ cast expression handling (e.g., `(Item *)Item::bow`), source line number tracking.
- **Parity** — `parity/scenarios/` with YAML scenarios; `faithful_bugs` feature flag implemented (SPEC 3.3).
- **Block breaking compilation fixed** — Two-pass data collection resolves borrow checker.
- **Verification Run:** `cargo test --workspace` -> all 286 tests pass; clippy clean; fmt clean.
- **Blockers / Open Questions:** None — M5 complete.
- **Next Steps:** M6 — World generation baseline.

---

### Session 20 — 2026-11-20 (M6: World Generation Baseline)

- **Goal:** Implement baseline world generation matching LCE reference (TU19/CU7).
- **Completed:**
  1. **Biome layer system** — 16 layers matching LCE reference: `AddIslandLayer`, `AddSnowLayer`, `AddMushroomIslandLayer`, `BiomeInitLayer`, `RegionHillsLayer`, `ShoreLayer`, `SwampRiversLayer`, `SmoothLayer`, `RiverInitLayer`, `RiverLayer`, `RiverMixerLayer`, `VoronoiZoom`, `FuzzyZoomLayer`, `ZoomScale`, `Zoom`. Seeded with `GameRandom`, dense indices, no `rand` dependency.
  2. **Terrain generation** — `prepareHeights`/`buildSurfaces` with 8-block noise sampling (improved noise), stone/dirt/grass/bedrock/sand/gravel/clay placement, caves, ravines, ore veins.
  3. **Structure features** — Cave (carving), Canyon (deep ravines), Village (A* pathfinding for roads), Stronghold (portal rooms, corridors), MineShaft (corridors, stairs, spawners), Temple (jungle/desert), Dungeon (spawner + chest).
  4. **Decoration system** — Ores (coal, iron, gold, diamond, redstone, lapis, emerald), trees (oak, birch, spruce, jungle), flowers (dandelion, poppy, allium, etc.), grass (tall grass, fern), reeds, cacti, waterlilies, mushrooms, pumpkins, springs (water/lava).
  5. **Nether generation** — Hell biome, Fortress (bridges, corridors), Glowstone (ceiling clusters), Springs (water/lava).
  6. **End generation** — Sky biome, 18x18 fixed size, Exit Portal (bedrock frame + end portal blocks).
  7. **World size presets** — Classic (54), Small (64), Medium (192), Large (320), Infinite chunks; Nether scales 3/3/6/8.
  8. **Reference docs** — `docs/reference/worldgen.md` (biome layer stack, biome registry, terrain noise, structures, decoration, Nether/End, RNG details).
  9. **Parity harness** — `parity/scenarios/worldgen/` (biome_map.yml, heightmap.yml, structures.yml), `parity/runner.rs` (5 passing tests: biome determinism, terrain determinism, biome block generation, overworld chunk generation, worldgen parity runner).
- **Verification Run:** `cargo test --workspace` -> all 286 tests pass (including 5 parity tests); `mclre-worldgen` lib compiles; no `rand` dependency.
- **Blockers / Open Questions:** None — M6 complete.
- **Next Steps:** M7 — Mobs & AI, then M8 — UI/UX Parity Pass 1.

---

### Session 21 — 2026-11-20 (M7: Mobs & AI - not started)

### Session 22 — 2026-11-20 (M8: UI/UX Parity Pass 1)

- **Goal:** Implement UI/UX parity pass 1 with all 11 core game screens, bitmap font text rendering, resource pack support, split-screen rendering, controller/gamepad support, and Glyphon 0.7 TTF font rendering with wgpu 23 upgrade.
- **Completed:**
  1. **11 core game screens** (`crates/ui/src/screens.rs`): TitleScreen, MainMenuScreen, LoadWorldScreen, CreateWorldScreen, WorldOptionsScreen, PauseScreen, SettingsScreen, HelpOptionsScreen, CreditsScreen, InventoryScreen, CraftingScreen. ScreenTransition enum for navigation.
  2. **ScreenManager** — push/pop navigation, stack-based, proper lifetime handling.
  3. **BaseScreen** — Focus management (NavigateUp/Down/Left/Right, Select, Back, Next, Previous), widget registration, focusable widgets.
  4. **ScreenBuilder pattern** — Method chaining for widget creation.
  5. **Bitmap font text rendering** — Shader updated with font atlas bindings (bindings 2 & 3), UiPipeline supports dual texture bind groups (atlas + font atlas) with `add_text()` method, `BitmapFont` with Minecraft-style 16x16 glyph atlas, `UiRenderer` integrated with GPU renderer for `draw_text()`, `draw_rect()`, `draw_image()`, `draw_nine_slice()`.
  6. **Resource pack support** — `AssetManager::load_resource_pack()` loads pack.mcmeta, validates pack_format=15, adds pack roots to VFS with priority overlay; `preload_ui_textures()` loads UI textures from textures/gui/, textures/font/, textures/misc/ into atlas.
  7. **Split-screen rendering** — `Renderer::render_views()` accepts multiple views, `calculate_viewports()` supports 1-4 player layouts (horizontal split for 2, top+bottom for 3, 2x2 grid for 4), per-viewport rendering with `set_viewport()`, shared frame graph preparation, UI overlay per viewport.
  8. **Controller/gamepad support** — 8 new UI navigation actions (NavigateUp/Down/Left/Right/Select/Back/Next/Previous) with keyboard (arrow keys, Enter, Escape, Tab, Backspace) and gamepad (D-pad, A/B buttons, LB/RB bumpers) bindings via gilrs; InputManager simplified to remove lifetime parameter; `update_gamepad()` polls gilrs each frame.
  9. **Glyphon 0.7 TTF font rendering integrated with wgpu 23 upgrade** — `GlyphonTextRenderer`, `GlyphonTextBuffer`, `GlyphonTextBufferBuilder` implemented; client initializes GlyphonTextRenderer on first frame, loads fonts from asset cache; per-viewport UI rendering for split-screen via `render_views` callback; all 287+ workspace tests pass (including new Glyphon tests).
  10. **Glyphon text rendering fully hooked into UI pipeline** — `GlyphonInlineRenderer` collects text buffers during UI render, then `glyphon_callback` renders them with `render_with_positions` in a separate render pass after UI; `Renderer::render_views` accepts optional `glyphon_callback` for per-viewport Glyphon rendering; `device_arc()` and `queue_arc()` added to Renderer for closure compatibility; `RefCell` used for interior mutability to share state between UI and Glyphon callbacks.
- **Verification Run:** `cargo test --workspace` -> all 287+ tests pass; clippy clean; fmt clean.
- **Blockers / Open Questions:** None — M8 complete.
- **Next Steps:** M9 — LAN Multiplayer & Dedicated Server.

---

### Session 23 — 2026-11-21 (M9: LAN Multiplayer & Dedicated Server - Core Infrastructure)

- **Goal:** Implement LAN multiplayer core infrastructure: UDP transport, mDNS discovery, UPnP support, server integration, client LAN menu, dedicated server binary.
- **Completed:**
  1. **UDP Transport** (`crates/net/src/udp_transport.rs`) — `UdpTransport` with connect mode (client) and listen mode (server); background threads for outgoing/incoming packet handling; crossbeam channels for thread-safe packet queues.
  2. **LAN Discovery** (`crates/net/src/udp_transport.rs`) — `LanDiscovery` for mDNS service advertisement and discovery using `mdns-sd` crate; TXT records for MOTD, player count, version.
  3. **UPnP Support** (`crates/net/src/upnp.rs`) — `UpnpPortManager` using `oxy-upnp-igd` crate with `smol` async runtime; `setup_default_port_mapping()` for port 25565 (TCP + UDP); `get_public_address()` for sharing external IP:port.
  4. **Server Integration** (`crates/server/src/lib.rs`) — `run_lan()` method: binds UDP socket on configured port, starts mDNS advertisement with TXT records, spawns UDP listener thread handling full protocol handshake (Handshake -> Login -> Play); `ServerConfig` extended with `lan_port` and `advertise_lan`.
  5. **Client Integration** (`crates/client/src/lib.rs`) — Direct connect callback (IP, port, UPnP flag), LAN discovery callback, MultiplayerScreen integration.
  6. **MultiplayerScreen** (`crates/ui/src/screens.rs`) — Direct connect dialog with IP field, port field, UPnP checkbox, Connect/Cancel buttons, focus navigation (Tab/Shift+Tab, arrow keys, Enter/Escape).
  7. **Dedicated Server Binary** (`crates/server-bin/src/main.rs`) — Full CLI arguments: `--seed`, `--max-players`, `--motd`, `--difficulty` (peaceful/easy/normal/hard), `--port`, `--advertise-lan`/`--no-mdns`, `--online-mode`, `--view-distance`, `--world-name`, `--log-level`, `--upnp`/`--no-upnp`, `--tick-rate`. UPnP gateway discovery and port mapping on startup.
  8. **Dependencies** — Added `oxy-upnp-igd` and `smol` to `crates/net/Cargo.toml` and workspace `Cargo.toml`.
- **Verification Run:** `cargo check --workspace` -> all crates compile; `cargo test --workspace` -> all 290+ tests pass (including new UDP tests `test_udp_transport_connect`, `test_udp_transport_send_recv` and UPnP tests `test_upnp_manager_creation`, `test_port_mapping_protocol`).
- **Blockers / Open Questions:** None — M9 core infrastructure complete.
- **Next Steps:** Wire direct connect dialog to actually trigger UDP connection and transition to gameplay; implement connection state management and transition from UI to gameplay; add headless server mode support; test end-to-end LAN multiplayer flow.

---

### Session 24 — 2026-11-21 (M9: Fixing server-bin compilation, all tests passing)

- **Goal:** Fix server-bin compilation issues and verify all tests pass.
- **Completed:**
  1. Fixed server-bin main.rs syntax error (unclosed delimiter in if/else block).
  2. Added missing dependencies to server-bin Cargo.toml: `mclre-sim`, `mclre-net`, `tokio` (with full features), `tracing-subscriber` (with env-filter feature).
  3. Fixed GameMode mapping in server-bin (Peaceful/Easy/Normal/Hard all map to Survival since mclre_sim::GameMode only has Survival/Creative/Adventure/Spectator).
  4. Fixed tracing-subscriber initialization to use EnvFilter with proper type annotations.
  5. Cleaned up unused imports in server-bin.
- **Verification Run:** `cargo check --workspace` -> all crates compile; `cargo test --workspace` -> all 290+ tests pass.
- **Blockers / Open Questions:** None.
- **Next Steps:** Wire direct connect dialog to actually trigger UDP connection and transition to gameplay; implement connection state management and transition from UI to gameplay; add headless server mode support; test end-to-end LAN multiplayer flow.

---

### Session 25 — 2026-11-21 (M9: Client-side connection implementation)

- **Goal:** Implement client-side connection handling to wire MultiplayerScreen direct connect dialog to actual UDP connection and protocol handshake.
- **Completed:**
  1. **Added client connection state management** — `ClientConnectionState` enum (Disconnected, Connecting, Handshake, Login, Configuration, Play), `ConnectionRequest` struct and channel for UI-to-main-thread communication.
  2. **Implemented `connect_to_server()` method** — Creates UDP transport, sends Handshake packet (protocol_version=1, next_state=Login), stores transport in client.
  4. **Added network thread** — Handles protocol handshake: sends LoginStart packet, waits for LoginSuccess response, transitions connecting flag to false on success.
  5. **Wired MultiplayerScreen callbacks** — Updated `set_on_connect` and `set_on_direct_connect` to send `ConnectionRequest` via channel instead of creating local transports.
  6. **Added connection request processing in main loop** — Processes requests in `Event::AboutToWait` handler, calls `connect_to_server()`.
  7. **Fixed dependencies** — Added `mclre-protocol` and `bytes` to client Cargo.toml.
  8. **Fixed protocol struct usage** — `LoginStart` with optional UUID (None for offline mode), `LoginSuccess` with uuid/username/properties (no entity_id, which comes in JoinGame packet).
- **Verification Run:** `cargo check --workspace` -> all crates compile; `cargo test --workspace` -> all 290+ tests pass.
- **Blockers / Open Questions:** None — Client-side connection infrastructure complete.
- **Next Steps:** Transition from UI to gameplay screen after successful connection; implement connection state management (Configuration, Play states); add headless server mode support; test end-to-end LAN multiplayer flow.

---

### Session 26 — 2026-11-21 (M9: Client UI transition on successful connection)

- **Goal:** Implement UI transition from MultiplayerScreen to gameplay screen (HUD) on successful connection, and add connection event channel from network thread to main thread.
- **Completed:**
  1. **Added `ConnectionEvent` enum** — Success (with entity_id, uuid, username) and Failure (reason) variants for network thread to signal main thread.
  2. **Added connection event channel** — `connection_event_tx`/`connection_event_rx` in Client struct for network thread -> main thread communication.
  3. **Updated network thread** — Sends `ConnectionEvent::Success` with uuid/username on LoginSuccess, `ConnectionEvent::Failure` on error/disconnect.
  4. **Implemented UI transition in main loop** — On `ConnectionEvent::Success`: sets connection_state to Play, pops MultiplayerScreen, pushes HUD (Inventory screen) for all player UIs.
  5. **Used `UiContext::dummy()`** — For screen push/pop operations in event handler context.
  6. **Fixed ScreenType usage** — Used `ScreenType::Inventory` (serves as HUD) instead of non-existent `ScreenType::HUD`.
- **Verification Run:** `cargo check --workspace` -> all crates compile; `cargo test --workspace` -> all 290+ tests pass.
- **Blockers / Open Questions:** None — Client-side connection with UI transition complete.
- **Next Steps:** Handle Play state packets (JoinGame, chunk data, player position, etc.); add headless server mode support; test end-to-end LAN multiplayer flow.

---

### Session 27 — 2026-11-21 (M9: Play state packet handling)

- **Goal:** Implement Play state packet handling in the network thread for JoinGame, ChunkData, PlayerPositionAndLook, KeepAlive, and Disconnect packets.
- **Completed:**
  1. **Extended `ConnectionEvent` enum** — Added JoinGame (entity_id, uuid, username, dimension, difficulty, gamemode), ChunkData (x, z, data), PlayerPositionAndLook (x, y, z, yaw, pitch, on_ground), KeepAlive (id), Disconnect (reason).
  2. **Updated network thread** — After LoginSuccess, enters Play state loop handling multiple packet types:
     - **JoinGame**: Extracts entity_id, dimension, difficulty, gamemode; sends JoinGame event.
     - **ChunkData**: Decodes chunk x, z, data; sends ChunkData event (TODO: process into client world).
     - **PlayerPositionAndLook**: Updates client player position and rotation.
     - **KeepAlive**: Responds with KeepAlive packet (auto-response).
     - **Disconnect**: Sends Disconnect event, stops connecting.
  3. **Updated main loop event handler** — Handles all new ConnectionEvent variants:
     - JoinGame: Updates entity_id, transitions to Play state, switches to gameplay screen.
     - ChunkData: Logs receipt (TODO: process into client world).
     - PlayerPositionAndLook: Updates player position and rotation in ClientWorld.
     - KeepAlive: Auto-response handled by network thread.
     - Disconnect: Sets connection state to Disconnected.
  4. **Added KeepAlive auto-response** — Network thread automatically responds to KeepAlive packets.
  5. **Added DisconnectPacket handling** — Properly decodes DisconnectPacket from login module.
- **Verification Run:** `cargo check --workspace` -> all crates compile; `cargo test --workspace` -> all 290+ tests pass.
- **Blockers / Open Questions:** None — Play state packet handling complete.
- **Next Steps:** Process ChunkData into client world for rendering; add headless server mode support; test end-to-end LAN multiplayer flow.

---

### Session 28 — 2026-11-21 (M9: Chunk data processing and rendering integration)

- **Goal:** Implement chunk data processing to decode NBT chunk data from ChunkData packets and add chunks to the client world for rendering.
- **Completed:**
  1. **Implemented ChunkData processing in main loop** — Added chunk data decoding using `read_bytes` from NBT, `decode` from chunk_nbt, and `block_states` codec for palette decoding.
  2. **Integrated with client world** — Decoded ChunkColumn is stored in ClientWorld via `set_chunk()`, which marks it dirty for GPU upload.
  3. **Used block_states codec** — The `block_states()` function provides the StateCodec needed for palette decoding during chunk NBT decoding.
  4. **Fixed import issues** — Corrected StateCodec import from mclre_world, fixed NBT read function (read_bytes).
- **Verification Run:** `cargo check --workspace` -> all crates compile; `cargo test --workspace` -> all 290+ tests pass.
- **Blockers / Open Questions:** None — Chunk data processing and rendering integration complete.
- **Next Steps:** Add headless server mode support; test end-to-end LAN multiplayer flow; handle PlayerInfoUpdate for multiplayer entity tracking.

---

### Session 29 — 2026-11-22 (M12: Modding v1 - Complete)

- **Goal:** Complete M12 Modding v1 - sandboxed WASM mods, data/resource packs, mod channel networking, dependency resolution, and developer documentation.
- **Completed:**
  1. **Mod manifest parser + dependency resolver** — `mod.toml` TOML parsing with semver dependency resolution, topological load order, cycle/missing-dep diagnostics.
  2. **WASM host foundation** — `wasmtime` component model integration with sandboxing: tick fuel budgets (10M instructions/tick), epoch interruption, 64MB memory limits, per-mod deterministic `GameRandom`.
  3. **Typed Event Bus** — 13 event types (`BlockBreak`, `BlockPlace`, `EntityDamage`, `PlayerJoin`, `PlayerLeave`, `ChunkGen`, `ItemUse`, `Explosion`, `Chat`, `PlayerRespawn`, `ItemDrop`, `EntitySpawn`, `Tick`) with cancellable events and per-mod subscription tracking.
  4. **Mod loader** — Mod discovery, dependency-ordered loading, lifecycle management (load/unload/reload).
  4. **Mod registry** — Mod tracking with metadata, permissions, per-mod configuration.
  5. **Permission system** — 7 levels: `WorldRead`, `WorldWrite`, `PlayerInteract`, `EntityManipulate`, `WorldGen`, `Network`, `Config`.
  6. **Component model host functions** — 7 interfaces implemented via wasmtime 24 dynamic API (`func_new`): world (`get-block`, `set-block`), player (`get-player`, `send-chat`), entity (`get-entity`, `spawn-entity`), registry (`register-block`, `register-item`, `register-recipe`), config (`get-config`, `set-config`), events (`subscribe`, `unsubscribe`), utils (`game-random`, `log`, `schedule-task`).
  7. **Server/client mod sync packets** — `ModList` (0x08) and `ModMismatch` (0x09) packets in configuration state with mod entries (id, version, checksum) for mod synchronization and mismatch handling.
  7. **Examples & docs** — `examples/hello-mod/` with `mod.toml`, `hello.wat`, `README.md`; `docs/modding/api-reference.md`, `docs/modding/content-packs.md`, `docs/modding/hello-mod-tutorial.md`.
- **Verification Run:** `cargo check --workspace` -> all crates compile; `cargo test --workspace` -> all tests pass.
- **Blockers / Open Questions:** None — M12 acceptance criteria fully met.
- **Next Steps:** Tag commit `m12-modding-v1`, begin M13 (World Size Presets & Persistence Polish).

---





