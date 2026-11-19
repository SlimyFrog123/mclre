# PROGRESS: Minecraft: Legacy Rust Edition (MCLRE)

> **Purpose:** Single live state ledger for seamless continuation across sessions.
> **Rule:** Update this file at every meaningful checkpoint, whenever a decision/verification is resolved, and before ending any session.

---

## 1. Current Status Snapshot

- **Current Phase:** Phase 0 COMPLETE · M0 (Foundation) complete locally and remotely — all acceptance criteria met.
- **Active Milestone:** M0 (finish line) → next is M1 (Core Data Model)
- **Overall Progress:** Phase 0 done (reference surveyed, CLAUDE.md + 14 ADRs). M0 complete: 22 crate skeletons, dependency-rule test (verified to fail on violation), CI workflow, deny.toml; local gates green: fmt, clippy `-D warnings`, `cargo-deny`, `cargo-audit`. **M1 started: `core::identifier` implemented and green.**
- **Working Directory:** `C:\Users\DJ\Desktop\mclre`
- **Last Updated:** 2026-11-18

### Immediate Next Actions (Start Here in Next Session)
1. **Begin M1 implementation:**
   - Implement core data model including Identifier + interning in crates/core
   - Implement registries, flattened block-state model, GameRandom, NBT wrapper, and profiles skeleton

---

## 2. Environment Snapshot (Verified 2026-11-17)

| Tool / Item | Observed Value | Notes |
|---|---|---|
| OS / Platform | Windows (`win32`) | Target CI matrix: Windows + Linux |
| `rustc` | `1.88.0 (6b00bc388 2025-06-23)` | Available; pinned in `rust-toolchain.toml` (components rustfmt + clippy auto-installed on first build) |
| `cargo` | `1.88.0 (873a06493 2025-05-10)` | Available |
| `rustup` toolchain | `stable-x86_64-pc-windows-msvc (active, default)` | Available |
| `git` | `2.56.0.windows.4` | Dedicated repo at `C:\Users\DJ\Desktop\mclre` (branch `main`); identity `DJ <89934852+SlimyFrog123@users.noreply.github.com>` |
| `cargo-deny` | **Installed:** `0.21.0` (prebuilt binary in `%USERPROFILE%\.cargo\bin`) | `cargo-deny check licenses bans sources` → exit 0 |
| `cargo-audit` | **Installed:** `0.23.0` (prebuilt binary in `%USERPROFILE%\.cargo\bin`; invoke as `cargo-audit audit`) | `cargo-audit audit` → exit 0, 0 vulnerabilities (33 deps) |
| Disk space | `C:` ~9 GB free; `E:` ~242 GB free | `C:` is tight — the first submodule clone attempt failed with "No space left on device" (transient; a `--depth 1` retry succeeded). Prefer shallow clones and watch space when adding big dependencies. |

---

## 3. Milestone Tracker

