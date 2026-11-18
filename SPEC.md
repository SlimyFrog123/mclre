# SPEC: Minecraft: Legacy Rust Edition (MCLRE)

> **Project name:** Minecraft: Legacy Rust Edition, abbreviated **MCLRE**. Repository name: `mclre`. Binaries: `mclre` (client) and `mclre-server` (dedicated server). A Rust rewrite of Minecraft: Legacy Console Edition.
> **Audience:** Claude Code (primary implementer) and the project owner (reviewer/decision maker)
> **Status:** v0.1 draft. Items marked **[VERIFY]** are facts from memory that must be checked against primary sources before being relied on. Items marked **[DECIDE]** need an explicit owner decision (record it as an ADR in `docs/adr/`).

---

## 0. How to use this document

1. Read sections 1–5 fully before writing any code. They define the non-negotiable architecture.
2. Work milestone by milestone (section 17). Do not start a milestone until the previous one meets its acceptance criteria.
3. When this spec is ambiguous or wrong, **do not guess silently**. Write a short ADR (`docs/adr/NNNN-title.md`: context, options, decision, consequences), flag it to the owner, and proceed with the least-irreversible option.
4. "Faithful" always means: *matches the behavior of the reference LCE build the active game profile targets*, not "matches Java Edition" and not "matches what I think is nicer". Improvements are allowed only where this spec lists them (section 3.3) and must be toggleable where they alter gameplay.
5. Keep `CLAUDE.md` at the repo root with a condensed version of sections 2, 3.5, 3.6, 4, 15 and 16 so every session starts with the core rules.

---

## 1. Vision and goals

### 1.1 One-sentence goal
A from-scratch, clean-architecture Rust reimplementation of Minecraft: Legacy Console Edition that recreates the original 1:1 (seed- and behavior-compatible, see section 3.6) with major improvements, runs well on modern and low/mid-range PCs, and is structured so it can be upgraded Title Update (TU) by TU and eventually reach feature and multiplayer parity with Minecraft: Java Edition.

### 1.2 Horizons

| Horizon | Goal |
|---|---|
| **Short term (v1.0)** | Fully working, playable, faithful build of the LCE version released on **December 17, 2014** (the "baseline profile"), with modern-hardware improvements, LAN multiplayer, local splitscreen, and early modding support. |
| **Mid term** | Advance the game TU by TU up to the newest version released on Xbox One / PS4 **[VERIFY: final Legacy Console TU number and its approximate Java parity]**. Integrate Iris-style shaderpack support. Java skins, LCE world importer, DLC/content-pack support. |
| **Long term** | Full parity and multiplayer compatibility with up-to-date Java Edition, delivered on an update-by-update basis, behind per-version game profiles. |

### 1.3 Explicit non-goals (for now)
- Console/handheld ports (Switch, Vita, PS3/360, etc.). Desktop only: Windows first, then Linux, then macOS.
- Online/internet multiplayer, matchmaking, relays (design for it; do not build it).
- Real authentication (placeholder only; see section 9).
- Bedrock Edition compatibility.
- Public or mass distribution of any kind. (Original game assets **are** kept in the private repo for now, in a clearly isolated directory; see sections 11.3 and 16.)

---

## 2. Architectural pillars (non-negotiable)

1. **Strict client/server separation.** The server crate must compile and run with **zero** dependency on graphics, audio, windowing, or UI crates. Singleplayer = an *integrated server* running in-process, talking to the client through the **same protocol and same code path** as a remote connection (in-memory channel transport instead of sockets). No "singleplayer shortcuts" that bypass the protocol.
2. **Namespaced identifiers everywhere.** No numeric block/item/entity/biome IDs in game logic, saves, or protocol. Everything is `namespace:path` (see section 5). Numeric IDs exist only inside (a) per-session protocol palettes, (b) in-memory registry indices, and (c) legacy-mapping tables used by the importer.
3. **Block states, not metadata.** Represent blocks as `(block id, typed property map)` from day one (modern "flattened" model), not legacy 4-bit metadata. Provide a legacy-metadata ↔ state mapping table for the importer and for fidelity checks.
4. **Game versions are data-driven profiles.** All version-specific behavior (mechanics, worldgen, recipes, mobs, protocol) is selected through a **Game Profile** (section 6). No scattered `if version >= X` in random places; use profile feature flags and versioned modules.
5. **Vanilla is a mod.** The base game content registers itself through the *same* public registry/event API available to mods. If vanilla can't be built with the mod API, the API is wrong.
6. **Deterministic, fixed-timestep simulation.** Server tick = 20 TPS, fixed step, no wall-clock dependence inside simulation. Worldgen and simulation use explicit seeded RNGs (and must be reproducible across platforms: no `HashMap` iteration order dependence, no float non-determinism in worldgen; use fixed-point or carefully controlled `f64`).
7. **Modern asset formats, legacy-format converters at the edge.** The engine only reads modern formats. Old formats (`.binka`, `.arc`, `.dds`, console save formats) are handled by offline *importer/converter tools* that run on user-supplied game files.
8. **Hardware-limit workarounds are options, not architecture.** Console-era limits (world size caps, render distance, entity caps, hard-coded chunk counts) are encoded as **presets/settings**, never as hard-coded assumptions in core systems.
9. **Performance is a feature.** Must be comfortably playable on low-to-mid range hardware (see budgets in section 12.5).

---

## 3. Fidelity policy

### 3.1 Reference target
- **Baseline profile:** LCE as of **December 17, 2014**. Identify the exact TU/CU number from primary sources and record it in `docs/reference/baseline.md` **[VERIFY: this was around TU19 on Xbox 360/PS3 — confirm]**.
- Primary reference: the original source and assets tree (section 3.5). Supplementary material the owner may supply: saves, and screenshots/video captures of the running original for visual and behavioral comparison. Claude Code should build comparison harnesses that consume this material from a user-configured path.

### 3.2 What "faithful" covers
Gameplay rules, block/item/mob behavior and stats, recipes, crafting UI flow (console-style crafting menu with groups/tabs), inventory layout, controls and controller-first UX, HUD, menus and menu flow, tutorial world, splitscreen (up to 4 local players), world-size presets, biome layouts as scaled by LCE, sounds/music selection, visual style (smooth lighting, fog, sky, particles), command-less feel (host privileges via in-game menu rather than slash commands), game options (difficulty, game mode, host options, trust players, fire spread, TNT explodes, etc.).

