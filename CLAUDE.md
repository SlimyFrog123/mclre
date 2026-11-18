# CLAUDE.md — MCLRE Working Rules (Condensed)

> Condensed from `SPEC.md` (§2, 3.5, 3.6, 4, 15, 16, 18). Read `PLAN.md` + `PROGRESS.md` at session start; this file is the evergreen rulebook.
> Project: **Minecraft: Legacy Rust Edition (MCLRE)** — clean-architecture Rust reimplementation of LCE (baseline build: Dec 17, 2014, profile `lce:141217`).

## Non-negotiable architecture (SPEC §2)
1. **Strict client/server separation.** Server compiles with zero graphics/audio/windowing/UI deps. Singleplayer = integrated server over the *same protocol* via in-memory transport. No singleplayer shortcuts.
2. **Namespaced IDs everywhere.** `namespace:path` (default ns `minecraft`; `lce:` for LCE-exclusives). Numeric IDs only in protocol palettes, registry indices, legacy-mapping tables.
3. **Block states, not metadata.** `(block, typed properties)` → dense `BlockStateId`; legacy `(id, meta)` mapping only in generated `data/legacy/*.toml`.
4. **Profiles, not version ifs.** Version behavior = `Game Profile` feature flags + versioned worldgen modules. Mechanics read `ctx.profile().has(Feature::X)`. Old worldgen modules frozen once shipped.
5. **Vanilla is a mod.** Base content registers via the same public API mods use.
6. **Deterministic fixed-timestep sim.** 20 TPS; `GameRandom` (reference-exact Java Random) for ALL gameplay/worldgen; `CosmeticRandom` for client visuals only. Gameplay crates must NOT depend on `rand`. No HashMap iteration order affecting RNG; reproduce reference call order/count exactly.
7. **Modern formats in engine; converters at the edge** (`tools/assetc` → `game-assets/`).
8. **Console limits are presets, not hard-codes** (world size, render distance, entity caps).
9. **Performance is a feature** (budgets in `docs/perf.md`, from M3).

## Fidelity & reference (SPEC §3.5/3.6)
- Reference: `reference/lce-141217/` — **read-only submodule. Never edit, compile, or copy files from it into `crates/`**. Port behavior behind our abstractions; idiomatic Rust, not transliteration.
- Key verified facts live in `docs/reference/baseline.md` (world sizes 54/64/192/320 chunks; Nether scale 3/3/6/8; End fixed 18×18; edge moats), `docs/reference/rng.md` (exact Java Random port + 65536-entry sine LUT).
- **Definition of Done per gameplay feature:** (a) `docs/reference/<feature>.md` cites reference files/functions; (b) RNG call order/count verified; (c) parity test passes; (d) deviations logged in `docs/reference/discrepancies.md` and mapped to SPEC §3.3 or a `faithful_bugs` flag.
- Ambiguity: write an ADR (`docs/adr/`), flag to owner, proceed least-irreversibly. Never guess silently.

## Workspace & dependency rules (SPEC §4)
- Crates under `crates/`: `core, profiles, content-vanilla, world, worldgen, sim, protocol, net, server, server-bin, client, render, ui, audio, input, assets, mod-api, modding, identity, app, tools/{assetc,devtools}`.
- **Forbidden deps:** `core/world/worldgen/sim/protocol/server/profiles/content-vanilla` → no `wgpu/winit/audio/UI`; `core/world/worldgen/sim/content-vanilla` → no `rand`; `client` → no `sim`/`server`; `server-bin` → no GPU libs. Enforced by dependency-graph test (`crates/tools/devtools`) + `cargo-deny`.
- Only `app` wires client + integrated server.
- Workspace lints: `#![forbid(unsafe_code)]` (grant exceptions crate-by-crate with `// SAFETY:`), `missing_docs = "warn"` on libs, `clippy -D warnings` in CI, edition 2024, MSRV pinned in `rust-toolchain.toml`.

## Engineering conventions (SPEC §18)
- No `unwrap()`/`expect()` on untrusted input (network/save data); corrupt data → log + recover, never panic.
- Errors: `thiserror` in libs, `anyhow` in bins/tools, `tracing` for logs.
- Testing: unit + property (`proptest`) + integration (headless client/server) + parity (YAML in `parity/`) + fuzz (packet decoder, NBT) + benches (`criterion`).
- Headless everything early; determinism tests across Windows/Linux.
- Commits: small, focused, conventional (`feat:`, `fix:`, `docs:`, `chore:`); docs/ADRs updated with architecture changes.
- Assets: proprietary converted output only in `game-assets/` (gitignored); CI/placeholder assets in `assets-placeholder/`; never `include_bytes!` proprietary files.
- Telemetry: none. Crash reports local only.

## Milestone discipline (SPEC §17, plan in PLAN.md)
- M0 foundation → M1 core data → M2 world storage → M3 render → M4 server/protocol → M5 faithful gameplay → M6 worldgen → M7 mobs/AI → M8 UI → M9 LAN → M10 splitscreen → M11 audio → M12 modding → M13 presets → M14 polish → v1.0.
- Do not start `M(N+1)` before `M(N)` acceptance criteria pass. Tag milestone commits.
- **Update `PROGRESS.md` at every meaningful checkpoint and before ending a session.**