| Milestone | Title | Status | Tag / Commit | Notes / Acceptance Gate |
|---|---|---|---|---|
| **Phase 0** | Reference Survey & Pre-M0 Groundwork | **Complete** | `09b225e`, `b0f6662` | Reference surveyed (`docs/reference/{reference-tree,baseline,rng,discrepancies,feature-flags}.md`); `CLAUDE.md` + ADRs 0000–0014 drafted (Proposed). V-01/V-03/V-05 resolved; V-02/V-04 partially (see ledger). |
| **M0** | Foundation | **~95% (local gates green)** | `d612c38`, `101e0b7` | DONE: 22 crate skeletons; dep-rule test in `mclre-devtools` (verified it FAILS when `mclre-server` gains a render dep, then reverted); `.github/workflows/ci.yml` (Windows+Linux); `deny.toml`; licenses applied (`MIT OR Apache-2.0`); fmt/clippy `-D warnings`/`cargo test --workspace`/`cargo-deny`/`cargo-audit` all exit 0 locally. REMAINING: run CI on a remote (no GitHub remote yet) → then tag `m0-foundation`. |
| **M1** | Core Data Model | Not Started | — | Blocked on M0. Gate: ~20 test blocks via public API, `GameRandom` parity vectors, no `rand` in gameplay crates. |
| **M2** | World Storage & Chunks | Not Started | — | Blocked on M1. Gate: chunk round-trip, crash-safety test, external Anvil inspect, bounded+infinite support. |
| **M3** | Minimal Client Rendering | Not Started | — | Blocked on M2. Gate: 32-chunk flat world `>= 60 FPS` (mid), Vulkan/DX12/GL start. |
| **M4** | Server Loop, Protocol, Integrated Server, Movement | Not Started | — | Blocked on M3. Gate: playable flat singleplayer via `InMemoryTransport` + headless client CI test. |
| **M5** | Faithful Core Gameplay | Not Started | — | Blocked on M4. Gate: golden-data recipes/hardness/drops + `parity/` scenarios pass. |
| **M6** | World Generation (Baseline) | Not Started | — | Blocked on M5. Gate: reference seed heightmap/biome/terrain parity + visual sign-off. |
| **M7** | Mobs & AI | Not Started | — | Blocked on M6. Gate: mob `parity/` scenarios + server tick budget met. |
| **M8** | UI/UX Parity Pass 1 | Not Started | — | Blocked on M7. Gate: full gamepad-only & KB+M-only menu flow + visual comparison sign-off. |
| **M9** | LAN Multiplayer & Dedicated Server | Not Started | — | Blocked on M8. Gate: 4-player 1h LAN soak test + decoder fuzzing + headless `mclre-server`. |
| **M10** | Splitscreen | Not Started | — | Blocked on M9. Gate: 4-player splitscreen `>= 30 FPS` (mid) + mixed LAN. |
| **M11** | Audio | Not Started | — | Blocked on M10. Gate: sound parity checklist + non-fatal audio init failure. |
| **M12** | Modding v1 | Not Started | — | Blocked on M11. Gate: `examples/hello-mod` on client/server + mod mismatch msg + sandbox test. |
| **M13** | World Size Presets & Persistence Polish | Not Started | — | Blocked on M12. Gate: all 5 presets + 10k-block infinite exploration without leaks/hitches. |
| **M14** | Tutorial World, Stats, Polish → v1.0 | Not Started | — | Blocked on M13. Gate: full baseline play-through checklist + `v1.0.0` tag. |

---

## 4. `[VERIFY]` & `[DECIDE]` Status Ledger

### 4.1 Reference Verifications (`[VERIFY]`)
| ID | Item | Status | Resolved Value / File |
|---|---|---|---|
| V-01 | `reference/lce-141217/` tree layout, build system, asset paths | **Resolved** | `docs/reference/reference-tree.md` — Visual Studio C++ solution; `Minecraft.World` = flat ~1,800 .cpp/.h (~5.7 MB); `Minecraft.Client` = ~3,572 files (~598 MB) incl. `.swf`/`.xui` UI, `.png`, `.ttf`, `.arc`/`.pck` archives, `.binka` audio; `PSVita/`+`sce_sys/`+`redist64/` = platform glue (do not port). |
| V-02 | Exact Dec 17, 2014 baseline TU/CU & final XB1/PS4 TU | **Partially resolved** | `docs/reference/baseline.md` — build is date 2014-12-17, display "1.6", product build 560, PC 1.8.2-era forward-ports; likely TU19/CU7 but the source carries no TU string → profile ID `lce:141217` until owner confirms. Final XB1/PS4 TU still open. |
| V-03 | Exact `classic`/`small`/`medium`/`large` dimensions, Nether/End scale, world edge behavior | **Resolved** | `docs/reference/baseline.md` — 54/64/192/320 chunk widths (864/1024/3072/5120 blocks), per-preset Nether scales 3/3/6/8, End fixed 18×18 chunks, edge **moats** for classic/small/medium (`ChunkSource.h:57-59`). |
| V-04 | `Mth` sine lookup tables, float/double ops, chunk/layer seed XOR constants | **Partially resolved** | `docs/reference/rng.md` — `Random.cpp` is an exact java.util.Random port (0x5DEECE66D/0xB/48-bit, modern power-of-2 nextInt path, Box-Muller cache); `Mth` 65536-entry f32 sine LUT. Chunk/layer seed constants still to analyze at M6. |
| V-05 | Asset container/audio/texture/lang formats (`.arc`, `.binka`, `.dds`, etc.) | **Resolved** | `docs/reference/reference-tree.md` §3 — `.arc` (5, reader in `ArchiveFile.{h,cpp}`), `.pck` (19), `.binka` (40+, music), `.png` (391), `.ttf` + bitmap fonts, `.swf` (376) + `.xui` (170) UI. No `.dds` found in Common/Media. |
| V-06 | Splitscreen multi-listener audio attenuation behavior | Pending | `docs/reference/audio.md` (at M11) |