### 3.3 Allowed deviations (improvements for modern hardware)
All of these must be **options** with the faithful behavior as the default unless noted:
- Resolution/aspect ratio/FOV/UI scale beyond console limits; uncapped or configurable frame rate; VSync option.
- Render distance beyond console limits (configurable; default matches LCE's feel).
- Higher-resolution texture pack support, mipmapping/anisotropic filtering options, MSAA/TAA options.
- Keyboard + mouse as first-class input (in addition to gamepad).
- Larger entity/particle caps than console (configurable).
- World size presets beyond console limits (section 8).
- Modern audio/asset formats; higher-quality audio.
- Bugfix toggles: where an LCE bug is known, provide `faithful_bugs` setting (default: **on** for gameplay-affecting bugs that players rely on; **off** for crashes/exploits that corrupt saves).

### 3.4 Verification approach
- Golden-data tests wherever possible: recipes table, loot tables, block hardness/blast resistance, item stats, mob attributes, XP curves, worldgen outputs for fixed seeds (compared with data dumped from the reference build, supplied by owner).
- A `parity/` directory with scenario descriptions (YAML) describing observed reference behavior and expected results; these become integration tests.

### 3.5 Original source and assets (the reference tree)

The owner has designated the original December 17, 2014 build's source code and assets as the **authoritative reference**:

- **Location:** `https://github.com/furax124/Minecraft-LCE-PS3/tree/main/MinecraftConsoles-141217`
- **Contents (per owner):** `Minecraft.Client` and `Minecraft.World`, which together contain everything code-wise needed for the initial build, plus the assets for the original version. **[VERIFY]** The exact directory layout, build system, and where assets sit were not inspected when this spec was written; confirm them first and record the findings in `docs/reference/reference-tree.md`.
- **How it is attached to this project:** clone it as a **read-only git submodule** at `reference/lce-141217/` (or a plain clone that is git-ignored). It is **never edited, never compiled into the game, and no file from it is copied into `crates/`**. Original assets are consumed only through `assetc` into `game-assets/` (section 11.3).
- **Provisional mapping** (confirm after reading the tree):

  | Reference | Informs (our crates) |
  |---|---|
  | `Minecraft.World` (levels, tiles/blocks, items, entities/mobs, AI, worldgen, biomes, structures, storage, recipes, game rules, packets) | `core`, `world`, `worldgen`, `sim`, `content-vanilla`, `protocol` (semantics only), `profiles` |
  | `Minecraft.Client` (rendering, models, particles, UI/menus, audio, input, platform glue, and likely the in-process server and connection code) | `client`, `render`, `ui`, `audio`, `input`, `server` (tick order, host options), `assets` |

- **Precedence of truth:** reference source > owner-supplied captures of the running original > wiki/community documentation > assumptions. When they disagree, record it in `docs/reference/discrepancies.md`.
- **What to take from it, exactly:** algorithms and data where fidelity is observable: `java.util.Random` call order in worldgen, noise parameters, biome layer stacks and scaling, structure generation, explosion algorithm, lighting propagation, fluid/fire/redstone update rules and tick ordering, mob attributes and AI goal priorities, loot/drop tables, recipes, item/block stats, XP curves, crafting menu grouping, UI layout and flow, animation timings, sound event mapping.
- **What not to take:** the architecture. This spec's architecture wins: namespaced IDs, block states instead of metadata, strict client/server separation, profile-driven versioning, registry-based content, modern asset formats. Do not reproduce class hierarchies, globals, singletons, or manual memory management; do not carry over console-platform code (PS3/360/PS4/Xbox One/Vita/Switch layers, SPU/threading shims, console storage and networking layers, platform trophy/profile services, the Flash/Iggy UI runtime). Port **behavior behind our abstractions**; idiomatic Rust, not line-by-line transliteration.
- **Numeric IDs in the reference** are translated to `namespace:path` through a generated legacy mapping table (`data/legacy/*.toml`), produced by a devtool that reads the reference data (section 5.3), not hand-typed.
- **Traceability:** write per-feature behavior notes in `docs/reference/<feature>.md` that cite reference file and function names (pointers, not pasted code) and the parity test(s) that cover them. Each feature flag in `feature-flags.md` cites its evidence the same way.
- **Extraction tooling:** `tools/devtools` includes extractors that read the reference tree and emit data files (recipes, item/block properties, mob attributes, loot tables, biome tables, language strings) into `data/`, so golden-data tests are generated from the reference, not retyped.

### 3.6 Seed compatibility and 1:1 parity (hard requirement)

MCLRE is a **1:1 recreation of the original with major improvements**. The improvements are exactly the deviations allowed in section 3.3 plus new content added by later profiles. **Everything else must behave identically to the reference.**

**Seed goal, both directions:**
- *Original → MCLRE:* a seed (and world size preset) from the original game generates the same world in MCLRE under the baseline profile.
- *MCLRE → original:* a seed from an MCLRE world created with a legacy-size preset, the baseline profile, and no new-mechanic flags can be typed into the original game and yields the same world.
- **Seed parity region:** parity is defined for the original's bounded world sizes. The unique identity of a world is the pair **(seed, size preset)**, because the original scales biome layout by world size. `infinite` worlds have no original counterpart; inside the original bounds they should match the corresponding preset only if the owner asks for it **[DECIDE]**, and everything outside is new behavior.
- Seed input must behave like the original: text seed → numeric seed conversion (reproduce the original hashing, including how an empty seed picks a random one), numeric strings, negative values, and 64-bit wrap-around.

**Randomness rules (apply to every system that affects gameplay or world state):**
1. **Port the reference RNG exactly:** same algorithm, same state update, same `nextInt(bound)` / `nextLong` / `nextFloat` / `nextDouble` / `nextGaussian` semantics, same seeding helpers. Unit-test against vectors produced by the reference.
2. **Replicate seed derivation exactly:** world seed → per-system seeds, per-chunk seeds (the reference's chunk-coordinate mixing and XOR constants), biome layer seeds, structure seeds (village/stronghold/mineshaft/temple/fortress region logic), decoration seeds, mob spawn, and so on. Record each derivation in `docs/reference/rng.md` with reference file/function pointers.
3. **Call order and call count matter.** Reproduce every RNG call in the same order, including calls whose results look unused or discarded, and calls inside loops whose count depends on earlier results. Never "optimize away" or reorder an RNG call.
4. **Replicate container iteration order** wherever it influences RNG consumption (for example the reference's list or map ordering for entities, block updates, structure piece lists). Do not use `HashMap` iteration; use ordered structures that reproduce the reference order.
5. **Numeric semantics:** match the reference's types and arithmetic: `f32` vs `f64` per operation, integer widths, wrapping vs saturating behavior (use `wrapping_*` explicitly), rounding and float→int conversion rules, operation order, and any lookup tables used instead of libm (for example sine tables) **[VERIFY in reference]**. No fast-math, no reassociation, no replacing the reference's noise or math helpers with crate equivalents. Add a CI check that floating-point results are bit-identical across Windows and Linux for the worldgen test seeds.
6. **Type-level separation:** provide `GameRandom` (reference-exact, deterministic, used by `world`, `worldgen`, `sim`, `content-vanilla`) and `CosmeticRandom` (client-only visuals such as particles and cosmetic jitter, free to differ). Gameplay crates must not depend on `rand`; enforce with a CI dependency check or lint.
7. **Anything that affects the server world must use `GameRandom`:** terrain, caves, structures, decoration, ore placement, random ticks (including crop growth, fire, leaf decay), mob spawning and AI randomness, drops and loot, enchanting, fishing, villager trades, XP, weather and thunder, explosion rays, etc. When unsure whether a use is cosmetic, treat it as gameplay.

**Process for "all non-new gameplay logic is checked against the source":**
- **Definition of Done for every gameplay feature:** (a) the reference files/functions are cited in `docs/reference/<feature>.md`; (b) all RNG usage was verified against the reference (call order, count, seeding); (c) a parity test exists (golden data, scenario, or seed-based) and passes; (d) any intentional deviation is listed in `docs/reference/discrepancies.md` and maps to an allowed improvement in 3.3 or a toggled flag.
- A feature without these cannot be marked complete in a milestone.
- Anything not on the improvement list is treated as a bug if it differs from the reference.

**Seed parity test harness:**
- Produce reference data for a fixed set of seeds × size presets × dimensions: per-chunk hashes of block data, biome maps, heightmaps, structure positions/types, and spawn point. Obtain it by whichever route the owner prefers **[DECIDE]**: (1) a small standalone harness built from the reference worldgen code, or (2) dumps captured from the running original, or (3) both for cross-checking.
- Compare in CI (reference dumps are stored in `game-assets/` or a data directory kept out of public hands, see section 16). Target: **exact match** for terrain, biomes, structures, and decoration inside the seed parity region.
- Add a differential debugger: for the first mismatching chunk, report the first differing stage (biomes → heightmap → caves → structures → decoration) and the RNG state at that point.

---

## 4. Workspace layout

Cargo workspace, edition 2024 (or latest stable), MSRV pinned in `rust-toolchain.toml`. Suggested crates (names are suggestions):

```
mclre/
├─ Cargo.toml                 # workspace
├─ CLAUDE.md                  # condensed rules for Claude Code
├─ spec.md                    # this file
├─ docs/{adr,reference,modding}/
├─ crates/
│  ├─ core/                   # Identifier, registries, math types, RNG, errors, NBT-like data, tick types
│  ├─ content-vanilla/        # vanilla blocks/items/entities/recipes registered via the public API
│  ├─ profiles/               # Game Profile definitions + feature flags + version modules
│  ├─ world/                  # chunks, block states, palettes, lighting, storage (region files), dimensions
│  ├─ worldgen/               # versioned world generators (per profile), biomes, structures
│  ├─ sim/                    # server-side game simulation: ticking, entities, physics, AI, redstone, fluids
│  ├─ protocol/               # packet definitions, codec, versioned handshake, transport-agnostic
│  ├─ net/                    # transports: in-memory, UDP/QUIC LAN, discovery; (later UPnP/relay)
│  ├─ server/                 # server library (integrated + dedicated), command/host-permission logic
│  ├─ server-bin/             # `mclre-server` dedicated server executable (headless)
│  ├─ client/                 # client library: prediction, interpolation, local player(s), world mirror
│  ├─ render/                 # wgpu renderer, chunk meshing, render graph, shaderpack hooks (later)
│  ├─ ui/                     # retained-mode UI toolkit recreating LCE UI, menus, HUD, crafting menus
│  ├─ audio/                  # audio engine
│  ├─ input/                  # keyboard/mouse/gamepad abstraction, per-player bindings
│  ├─ assets/                 # asset loading, resource packs, atlas building, localization
│  ├─ modding/                # mod manifest, loader, WASM host, mod API (host-side)
│  ├─ mod-api/                # guest-side SDK crate that mods depend on (no_std-friendly where possible)
│  ├─ identity/               # IdentityProvider trait + launch-arg implementation (placeholder)
│  ├─ app/                    # `mclre` client executable: window loop, glue, CLI args
│  └─ tools/
│     ├─ assetc/              # asset importer/converter from user's LCE install (binka→ogg, etc.)
│     ├─ lce-import/          # LCE world importer (later milestone)
│     └─ devtools/            # worldgen viewers, packet inspectors, registry dumpers
└─ parity/                    # parity scenarios & golden-data comparison harness
```

**Dependency rules (enforced in CI with `cargo-deny` + a dependency-graph test):**
- `core`, `world`, `worldgen`, `sim`, `protocol`, `server`, `profiles`, `content-vanilla` **must not** depend on `wgpu`, `winit`, audio, or UI crates.
- `client` may depend on `protocol`, `world`, `core`; **not** on `sim` internals or `server`. (Client-side prediction uses a small, explicitly shared `physics` module in `core` or `world`, not the server's full simulation.)
- `app` is the only crate allowed to wire `client` + `server` together (integrated server).

---

## 5. Identifiers, registries, and data model

### 5.1 `Identifier`
- Type: `Identifier { namespace: Ns, path: Path }`, string form `namespace:path`. Default namespace is `minecraft` when omitted in *user-facing input only* (never in storage).
- Validation matches modern Minecraft: namespace `[a-z0-9_.-]`, path `[a-z0-9_.-/]`.
- Interned for cheap comparison/hashing; `Copy`-able handle form for hot paths.
- Which namespaces: `minecraft` for vanilla content that exists in Java Edition under the same name; `lce` for **LCE-only** content (mini-games, console-exclusive items/blocks/mobs, console UI assets, etc.); mod namespaces for mods. **[DECIDE]**: whether LCE-only content that later became vanilla stays in `lce:` or migrates to `minecraft:` — recommended: use `minecraft:` for anything that exists in Java under that ID (e.g. `minecraft:stone`), `lce:` only for things with no Java equivalent.

### 5.2 Registries
- Typed registries: blocks, items, entity types, block entity types, biomes, dimensions, enchantments, status effects, recipes (types), sound events, particle types, game rules, menu types, etc.
- Lifecycle: `Registering → Frozen`. Registration happens during startup (vanilla + mods); after freeze the registry is immutable and indexed densely for fast runtime lookup.
- Server → client **registry sync** at join: the server sends the list of identifiers (by registry, in order) and the client verifies it can handle them; mismatches produce a clear "missing mod/content" disconnect message. In-session protocol uses compact numeric indices from this sync.
- Tags (`#minecraft:logs` style) supported from the start for blocks, items, entities, biomes (data-driven JSON).

### 5.3 Block model
- `BlockId` (registry index) + `BlockStateId` (global dense state index computed from properties at freeze time).
- Properties: bool / int-range / enum, declared at registration.
- Chunk storage: palette-compressed 16×16×16 sections (modern style) with a **configurable world height** (default LCE height, extensible: see section 8). Section-based even though LCE used a 16×16×128 column; sections are an internal representation, not a fidelity change.
- Legacy mapping: `data/legacy/blocks.toml` maps `(legacy_id, legacy_meta) → state`. Used by importer and parity tools; **not** used at runtime.

### 5.4 Data-driven content
- Where Java Edition is data-driven (loot tables, recipes, tags, advancements-equivalent, biome definitions, dimension types), use the same **JSON schemas as Java Edition's data packs where feasible**, so that later parity is cheap and data-pack-style mods work. Where LCE differs, define LCE-specific schema in the `lce:` namespace and document it.
- Behavior code (block logic, entity AI) lives in Rust in `content-vanilla`, registered via the public API.

### 5.5 Save data format
- World saves use an **Anvil-compatible region layout** (`region/r.X.Z.mca`) with NBT chunk data using namespaced IDs and the modern block-state palette layout, plus a top-level `level.dat`-style file carrying: game profile ID, world size preset, seed(s), dimension data, game rules, registry snapshot (for missing-content detection), data version.
- Rationale: maximal future compatibility with Java tooling and Java world import/export.
- Save data must carry a monotonically increasing **`data_version`** per profile and a **DataFixer** pipeline (versioned upgraders) from the start, so older worlds can be migrated when the game advances TU by TU.
- Autosave on a background thread; atomic writes (write temp + rename); crash-safe.

---

## 6. Game Profiles (versioning system)

A **Game Profile** is a named, immutable bundle describing exactly which game version's rules a world uses. Examples: `lce:tu19` (baseline, **[VERIFY]** number), later `lce:tuNN`, `java:1.x.y`.

### 6.1 Contents of a profile
- **Feature flags** (typed enum set): each gameplay mechanic that changed across versions is a flag (e.g. combat model, hunger/regeneration rules, fire spread rules, villager trading model, enchanting model, anvil behavior, elytra, boats, shield, etc.). Mechanic code reads flags, never version numbers.
- **Content availability:** which registry entries exist (blocks/items/mobs/biomes/recipes) in this profile. An entry has `since`/`until` profile bounds so one codebase can serve all profiles.
- **Worldgen module selection:** each profile pins the worldgen implementation versions (terrain, caves, biomes layer stack, structures, decoration order). Old generators are **frozen**: never edited for new features, only for bugfixes explicitly marked as such.
- **Protocol version(s)** supported.
- **Data version** for save migration.
- **UI/Presentation set:** menu/crafting UI variants, default resource set.

### 6.2 Behavior
- New worlds are created *with a selected profile* (default: newest stable profile). The profile ID is stored in the save.
- A world can be **upgraded** to a newer profile via explicit user action (with backup), running the DataFixer chain; downgrade is **not** supported (but creating a new world in an older profile is, which satisfies "play an older version").
- Per-profile **game rule presets** plus user overrides at world creation ("disable new mechanics" = choose older profile or toggle individual feature flags where the owner marks the flag as user-toggleable).
- Servers advertise their profile; clients with matching protocol connect. A client can host/join worlds of any profile it has installed (the game is a single binary containing all profiles).

### 6.3 Code organization for versioned behavior
- `profiles` crate defines `FeatureFlag`, `ProfileId`, `Profile`.
- Worldgen: `worldgen::terrain::{v_tu19, v_tuNN, ...}` modules behind a `TerrainGenerator` trait. Never delete old modules.
- Mechanics: `sim` uses `ctx.profile().has(Feature::X)` at decision points; keep these centralized and documented in `docs/reference/feature-flags.md` (one row per flag: description, introduced-in, source evidence).

---

## 7. Server and simulation

### 7.1 Server architecture
- `server` library exposes `Server::new(config, profile, world_storage, transport(s))`. Runs the tick loop on its own thread(s). Network I/O on async runtime (see section 14); simulation is **not** async.
- Authoritative: server owns world, entities, inventories, RNG, rules. Clients are untrusted (validate everything: movement plausibility, block interaction reach, inventory clicks, crafting results).
- Tick pipeline (per 50 ms tick), in a fixed documented order that is derived from the reference game's order where known: process inbound packets → player tick → entity tick → block updates / scheduled ticks / random ticks → fluids → redstone → weather/time → mob spawning → chunk loading/unloading → outbound packet flush → autosave scheduling.
- Parallelism: chunk-level parallelism for lighting, worldgen, and chunk (de)serialization. Entity/block simulation stays single-threaded and deterministic initially; any later parallelism must preserve determinism.
- Tick-lag handling: server never skips ticks silently; log "can't keep up" with metrics.
- Dedicated server (`mclre-server`): headless binary, `server.toml` config (profile, world size preset, seed, port, max players, host-privilege defaults, view distance, motd), console command input, graceful shutdown with save, no GPU libs linked.
- Host privileges: LCE has no slash commands; implement **host options** (per-player privilege flags: build/mine, use doors, use containers, attack players/mobs, fly, exhaustion, invisibility, teleport, etc. as in the reference) plus a minimal internal command system that powers both the dedicated console and later Java-style commands (so later profiles can expose `/` commands).

### 7.2 Physics and entities
- Implement physics (AABB collision, movement, gravity, friction, step-up, water/lava, ladders, etc.) as a pure, deterministic module shared by server and client prediction.
- Entity framework: **[DECIDE]** between `bevy_ecs` (standalone, no Bevy app) and a custom/`hecs` approach. Recommended: **`bevy_ecs` standalone** for its scheduling and query ergonomics, wrapped behind a thin `sim::entity` API so the choice is replaceable. Entity "classes" (zombie, creeper…) are archetypes + behavior systems + data, registered through the mod API.
- Mob AI: goal/selector system matching LCE's behavior (priorities, navigation, pathfinding semantics). Pathfinding limits are faithful by default.
- Block entities (chests, furnaces, signs, etc.), redstone, fluids, fire, growth/random ticks, TNT/explosions with the **reference algorithm** (important for fidelity).

### 7.3 Lighting
- Sky + block light (0–15) with the reference propagation semantics. Incremental updates on block change; section-based storage; computed off-thread for generation, incrementally on the server for changes. Client receives light data (or recomputes) per protocol decision **[DECIDE]**; recommended: server sends light, client does not simulate it.

### 7.4 Time, weather, game rules
- Day/night cycle, weather, moon phases as reference. Game rules registry with typed values (`keepInventory`, `mobGriefing`, etc., as they exist in the profile).

---

## 8. World model, sizes, storage

### 8.1 World size presets
Implement as `WorldSizePreset` configured at world creation and stored in the save:

| Preset | Intent | Notes |
|---|---|---|
| `classic` | PS3 / Xbox 360 sized | Fixed finite overworld (reference: 864×864 blocks, i.e. 54×54 chunks **[VERIFY]**), with LCE's Nether scale and End layout for that size **[VERIFY]**. |
| `small` / `medium` / `large` | PS4 / Xbox One sized worlds | Reference sizes **[VERIFY: commonly cited 1024 / 3072 / 5120 blocks per side]**; Nether/End scaling per reference. |
| `infinite` | Modern-style "infinite" | Practically unbounded (limited by coordinate range as in Java: ±30,000,000 blocks) with region-file streaming. |

Requirements:
- **v1.0 requirement (owner decision):** both the legacy-size presets (`classic`, `small`, `medium`, `large`) and `infinite` ship in the first full version and are selectable at world creation. The storage, chunk lifecycle, worldgen and networking code must support both from M2 onward; do not build finite-only first and retrofit infinite later.
- Finite presets reproduce the console **edge behavior** (the world boundary, edge-of-world visuals/behavior such as the barrier/ocean-edge treatment, **[VERIFY]**) and the **scaled biome layout** (LCE scaled biome size to fit the world; replicate it exactly by pinning the biome layer scale parameters in the profile's worldgen module).
- `infinite` uses the same generator code with unbounded coordinates and the modern biome scale options where the profile allows.
- Nether/End presets per size are data (`WorldSizePreset::nether_scale`, etc.), not code.
- A **migration path** from finite to infinite worlds may be added later (the world edge is just "not generated yet" for infinite); design chunk storage so that is possible. **[DECIDE]** whether to support it.
- World height is a profile/dimension property (default: baseline profile height, designed to grow with later profiles).

### 8.2 Chunk lifecycle
- States: `Unloaded → Loading → Generated(partial stages) → Lit → Ready → Ticking → Unloading`. Use staged generation (as modern Minecraft) to allow structures/decoration crossing chunk borders without ad-hoc "populate neighbors" hacks, while still **producing byte-identical results to the reference generator** for the baseline profile where feasible. Document any unavoidable divergence.
- View distance and simulation distance are separate settings (defaults emulate LCE).

### 8.3 Dimensions
- Overworld, Nether, The End as dimension types (data-driven: height, light, ambient, fixed time, etc.), registered as `minecraft:overworld`, `minecraft:the_nether`, `minecraft:the_end`.
- Portal linking logic as in the reference with per-preset coordinate scaling.

---

## 9. Identity, DLC entitlements, and launcher hooks (placeholders)

### 9.1 Identity (current requirement)
- Provided by **launch arguments**:
  - `--username <display name>` (validate length/charset per reference rules; reasonable fallback if missing: refuse to start or `Player` + warning — **[DECIDE]**, recommended: refuse in dedicated/multiplayer, fallback `Player` in dev builds).
  - `--uuid <uuidv7>` — validated as UUID **version 7**; reject others with a clear error. This is the player's stable identity for singleplayer saves (player data keyed by UUID) and multiplayer.
- Define a trait in `identity`:
  ```rust
  pub trait IdentityProvider: Send + Sync {
      fn local_players(&self) -> Vec<LocalIdentity>;      // splitscreen: guests derived deterministically
      fn session_proof(&self) -> Option<AuthProof>;        // opaque bytes, None for now
      fn verify_remote(&self, claim: &IdentityClaim, proof: Option<&AuthProof>) -> VerifyResult; // placeholder: Accept
  }
  ```
  Current implementation: `LaunchArgIdentity`. Later implementation: launcher/auth-system-backed.
- **Splitscreen guest identities:** secondary local players need their own UUIDs. Derive deterministically (e.g. UUIDv7-shaped IDs generated and persisted per-install, or UUIDv5 from the primary UUID + slot) **[DECIDE]**; document it. Guests must not collide across installs if they join LAN games.
- The handshake protocol (section 10) must contain: identity claim (uuid + display name), an **optional opaque auth proof field** (empty now), and a server-side "verify" hook (accept-all now). When real authentication arrives, **no protocol-breaking change** should be necessary beyond bumping a minor version.

### 9.2 Entitlements / DLC hook
- Define `EntitlementProvider` trait: `fn owns(&self, content_pack: &Identifier) -> bool`. Current implementation: everything unlocked. Later: launcher-backed.
- DLC/content packs (skin packs, texture packs, mash-up packs, themes, avatar items, etc.) are loaded through the **content pack** system (section 11.4), and gated through `EntitlementProvider`, so the launcher can later control availability without engine changes.

---

## 10. Networking and multiplayer

### 10.1 Requirements
- **Phase 1 (v1.0):** full multiplayer over **LAN** (host from game, join via LAN discovery or direct IP:port, dedicated server on LAN) and **integrated-server splitscreen** (up to 4 local players) mixed with remote players.
- **Phase 2 (later):** UPnP port mapping (`igd-next`), then launcher-mediated relay/NAT traversal ("Steam networking"-like). The transport layer must be abstracted so these are *new transports*, not protocol changes.

### 10.2 Layering
```
Game logic  ⇄  Protocol (typed packets, versioned)  ⇄  Session (handshake, keepalive, encryption)  ⇄  Transport trait  ⇄  {InMemory, LAN-UDP/QUIC, (UPnP), (Relay)}
```
- `Transport` trait: reliable ordered channel(s), reliable unordered, unreliable; connection events; peer address/identity metadata. Implementations: `InMemoryTransport` (integrated server), `QuicTransport` (LAN default; see crates), later `RelayTransport`.
- **Discovery:** LAN discovery via mDNS (`mdns-sd`) and/or UDP broadcast **[DECIDE]**; advertise: server name/MOTD, profile ID, protocol version, player count, world size preset, host display name.

### 10.3 Protocol design
- Own protocol (not LCE's wire format; not Java's yet). Packets are explicit Rust types with hand-written or derive-generated `Encode`/`Decode` (explicit field order and versioning; **avoid pure-serde-derived formats for the wire** so versioning is controllable).
- States: `Handshake → Login → Configuration (registry sync, profile/ruleset, resource/content-pack info) → Play`. Deliberately mirrors modern Java protocol structure to ease the future Java compat layer.
- Compression (zstd or lz4) above a size threshold; optional encryption (QUIC/TLS provides it on the QUIC transport; the in-memory transport needs none). Self-signed certs for LAN with trust-on-first-use or ignore-cert for LAN mode **[DECIDE]**.
- Chunk data on the wire: palette-compressed sections + light data + block entities; delta updates for block changes (batched per chunk-section per tick).
- Entity sync: spawn/despawn/relative move/teleport/metadata/equipment/effects/animations; server-authoritative with client interpolation; client-side prediction for the local player(s) with server reconciliation.
- Versioning: the handshake carries `protocol_version` + `profile_id`; mismatch → structured disconnect reason. Keep protocol definitions in `protocol::v{N}` modules so old versions can be retained when necessary.
- **Java compat plan (long-term, do not build now):** a separate crate `protocol-java` implementing a `ProtocolAdapter` that translates Java packets ↔ internal game events. This is why game logic must communicate through internal events/types and not directly with wire packets. Keep `server` talking to an abstract `ClientConnection` that yields typed *internal* messages; the native protocol is simply the first adapter.

### 10.4 Splitscreen
- Up to 4 local players on one client process, each with: own viewport, HUD, input device assignment, inventory/crafting UI instance, camera, and its own server-side player entity/connection (via the in-memory transport; each local player is a distinct connection to the integrated server).
- Renderer must support N viewports per frame from the start (scissor/viewport + per-viewport view/projection uniforms; shared chunk mesh data). Layouts as reference (horizontal/vertical split, quad) plus optional modern layout choices.
- Shared-world resources (chunk meshes, texture atlas) are built once and drawn per viewport.

### 10.5 Anti-cheat/robustness baseline
- Server-side validation for movement speed/teleport/fly rules (per host privileges), block reach/line of sight, interaction cooldowns, inventory transaction validation, packet rate limits, bounded packet sizes, no panics on malformed input (fuzz tests for the decoder).

---

## 11. Assets, content packs, localization, audio

### 11.1 Modern asset formats
| Purpose | Format |
|---|---|
| Textures | PNG (authoring/import), runtime packed to GPU formats (BCn/ASTC where supported, else RGBA8); KTX2 allowed for pre-compressed; mipmaps generated at load |
| Block/item models | JSON models + blockstate JSON in the **Java resource pack style** (parent models, elements, faces, variants/multipart). Gives resource-pack and later Java-parity compatibility |
| Audio | **Ogg Vorbis** for music/long sounds, short effects as Ogg or WAV; Opus optional later |
| Fonts | TTF/OTF + bitmap font support (LCE-style pixel font replicated or rebuilt from owner-supplied assets) |
| Localization | JSON language files keyed by `namespace:key` or `namespace.key` strings (modern style), converted from LCE's language files by `assetc` |
| UI layouts | Data files (RON/JSON) describing layout trees, styles, and animations (see 12.4); no Flash/SWF runtime |
| Structures/worldgen data | NBT structure files and JSON where applicable |
| Archives/packs | Directory or ZIP, with a `pack.toml`/`pack.json` manifest |

### 11.2 Asset pipeline
- `assets` crate: virtual filesystem layering (base game assets → content packs → resource packs → mod assets → user overrides), hot-reload in dev.
- **Atlas:** build a block texture atlas or **texture array** at load; choose per GPU capability and low-end settings. Animated textures (water, lava, fire, portal) per reference timings.
- **Resource pack compatibility:** Java-style resource packs load for textures/models/sounds/lang where equivalent features exist (start with the basics; expand per profile).

### 11.3 `assetc` — importer for the owner's LCE files
- CLI tool that takes the path to the original asset source (by default the reference tree's assets, `reference/lce-141217/`, section 3.5; any extracted LCE dump also works) and produces the engine's asset directory (`game-assets/`):
  - Extract container archives (e.g. `.arc`/pack files **[VERIFY formats]**).
  - Convert textures (DDS/other → PNG/KTX2), audio (`.binka` → Ogg Vorbis; `.binka` decoding can be delegated to an external decoder such as `vgmstream` invoked by the tool, **[DECIDE]** vs. bundling), language files, UI source assets as needed.
  - Emit a manifest recording source hashes and tool version; idempotent and resumable.
- **Current policy (owner decision): the converted original game assets live in the private repo** to keep development simple. Rules that keep this cheap to change later:
  - All original/derived proprietary assets go in **one isolated top-level directory**, `game-assets/` (converted output of `assetc`, plus the unmodified source dump under `game-assets/source/` if the owner wants it tracked). Nothing proprietary is ever placed inside `crates/` or `docs/`, and no asset is embedded into source files (`include_bytes!` of proprietary files is not allowed; load from the asset directory at runtime).
  - Use **Git LFS** (or an equivalent) for `game-assets/` so normal clones and diffs stay fast **[DECIDE]**.
  - `assetc` stays in the repo and must remain runnable and reproducible, so `game-assets/` can be regenerated from a user's own dump and later removed from the repo without code changes.
  - Keep a small set of **open placeholder assets** (`assets-placeholder/`, CC0/self-made) so CI can run without proprietary content and the engine's "assets missing" path stays tested.
  - The `app` first-run flow may default to `game-assets/` and still supports pointing at another asset directory.

### 11.4 Content packs (skin packs, texture packs, mash-ups, DLC)
- A **content pack** = ZIP/dir with `content-pack.toml` manifest: `id` (`namespace:path`), version, type(s) (skins, textures, theme, mashup world, music, avatar items…), dependencies, icons, localized names/descriptions.
- Loader mounts packs into the asset VFS; world templates (mash-up worlds) are importable as new-world templates.
- Gated through `EntitlementProvider` (section 9.2).
- Document the format in `docs/modding/content-packs.md`. A converter from LCE DLC formats is a later `assetc` feature.

### 11.5 Audio engine
- Crate recommendations in section 14. Requirements: positional 3D sound with LCE-matching attenuation, per-player listener for splitscreen (mix N listeners sensibly: each sound is spatialized for the nearest/relevant listener(s) — **[DECIDE]** policy; reference likely plays to all with per-player attenuation), music playback with LCE's selection/timing logic, sound events from registry `minecraft:`/`lce:` sound events mapped to files via JSON (`sounds.json`, Java-like).
- Audio thread isolated; failure to initialize audio must not crash the game.

### 11.6 Skins (post-v1.0 but design now)
- Skin model abstraction: player model as data (classic 4px arms / slim 3px arms, 64×64 and legacy 64×32 formats), skin texture sourced via `SkinProvider` trait. Providers: LCE-style packs, local library, later Java-style URL/profile providers.
- Skin library & picker UI (list, preview, import PNG, choose model variant) — milestone after v1.0.
- Skin data travels in the Configuration/Play handshake (hash + bytes, size-limited, cached by hash).

---

## 12. Client, rendering, UI, input

### 12.1 Client structure
- `client` library: owns `ClientWorld` (mirror of server-sent chunks), local player(s) state, entity interpolation, prediction, UI state machine, and talks to the server via `ClientConnection` (protocol-only).
- `app` runs the winit event loop; frame loop decoupled from the 20 TPS client-side tick (interpolate with partial ticks as the reference does).

### 12.2 Renderer (wgpu)
- **Backends:** request backends in this order on Windows: Vulkan → DX12 → GL, selectable via setting/CLI `--gpu-backend`. Linux: Vulkan → GL. macOS: Metal. Keep code free of backend-specific assumptions; test on at least Vulkan and DX12 (and GL as the low-end fallback) in CI where possible. Design toward optional WebGPU/wasm later (don't depend on features unavailable there for core paths; gate optional ones behind capability checks).
- **Feature negotiation:** query adapter limits/features; define a `GpuCaps` struct; optional paths (multi-draw-indirect, texture arrays/bindless-ish arrays, compute-based culling, BC/ASTC) with guaranteed simple fallbacks for lower-end/GL.
- **Render graph:** an explicit pass-based frame graph. Pass names and resource semantics are designed to **map onto Iris/OptiFine shaderpack program stages** (see section 13): shadow, gbuffers (terrain, water, entities, block entities, hand, clouds, sky, weather, particles, etc.), deferred, composite, final. Even before shader packs exist, structure the vanilla renderer as: shadow(optional) → gbuffer-like opaque pass → translucent pass → composite/post → UI, so the shaderpack work later doesn't require a rewrite.
- **Chunk meshing:**
  - Meshing on worker threads (rayon or custom pool), per 16³ section, from palette data + neighbor borders.
  - Face culling, per-vertex AO / **LCE smooth lighting** replicating the reference look; biome tinting (grass/foliage/water) via per-vertex color or biome-tint lookup texture.
  - Separate opaque / cutout / translucent geometry; translucent sorting as reference (per-chunk back-to-front sections).
  - Compact vertex format (packed positions, normals/face index, UVs via atlas index, light, tint).
  - Incremental remeshing on block updates, with priority queue (near player first), frame-time budgeting for uploads (e.g. N MB/frame).
  - Optional greedy meshing **disabled by default** if it harms fidelity of smooth lighting/AO; **[DECIDE]** after profiling.
- **Culling:** frustum culling per section, plus simple occlusion culling (e.g. cave culling/"visibility graph" as in modern Minecraft) as an option for low-end.
- **Sky, fog, clouds, weather, stars, sun/moon, particles, entity models (skeletal part-based like Minecraft's model system), item rendering (inventory 3D icons, held items, dropped items), block entities (chests, signs, banners if present in profile), first-person hand** — all per reference visuals.
- **Settings for low/mid hardware:** render distance, fancy/fast graphics, smooth lighting on/off, cloud quality, particle quality, texture resolution cap, mipmap level, entity distance, max FPS, resolution scale, MSAA off by default. Profile to meet budgets in 12.5.
- **Splitscreen:** per-viewport camera and UI scaling (section 10.4).

### 12.3 Input
- `input` crate abstracts keyboard/mouse/gamepad (`winit` + `gilrs`) into **actions** (jump, sneak, attack, use, inventory, crafting, next/prev item, etc.) with per-player bindings; default gamepad layout replicates LCE's console controls (including LB/RB hotbar scroll, crafting tabs, quick-move behavior, pointer-less inventory navigation) and a KB+M layout. Menu navigation must work fully with gamepad (D-pad/focus navigation), mouse, and keyboard.
- Multiple local players map to different devices (up to 4 controllers, or 1 KB+M + controllers).

### 12.4 UI
- LCE's UI was authored in a Flash-based middleware; **do not** attempt to run SWF. Build a **retained-mode, data-driven UI toolkit** in `ui` using wgpu (and a text crate; see section 14) that can reproduce the LCE look & feel: panel/9-slice styles, focus-based navigation, scene stack (title → load/create world → game; pause menu; inventory; crafting; furnace; enchanting; anvil; trading; host options; settings; help & options; texture/skin pack menus; leaderboards-stub; achievements/trophies).
- Layout/style described in data files (RON/JSON) so UI is themeable and modifiable by content packs/mods; localization-ready; scales across resolutions and splitscreen viewports (each viewport has its own UI scale + safe area).
- Menu flow, sounds, and animation timings follow the reference (use captured video for comparison).
- `egui` may be used **only** for developer tooling (debug overlay, profilers, registry browser), never for shipping game UI.
- Accessibility baseline: UI scale, subtitles option, controller remap, color-blind-friendly options as time allows.

### 12.5 Performance budgets (target)
Define a baseline "low" and "mid" hardware profile in `docs/perf.md`; suggested defaults to confirm with the owner **[DECIDE]**:
- **Low:** integrated GPU (e.g. Intel UHD 620 class), 4 core CPU, 8 GB RAM → 1080p/720p at ≥ 30 FPS, render distance equivalent to LCE default.
- **Mid:** GTX 1060 / RX 580 class → 1080p at ≥ 60 FPS at render distance 12–16 chunks.
- Server: idle-world tick ≤ 5 ms; with 4 players spread out ≤ 25 ms (p99 ≤ 45 ms).
- Memory: client ≤ 2 GB at default settings.
- Continuous benchmarking (`criterion` + a scripted flythrough benchmark) gated in CI as non-blocking trend reports first.

---

## 13. Shaders (mid-term, Iris integration) — design now, build later

**Feasibility note (important):** Iris is a Java mod licensed under **LGPL-3.0** (verify current license) and is written against OpenGL via Minecraft's Java renderer; its shaderpack support follows the **OptiFine/Iris shaderpack format** (GLSL programs such as `gbuffers_terrain`, `composite`, `final`, `shadow`, plus `shaders.properties`, `block.properties`, uniforms and `#include`/option preprocessing). It **cannot be "integrated" as code** into a Rust/wgpu engine. The realistic approach is to **re-implement the shaderpack pipeline** in Rust, using Iris (and its documentation/format spec) as the compatibility reference, respecting its license if any code or substantial structure is ported (**[DECIDE]** with the owner: clean-room reimplementation from format docs vs. derivative work under LGPL).

Known hard problems to plan for (record them as ADRs when work begins):
1. **GLSL dialect:** shaderpacks use desktop GLSL (commonly `#version 120–460`, compatibility-profile built-ins like `gl_ModelViewMatrix`, `gl_Vertex`, `gl_MultiTexCoord`, `attribute/varying`). wgpu consumes WGSL / SPIR-V / limited GLSL (via `naga`, which supports a restricted core-profile GLSL). A **translation layer** is required: preprocess (`#include`, `#define`, option macros) → compatibility-profile lowering to modern GLSL with injected uniforms/attributes → compile to SPIR-V (e.g. `shaderc`/`glslang` via a Rust binding, or `naga` where it suffices) → feed wgpu. This is the single largest risk; spike it early (**M-Shader-0**) with a few popular packs before committing to the timeline.
2. **Backends:** SPIR-V passthrough works on Vulkan; on DX12/GL/Metal, wgpu/naga must cross-compile — validate coverage of the constructs shader packs use (image load/store, SSBOs, integer ops, derivatives, `texelFetch`, shadow samplers, MRT up to 8 drawbuffers, etc.).
3. **Buffer/format semantics:** `colortex0–15`, `shadowcolor`, `depthtex0/1/2`, custom textures, buffer flipping, mip-mapped `colortex`, `RENDERTARGETS` directives, `drawbuffers`, per-pass `scale/size` properties.
4. **Uniform set:** hundreds of uniforms (camera/time/weather/biome/etc.) — build a data-driven uniform provider keyed by name.
5. **Block/item/entity ID mapping:** shaderpacks reference blocks by numeric IDs via `block.properties`; with namespaced IDs, support the `namespace:path` syntax used by modern Iris (`block.properties` already supports namespaced names) and generate numeric `mc_Entity` IDs per pack at load.

**What to do now to keep this possible (cheap):**
- Renderer uses the staged render graph (12.2), vertex attributes that can supply `mc_Entity`-like block IDs, normals, tangent data, per-vertex light, and midBlock-style data when needed.
- Keep the renderer's pass and resource definitions data-described, so a shaderpack can later replace the pass list.
- Vanilla shaders written in WGSL with structure mirroring `gbuffers_*` programs.

---

## 14. Tech stack (crates) — proposals for discussion

> Default rule: prefer well-maintained, widely used crates; pin versions in `Cargo.lock`; justify every new dependency in the PR/commit message; run `cargo-deny` (licenses: allow MIT/Apache-2.0/BSD/ISC/Zlib/MPL-2.0 (verify policy), flag GPL/LGPL); run `cargo-audit` in CI.

| Area | Proposed | Alternatives / notes |
|---|---|---|
| Graphics | **`wgpu`** (owner-mandated) | Use `naga` for WGSL; `wgpu` backend env/flags for selection |
| Windowing | **`winit`** | `raw-window-handle` comes via wgpu/winit |
| Math | **`glam`** | `bytemuck` for GPU buffer POD casts |
| ECS (server entities) | **`bevy_ecs`** standalone **[DECIDE]** | `hecs`, `legion`; or custom |
| Async / networking runtime | **`tokio`** for network I/O only | Simulation loop on dedicated OS thread(s) |
| LAN transport | **`quinn`** (QUIC, TLS built-in, multiple streams + datagrams) **[DECIDE]** | `renet`/`laminar`/custom UDP reliability if QUIC overhead or cert handling is a problem |
| LAN discovery | **`mdns-sd`** + optional UDP broadcast | |
| UPnP (later) | **`igd-next`** | |
| Serialization (configs) | **`serde`** + `toml`, `serde_json`, `ron` | |
| Wire protocol | custom `Encode/Decode` (hand-rolled/derive macro) | `bytes` crate for buffers; avoid serde for wire |
| NBT | **`simdnbt`** or **`fastnbt`** **[DECIDE]** | Must read/write Java-compatible NBT |
| Region storage | custom Anvil implementation in `world` | Keep dependency-free for the server |
| Compression | `zstd`, `lz4_flex`, `flate2` (zlib for Anvil) | |
| Images | `image` (+ `ddsfile`/`texpresso` for converters), `ktx2` | |
| GPU texture compression | `basis-universal` or offline BCn encode **[DECIDE]** | optional |
| Text rendering | `glyphon` (cosmic-text) or `fontdue` + custom atlas **[DECIDE]** | Bitmap font path must exist for the pixel look |
| Audio | **`kira`** or **`rodio`** (+ `symphonia` / `lewton` for Ogg) **[DECIDE]** | Need 3D positioning/multi-listener → `kira` likely better; verify |
| Input | `winit` events + **`gilrs`** | |
| Parallelism | `rayon` (worldgen/meshing), `crossbeam` channels, `parking_lot` | |
| RNG | custom reimplementation of **Java `Random` (LCG)** for fidelity + `rand` for non-gameplay | Worldgen fidelity depends on exact `java.util.Random` semantics (including `nextInt(bound)` rejection logic, `nextGaussian`); implement and unit-test against known Java outputs |
| Noise | custom ports of the reference Perlin/Simplex/Improved noise | Do not substitute generic noise crates for baseline worldgen (changes output) |
| Mods (sandbox) | **`wasmtime`** (WASM component model) **[DECIDE]** | `wasmer`; embedded scripting (`rhai`, `mlua`) as optional secondary layer |
| UUID | `uuid` (with `v7` feature) | |
| Logging/metrics | `tracing`, `tracing-subscriber`, `tracing-tracy` (optional) | |
| Errors | `thiserror` (libs), `anyhow` (bins/tools) | |
| CLI | `clap` | |
| Testing | `proptest`, `insta` (snapshots), `criterion`, `cargo-fuzz` | |
| Dev UI | `egui` (+ `egui-wgpu`) behind a `dev-ui` feature | |
| Hashing | `ahash`/`rustc-hash` for hot paths; deterministic ordered maps where iteration order matters (`indexmap`, `BTreeMap`) | |

**Unsafe policy:** `#![forbid(unsafe_code)]` by default in all crates except explicitly allowed ones (`render` for GPU interop if needed, `core` for interning if needed); each `unsafe` block needs a `// SAFETY:` comment and a test.

---

## 15. Modding support

Modding is a **first-class design constraint from M1**, not a late addition.

### 15.1 Layers
1. **Data packs & resource packs** (no code): JSON/NBT data (recipes, loot, tags, biomes, structures, dimension types, models, textures, sounds, languages). Java-compatible shapes where possible.
2. **WASM mods (code):** sandboxed plugins compiled to WASM (component model with WIT interfaces **[DECIDE]**), using the `mod-api` SDK crate (guest side). Capabilities: register blocks/items/entities/recipes/sounds/etc., subscribe to events, implement block/entity behavior callbacks, add commands, add UI screens (data-driven + callbacks), custom packets over a mod channel.
3. **Native Rust mods (dev-only, optional):** a trusted, in-process mode for first-party tooling and for performance-critical mods, behind a feature flag and explicit user confirmation; ABI stability not promised. **[DECIDE]** whether to include at all.

### 15.2 Mod package
- `mod.toml`: `id` (namespace), `name`, `version` (semver), `authors`, `license`, `api_version` (semver range), `side` (`common` | `server` | `client`), `dependencies` (`id`, version range, optional/required), `entrypoint(s)`, `permissions` (sandbox capabilities requested), assets/data directories.
- Load order via dependency resolution (topological), deterministic; clear errors for cycles/missing deps.

### 15.3 Mod API requirements
- **Registration API** identical to what `content-vanilla` uses (pillar 5).
- **Event bus** with typed events and priorities, cancellable events where meaningful (block break/place, entity damage, player join/leave, chat, tick start/end, chunk load/gen, explosion, item use, container click, etc.).
- **Server/client side split** enforced by the API types (client-only mod cannot touch authoritative state).
- **Network:** custom mod channels (namespaced), registry-sync-aware (clients without required server mods get a clear message listing what's missing; optional "server-only mod" mode where no client install is needed if the mod only uses vanilla-compatible content).
- **Sandbox/limits:** fuel/time budgets per tick, memory caps, no filesystem/network access unless a permission is declared and granted, deterministic host functions (no wall-clock for simulation-affecting code).
- **API versioning:** semver; deprecation policy; compatibility shims per profile where API semantics differ. The API must be profile-aware (a mod can query active profile/feature flags).
- **Hot reload** of data/resource packs in dev; WASM mod reload in dev builds.
- **Docs & examples:** `docs/modding/` with a "hello block" tutorial, an example mod crate in `examples/`, and an auto-generated API reference.
- The first mod milestone is the **vanilla content crate itself** + one tiny example mod (new block + recipe) as the acceptance test.

---

## 16. Legal, licensing, and repository hygiene

- **Source policy (current owner decision):** the original source tree is available as the behavioral reference (section 3.5). The previous clean-room policy is **superseded**. Consequences: the project is a derivative of the original, which is acceptable only because it stays private (below). Engineering rules that remain: the reference tree stays a separate read-only submodule; no reference file is copied into `crates/`; the Rust code is an idiomatic reimplementation shaped by this spec's architecture, not a mechanical translation. Record this decision in `docs/adr/` so it can be revisited.
- **Assets (current owner decision):** original game assets are included in the private repo for now, isolated in `game-assets/` (section 11.3). The **code** must still be an independent reimplementation (no copied/decompiled source). CI uses only `assets-placeholder/` so it never needs proprietary content. Keep the repository **private**: no public remotes, forks, release artifacts, container images, or CI artifacts containing `game-assets/`.
- Naming: the project is named Minecraft: Legacy Rust Edition (MCLRE) by owner decision. Since it is private and not distributed, the use of the "Minecraft" name is not a current concern; **revisit the name before any distribution** **[DECIDE]**.
- **Distribution scope (stated by the owner):** this is a strictly local, private project for a small group of people who have already purchased the game and its DLC. Nothing will be sold or mass-distributed. Engineering consequences: (a) for now, assets and DLC are included in the private project as-is (the owner has explicitly decided to defer legal questions); (b) the isolation rules in section 11.3 keep a later switch to "each participant imports from their own copy" a small change; (c) content packs stay separable and entitlement-gated (section 9.2), so the launcher can unlock packs per participant later. If the scope ever widens beyond this group, or the repo or builds ever leave private hands, revisit this section before distributing anything.
- Own code license: **[DECIDE]** (suggest MIT/Apache-2.0 dual-license, or a copyleft license if Iris-derived code will be used, see section 13).
- Dependency license checks via `cargo-deny` in CI.

---

## 17. Milestones and acceptance criteria

Each milestone ends with a tagged commit, passing CI, and updated docs.

**M0 — Foundation**
- Workspace, CI (fmt, clippy `-D warnings`, tests, `cargo-deny`, `cargo-audit`) on Windows + Linux, `CLAUDE.md`, ADR template, logging, error conventions, `rust-toolchain.toml`.
- *Accept:* `cargo test --workspace` passes; dependency-rule test fails if `server` pulls in `wgpu`.

**M1 — Core data model**
- `Identifier` (+ interning, validation), registry system with freeze + tags, block/state model, Java-`Random` port with tests against known Java outputs, deterministic noise primitives, NBT read/write, profile system skeleton (`lce:tu19` stub).
- *Accept:* register ~20 test blocks through the public API; property-based tests for state indexing; RNG output and seed-derivation vectors match the reference implementation (section 3.6); `GameRandom`/`CosmeticRandom` split in place with the CI check that gameplay crates do not depend on `rand`.

**M2 — World storage & chunks**
- Palette-compressed sections, light storage, region (Anvil) read/write, DataFixer scaffolding, async chunk IO, world save metadata (profile, size preset, seed, registry snapshot). Must work for both bounded (legacy-size) and unbounded (infinite) coordinate spaces.
- *Accept:* write → read round-trip for random chunks, crash-safety test (kill during save), Java-tool can open a generated region (manual check).

**M3 — Minimal client rendering**
- `app` window, wgpu init with backend selection + `GpuCaps`, texture atlas from placeholder assets, chunk meshing (flat test world), camera, frustum culling, debug overlay (`egui`).
- *Accept:* render a 32-chunk-radius flat world ≥ 60 FPS on the "mid" profile; backends Vulkan/DX12/GL all start (GL may be slower).

**M4 — Server loop, protocol, integrated server, player movement**
- Server tick loop, in-memory transport, protocol v1 (handshake/login/config/play subset), registry sync, chunk streaming, player entity with physics (shared prediction module), block break/place, reach validation, identity launch-args + `IdentityProvider`.
- *Accept:* singleplayer flat world playable: walk/jump/swim/fly, place/break blocks; same code path as a remote connection (integration test with a headless client).

**M5 — Faithful core gameplay**
- Vanilla content (baseline profile): blocks, items, tools, block hardness/drops, inventory, **LCE-style crafting menu & recipes**, furnace, chests, health/hunger, damage, death/respawn, day/night, lighting, fluids, fire, TNT, redstone basics, XP.
- *Accept:* golden-data tests for recipes/hardness/drops pass against owner-supplied tables; parity scenarios for crafting/furnace/fluids pass.

**M6 — World generation (baseline)**
- Frozen baseline generators: biome layers (with finite-size scaling), terrain, caves/ravines, structures (villages, strongholds, mineshafts, temples, Nether fortress, End), decoration, ores, Nether & End dimensions, portals.
- *Accept:* for owner-supplied reference seeds, heightmap/biome maps match reference dumps within an agreed tolerance (target: exact for terrain/biomes); visual side-by-side signoff.

**M7 — Mobs & AI**
- Entity framework finalized, passive/hostile mobs, spawning rules, AI goals, pathfinding, combat per baseline profile, animals/breeding, villagers (as in profile), bosses (Ender Dragon, Wither if in baseline), projectiles, item entities, XP orbs.
- *Accept:* behavior parity scenarios; server perf budget met with N mobs.

**M8 — UI/UX parity pass 1**
- Title screen, create/load world (incl. profile + world size + seed + host options), pause, inventory/crafting/furnace/chest/enchant/anvil/brewing/trading screens, HUD, settings, controls remap, gamepad navigation.
- *Accept:* full menu flow playable with gamepad only and KB+M only; visual comparison sign-off against reference captures.

**M9 — LAN multiplayer + dedicated server**
- QUIC transport, mDNS discovery, join by IP, host options/privileges, player list, chat, entity sync + interpolation, `mclre-server` headless build, server config, graceful shutdown.
- *Accept:* 4 remote players on LAN for 1 hour soak test; fuzz tests for packet decode; dedicated server has no GPU dependencies (checked in CI).

**M10 — Splitscreen**
- Up to 4 local players with viewports, per-player UI/HUD/inputs, mixed with LAN players.
- *Accept:* 4-player splitscreen stable at "mid" profile ≥ 30 FPS; per-player inventories/crafting independent.

**M11 — Audio**
- Audio engine, sound events, music logic, positional audio, splitscreen listeners.
- *Accept:* parity checklist for key sound events; audio init failure non-fatal.

**M12 — Modding v1**
- Mod manifest/loader, WASM host + `mod-api`, registration + events, data/resource packs, mod channel networking, example mod + docs.
- *Accept:* example mod (block + recipe + event handler) loads on server and client; mismatched mod set produces a readable disconnect message; sandbox limits tested with a hostile mod.

**M13 — World size presets & persistence polish**
- `classic`, `small/medium/large`, and `infinite` fully working, edge behavior, Nether/End scaling, autosave, backups, world upgrade tooling skeleton.
- *Accept:* each preset creates/loads/saves correctly; infinite world exploration 10,000+ blocks in each direction without leaks or hitches beyond budget.

**M14 — Tutorial world, stats, polish → v1.0**
- Tutorial world, achievements/trophies equivalents, stats, remaining baseline-profile features, performance pass, crash handling (panic hook + report file), packaging (Windows installer/zip), first-run asset import flow.
- *Accept:* full play-through checklist of the baseline profile; no known save-corrupting bugs; v1.0 tag.

### Post-1.0 roadmap (ordered, flexible)
1. Java-style skin support, skin library & picker (classic/slim).
2. LCE world importer (`lce-import`: legacy save format + legacy block ID/meta → block-state mapping; entity/block-entity NBT mapping; size-preset detection).
3. DLC/content-pack converters, launcher entitlement integration, authentication integration, UPnP, launcher-mediated relay networking.
4. Shaderpack support (section 13): spike → pipeline → compatibility testing with popular packs.
5. **TU-by-TU upgrade train:** for each TU: add its changes as a new profile (`lce:tuNN`), new content `since` bounds, feature flags, new frozen worldgen modules if needed, DataFixers, parity tests. Keep every previous profile playable.
6. **Java parity track:** per Java release: add profile `java:x.y.z`, `protocol-java` adapter progress, mechanics under flags; target compatibility with current Java Edition servers/clients incrementally.

---

## 18. Engineering conventions

- **Code quality:** `rustfmt` default, `clippy` with `-D warnings` (pedantic selectively), docs on all public items (`#![warn(missing_docs)]` for library crates).
- **Errors:** no `unwrap()`/`expect()` in non-test, non-startup code paths on untrusted input (network/save data). No panics from malformed packets or corrupt chunks: corrupt data → logged + recovery path (regenerate or skip with warning, never silent loss without backup).
- **Testing pyramid:** unit tests (pure logic), property tests (state indexing, codec round trips), integration tests (headless server + headless client over in-memory and loopback QUIC), parity tests (scenario YAML), fuzz targets (packet decoder, NBT/region reader, mod manifest parser), benchmarks (meshing, chunk gen, tick).
- **Headless everything:** the headless client (no window) must exist early for CI integration tests.
- **Determinism tests:** same seed + same inputs → identical world hash across runs and platforms (CI matrix Windows/Linux).
- **Commits:** small, focused, conventional-commit style; each PR updates docs/ADRs if architecture changes.
- **Docs:** `docs/reference/` stores *behavioral* findings (what the original does, with evidence references), separate from design docs.
- **Telemetry:** none. Crash reports local only.
- **Config:** settings in `options.toml` (client) and `server.toml` (server), versioned with migration; CLI args override.

---

## 19. CLI (initial)

Client (`mclre`):
```
--username <name>          Display name (required unless --dev-defaults)
--uuid <uuidv7>            Player identity (must be UUID v7)
--game-dir <path>          Data directory (saves, options, packs, mods)
--assets <path>            Asset directory (output of assetc)
--gpu-backend <vulkan|dx12|gl|metal|auto>
--connect <host[:port]>    Join directly
--profile <id>             Default profile for new worlds (e.g. lce:tu19)
--dev-defaults             Dev only: auto-generate identity
--log <filter>             tracing filter
```
Server (`mclre-server`):
```
--config <path>   --world <path>   --port <n>   --profile <id>   --world-size <classic|small|medium|large|infinite>   --seed <n>   --headless-console
```
Both must log the effective identity/profile/config at startup (without secrets).

---

## 20. Open decisions (collect answers in ADRs)

1. Final choices: ECS (`bevy_ecs` vs other), transport (`quinn` vs other), audio crate, text rendering crate, NBT crate, WASM runtime/ABI.
2. Namespace policy for LCE-only content (`lce:` vs `minecraft:`).
3. Exact reference TU/CU for the baseline and final Xbox One/PS4 target; exact world size numbers and edge behavior.
4. Splitscreen guest identity derivation; audio policy for multiple listeners.
5. Light data: server-sent vs client-computed.
6. Whether finite → infinite world conversion is supported.
7. Clean-room vs derivative-work policy for shader pipeline (Iris is LGPL); overall project license.
8. Native (non-WASM) mod support.
9. Low/mid hardware definition and perf targets.
10. Product/binary naming relative to trademarks; legal position on distributing DLC.
11. `faithful_bugs` default policy list (which LCE bugs are preserved).

---

## 21. First tasks for Claude Code (kick-off checklist)

0. **Survey the reference tree first** (section 3.5): add it as a read-only submodule at `reference/lce-141217/`, inventory `Minecraft.World` and `Minecraft.Client` (module list, build system, asset locations, data tables), and write `docs/reference/reference-tree.md` with the verified layout and a confirmed crate mapping. Resolve the `[VERIFY]` items in this spec (exact TU, world size numbers and edge behavior, asset archive formats) from what it shows, and update the spec accordingly.
1. Create the workspace and crate skeletons from section 4; add CI and the dependency-rule test.
2. Write `CLAUDE.md` (condensed rules) and `docs/adr/0000-template.md`, `docs/adr/0001-record-architecture-decisions.md`.
3. Implement `Identifier`, registries, tags (M1) with exhaustive tests.
4. Implement the Java `Random` port and test vectors; stub noise module.
5. Implement block state model + palette sections; Anvil read/write (M2).
6. Create ADRs for each **[DECIDE]** in section 20 with a recommendation and ask the owner to confirm before M3/M4 depend on them (ECS, transport, audio, text, NBT).
7. Proceed to M3 only after M0–M2 acceptance criteria are met.