### 4.2 Open Decisions (`[DECIDE]`)
> All ADRs are now **drafted as "Proposed"** (`docs/adr/0002`–`0014`) with recommended defaults active until the owner confirms. Statuses below track *owner confirmation*.

| ID | Decision | Status | Proposed Default | ADR Path |
|---|---|---|---|---|
| D-01 | Private repo reference/asset policy & Git LFS for `game-assets/` | ADR drafted | Isolated `game-assets/`, read-only `reference/lce-141217/`, LFS deferred | `docs/adr/0002-reference-and-asset-policy.md` |
| D-02 | LCE-only namespace policy (`lce:` vs `minecraft:`) | ADR drafted | `minecraft:` if Java ID exists, else `lce:` | `docs/adr/0003-identifier-namespaces.md` |
| D-03 | `infinite` preset terrain match inside finite bounds | ADR drafted | Unbounded Java-like layout; parity region = finite presets | `docs/adr/0004-infinite-world-bounds-behavior.md` |
| D-04 | Server ECS framework | ADR drafted | `bevy_ecs` standalone behind `sim::entity` | `docs/adr/0005-ecs-selection.md` |
| D-05 | NBT crate (`simdnbt` vs `fastnbt`) | ADR drafted | `simdnbt` behind internal `core::nbt` API | `docs/adr/0006-nbt-crate.md` |
| D-06 | Light propagation authority | ADR drafted | Server computes & sends light | `docs/adr/0007-lighting-authority.md` |
| D-07 | Text rendering crate | ADR drafted | `glyphon` + custom bitmap pixel font path | `docs/adr/0014-client-structure-and-text.md` |
| D-08 | Chunk greedy meshing vs AO fidelity | ADR drafted | Per-face first; greedy off by default | `docs/adr/0008-chunk-meshing-ao.md` |
| D-09 | Missing `--username` policy & splitscreen guest UUID derivation | ADR drafted | Refuse in MP/prod, `Player` in `--dev-defaults`; UUIDv5 guests | `docs/adr/0009-identity-and-guests.md` |
| D-10 | LAN transport, certs & discovery | ADR drafted | `quinn` (QUIC, TOFU/LAN cert) + `mdns-sd` | `docs/adr/0010-networking-transport-discovery.md` |
| D-11 | Audio crate, multi-listener mix & `assetc` `.binka` decoder | ADR drafted | `kira` + external `vgmstream` invocation | `docs/adr/0011-audio-and-assetc-decoders.md` |
| D-12 | Modding WASM runtime & dev-only native Rust mods | ADR drafted | `wasmtime` (WIT component model) | `docs/adr/0012-modding-runtime.md` |
| D-13 | Perf hardware targets, `faithful_bugs` defaults, finite→infinite migration, shader/project license, naming | ADR drafted | Low/mid profiles defined; bugs on/off by class; MIT/Apache-2.0 dual; shader clean-room | `docs/adr/0013-perf-licensing-and-scope.md` |

---

## 5. Session Log

### Session 1 — 2026-11-17
- **Goal:** Read `SPEC.md`, create detailed development plan (`PLAN.md`), and establish seamless progress tracking (`PROGRESS.md`).
- **Completed:**
  - Located and read `C:\Users\DJ\Desktop\mclre\SPEC.md` (664 lines, v0.1 draft).
  - Checked directory contents (`SPEC.md` only) and environment (`rustc 1.88.0`, `cargo 1.88.0`, `git 2.56.0`, parent `Desktop/` has an empty `.git`, `cargo-deny` and `cargo-audit` not currently installed).
  - Created `C:\Users\DJ\Desktop\mclre\PLAN.md` covering architectural guardrails, workspace crate layout, `[VERIFY]`/`[DECIDE]` registries, detailed Phase 0 + M0–M14 execution steps + acceptance gates, and Post-1.0 roadmap.
  - Created `C:\Users\DJ\Desktop\mclre\PROGRESS.md` with live status snapshot, environment baseline, milestone table, `[VERIFY]`/`[DECIDE]` tracker, and session log.
- **Files Created / Modified:**
  - `C:\Users\DJ\Desktop\mclre\PLAN.md` (created)
  - `C:\Users\DJ\Desktop\mclre\PROGRESS.md` (created)
- **Verification Run:** Confirmed `rustc`, `cargo`, `git`, and directory layout via shell/glob; no builds run (no Rust workspace exists yet).
- **Blockers / Open Questions for Owner:**
  - Confirm initializing a dedicated git repository inside `C:\Users\DJ\Desktop\mclre\` (separate from parent `C:\Users\DJ\Desktop\.git`).
  - Confirm proceeding to Phase 0 (cloning `https://github.com/furax124/Minecraft-LCE-PS3` `MinecraftConsoles-141217` into `reference/lce-141217/`).
- **Next Steps:** Follow Section 1 ("Immediate Next Actions") in `PROGRESS.md`.

### Session 2 — 2026-11-17
- **Goal:** Initialize the dedicated git repository, attach the LCE reference submodule, and scaffold the Rust/Cargo project structure.
- **Completed:**
  - Initialized dedicated git repo at `C:\Users\DJ\Desktop\mclre` (branch `main`); confirmed the parent `Desktop\.git` no longer affects `git status` inside `mclre`.
  - Attached submodule `https://github.com/furax124/Minecraft-LCE-PS3.git` at `reference/lce-141217` (`--depth 1`, commit `0aa818c2`, branch `main`). First clone attempt failed transiently ("No space left on device" / C: had ~9 GB free); a shallow retry succeeded. Set `submodule.reference/lce-141217.update none` to discourage accidental updates. Verified tree: repo root contains `MinecraftConsoles-141217/{Minecraft.Client, Minecraft.World, MinecraftConsoles.sln}`, `mc-console-oct2014/`, `mc-cpp-snapshot/`, `README.md`, and a torrent file.
  - Created workspace root: `Cargo.toml` (resolver "3", edition 2024, workspace lints: `unsafe_code = "forbid"`, `missing_docs = "warn"`, clippy `all` + `pedantic` warn; version-pinned `[workspace.dependencies]`; internal path deps pre-declared), `rust-toolchain.toml` (pinned 1.88.0 + rustfmt/clippy), `.gitignore` (`/target/`, `/game-assets/`, logs, crash-reports).
  - Generated 22 crate skeletons under `crates/` (all with `[lints] workspace = true`): libs `core, profiles, content-vanilla, world, worldgen, sim, protocol, net, server, client, render, ui, audio, input, assets, mod-api, modding, identity, tools/devtools`; bins `app` (pkg `mclre`, bin `mclre`), `server-bin` (pkg `mclre-server-bin`, bin `mclre-server`), `tools/assetc` (bin `assetc`).
  - Verified with `cargo check --workspace` (EXIT=0; ~1.1s). No external deps are pulled yet.
- **Files Created / Modified:**
  - `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`, `.gitignore` (created)
  - `crates/*/Cargo.toml` + `crates/*/src/{lib,main}.rs` (22 crates, created)
  - `PROGRESS.md` (updated)
- **Git History:**
  - `43ed07c` chore: seed repo with SPEC, PLAN, PROGRESS, and read-only LCE reference submodule (lce-141217)
  - `5883ad0` chore: scaffold cargo workspace with 22 crate skeletons (M0 start)
- **Verification Run:** `cargo check --workspace` → success. `git submodule status` → `0aa818c2 ... reference/lce-141217 (heads/main)`.
- **Blockers / Open Questions for Owner:**
  - None new. `cargo-deny`/`cargo-audit` still need installing (M0). C: disk space (~9 GB) is tight; watch it when adding `wgpu`/`wasmtime` etc.
- **Next Steps:** Follow Section 1 ("Immediate Next Actions"): reference-tree survey + `docs/reference/*` docs, `CLAUDE.md`, ADRs, then finish M0 (CI, `deny.toml`, dependency-rule test).

### Session 3 — 2026-11-17
- **Goal:** Continue next steps: reference survey docs, CLAUDE.md, ADRs, finish M0 — documenting each change as it happens.
- **Completed (in order):**
  1. **Reference survey** — inventoried submodule (`Minecraft.World`: ~1,800 flat .cpp/.h, 5.7 MB; `Minecraft.Client`: ~3,572 files, 598 MB with `.swf`×376, `.xui`×170, `.png`×391, `.arc`×5, `.pck`×19, `.binka`×40+, `.ttf`×10). Verified `Random.cpp` = exact java.util.Random port; `Mth` 65536-entry sine LUT; world sizes (54/64/192/320 chunks; Nether scales 3/3/6/8; End fixed 18×18; edge moats in `ChunkSource.h:57-59`); version = "1.6"/build 560/1.8.2-era. Wrote `docs/reference/{reference-tree,baseline,rng,discrepancies,feature-flags}.md`. → commit `09b225e`.
  2. **Governance** — wrote `CLAUDE.md` (condensed SPEC rules) + `docs/adr/0000-template.md`, `0001`, and drafts `0002`–`0014` (all "Proposed" with recommended defaults). → commit `b0f6662`.
  3. **M0 dependency-rule test** — `crates/tools/devtools/tests/dependency_rules.rs` (via `cargo metadata --no-deps`): gameplay crates ⊬ GPU/UI/client crates; pure gameplay ⊬ `rand`; `client` ⊬ `sim`/`server`; `server-bin` ⊬ client-side crates. **Gate proven**: temporarily added `mclre-render` to `mclre-server` → test FAILED with clear message → reverted → green.
  4. **CI + deny** — `.github/workflows/ci.yml` (windows+ubuntu: fmt, clippy `-D warnings`, test, cargo-deny action, audit-check); `deny.toml`. Fixed clippy 1.88 `lint-groups-priority` (group priority -1, specific allows priority +1); selective pedantic: `doc_markdown`, `module_name_repetitions` allowed workspace-wide. All local gates exit 0. → commit `d612c38`.
  5. **Licenses** — applied `MIT OR Apache-2.0` (ADR 0013 default) to all 24 manifests so `cargo-deny` passes; `deny.toml` `unused-allowed-license = "allow"`. → commit `101e0b7`.
  6. **Tooling installed** — `cargo-deny 0.21.0` and `cargo-audit 0.23.0` (prebuilt releases → `%USERPROFILE%\.cargo\bin`; note cargo-audit releases live under `rustsec/rustsec`, and the CLI needs the `audit` subcommand).
- **Verification Run:** `cargo fmt --all -- --check` 0 · `cargo clippy --workspace --all-targets -- -D warnings` 0 · `cargo test --workspace` 0 (2 dep-rule tests pass) · `cargo-deny check licenses bans sources` 0 · `cargo-audit audit` 0 (33 deps, 0 vulns).
- **Files Created / Modified:** `CLAUDE.md`; `docs/reference/*` (5); `docs/adr/*` (16); `crates/tools/devtools/tests/dependency_rules.rs` + `Cargo.toml`; `.github/workflows/ci.yml`; `deny.toml`; root + 22 crate `Cargo.toml`s (fmt fixes, license, lint config); `PROGRESS.md`.
- **Blockers / Open Questions for Owner:**
  - Confirm ADRs 0002–0014 (esp. D-02 namespaces, D-05 NBT before M1 freezes).
  - Confirm baseline TU/CU externally; profile currently `lce:141217`.
  - Provide a private GitHub remote (or confirm none) so `ci.yml` runs; then tag `m0-foundation`.
- **Next Steps:** M1 — Core Data Model (see Section 1 item 2).
