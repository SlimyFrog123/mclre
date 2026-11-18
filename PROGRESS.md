# PROGRESS: Minecraft: Legacy Rust Edition (MCLRE)

> **Purpose:** Single live state ledger for seamless continuation across sessions.
> **Rule:** Update this file at every meaningful checkpoint, whenever a decision/verification is resolved, and before ending any session.

---

## 1. Current Status Snapshot

- **Current Phase:** Phase 0 — Reference Survey & Pre-M0 Groundwork (repo + reference submodule + workspace scaffold DONE; reference survey / CLAUDE.md / ADRs still pending)
- **Active Milestone:** Pre-M0 (Kick-off checklist §21.0) — M0 workspace scaffold started early
- **Overall Progress:** Dedicated git repo initialized (`main` branch, 2 commits); `reference/lce-141217` submodule attached (shallow, commit `0aa818c`); Cargo workspace with 22 crate skeletons created and verified with `cargo check --workspace`.
- **Working Directory:** `C:\Users\DJ\Desktop\mclre`
- **Last Updated:** 2026-11-17

### Immediate Next Actions (Start Here in Next Session)
1. **Survey the reference tree (`SPEC.md` §21.0):**
   - Submodule is already attached at `reference/lce-141217/` (repo root; the Dec-2014 build is under `reference/lce-141217/MinecraftConsoles-141217/` with `Minecraft.Client`, `Minecraft.World`, `MinecraftConsoles.sln`).
   - Inventory `Minecraft.World` and `Minecraft.Client` and write `docs/reference/reference-tree.md`, `docs/reference/baseline.md`, `docs/reference/rng.md`, `docs/reference/discrepancies.md`, and `docs/reference/feature-flags.md`.
   - Resolve `[VERIFY]` items `V-01` through `V-05`.
2. **Create governance files & initial ADRs:**
   - Create `CLAUDE.md` (condensed core rules from `SPEC.md`).
   - Create `docs/adr/0000-template.md`, `docs/adr/0001-record-architecture-decisions.md`, and draft ADRs for `[DECIDE]` items (`D-01`..`D-13`) for owner sign-off.
3. **Finish M0 — Foundation:** Remaining M0 work after the scaffold: `.github/workflows/ci.yml` (Windows + Linux: fmt, clippy `-D warnings`, test, `cargo-deny`, `cargo-audit`, dependency-graph test), `deny.toml`, and the automated dependency-rule test (see `PLAN.md` §4 M0). `cargo-deny` and `cargo-audit` are **not installed** locally yet.

---

## 2. Environment Snapshot (Verified 2026-11-17)

| Tool / Item | Observed Value | Notes |
|---|---|---|
| OS / Platform | Windows (`win32`) | Target CI matrix: Windows + Linux |
| `rustc` | `1.88.0 (6b00bc388 2025-06-23)` | Available; pinned in `rust-toolchain.toml` (components rustfmt + clippy auto-installed on first build) |
| `cargo` | `1.88.0 (873a06493 2025-05-10)` | Available |
| `rustup` toolchain | `stable-x86_64-pc-windows-msvc (active, default)` | Available |
| `git` | `2.56.0.windows.4` | Dedicated repo at `C:\Users\DJ\Desktop\mclre` (branch `main`); identity `DJ <89934852+SlimyFrog123@users.noreply.github.com>` |
| `cargo-deny` | Not installed (exit code 1) | Install before running M0 license/advisory checks |
| `cargo-audit` | Not installed (exit code 1) | Install before running M0 advisory checks |
| Disk space | `C:` ~9 GB free; `E:` ~242 GB free | `C:` is tight — the first submodule clone attempt failed with "No space left on device" (transient; a `--depth 1` retry succeeded). Prefer shallow clones and watch space when adding big dependencies. |

---

## 3. Milestone Tracker

| Milestone | Title | Status | Tag / Commit | Notes / Acceptance Gate |
|---|---|---|---|---|
| **Phase 0** | Reference Survey & Pre-M0 Groundwork | **In Progress** | `43ed07c` | Git repo init + reference submodule attached + workspace scaffold DONE (done early, ahead of M0). Remaining: `docs/reference/*` survey, `CLAUDE.md`, ADRs. |
| **M0** | Foundation | **In Progress** | — | Workspace skeleton (22 crates, edition 2024, workspace lints, `rust-toolchain.toml`) done; `cargo check --workspace` green. Remaining: CI, `deny.toml`, dependency-rule test, `cargo-deny`/`cargo-audit` install. Gate: `cargo test --workspace` + dep-rule test + CI + `cargo-deny`/`cargo-audit`. |
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
| V-01 | `reference/lce-141217/` tree layout, build system, asset paths | Pending | `docs/reference/reference-tree.md` |
| V-02 | Exact Dec 17, 2014 baseline TU/CU & final XB1/PS4 TU | Pending | `docs/reference/baseline.md` |
| V-03 | Exact `classic`/`small`/`medium`/`large` dimensions, Nether/End scale, world edge behavior | Pending | `docs/reference/baseline.md` |
| V-04 | `Mth` sine lookup tables, float/double ops, chunk/layer seed XOR constants | Pending | `docs/reference/rng.md` |
| V-05 | Asset container/audio/texture/lang formats (`.arc`, `.binka`, `.dds`, etc.) | Pending | `docs/reference/reference-tree.md` |
| V-06 | Splitscreen multi-listener audio attenuation behavior | Pending | `docs/reference/audio.md` |

### 4.2 Open Decisions (`[DECIDE]`)
| ID | Decision | Status | Proposed Default | ADR Path |
|---|---|---|---|---|
| D-01 | Private repo reference/asset policy & Git LFS for `game-assets/` | Pending | Isolated `game-assets/`, read-only `reference/lce-141217/` | `docs/adr/0002-...` |
| D-02 | LCE-only namespace policy (`lce:` vs `minecraft:`) | Pending | `minecraft:` if Java ID exists, else `lce:` | `docs/adr/0003-...` |
| D-03 | `infinite` preset terrain match inside finite bounds | Pending | Confirm with owner | `docs/adr/0004-...` |
| D-04 | Server ECS framework | Pending | `bevy_ecs` standalone behind `sim::entity` | `docs/adr/0005-...` |
| D-05 | NBT crate (`simdnbt` vs `fastnbt`) | Pending | `simdnbt` (verify API/allocations) | `docs/adr/0006-...` |
| D-06 | Light propagation authority | Pending | Server computes & sends light | `docs/adr/0007-...` |
| D-07 | Text rendering crate | Pending | `glyphon` + custom bitmap pixel font path | `docs/adr/0008-...` |
| D-08 | Chunk greedy meshing vs AO fidelity | Pending | Per-face first; greedy off by default | `docs/adr/0009-...` |
| D-09 | Missing `--username` policy & splitscreen guest UUID derivation | Pending | Refuse in MP/prod, `Player` in `--dev-defaults`; deterministic guest UUIDs | `docs/adr/0010-...` |
| D-10 | LAN transport, certs & discovery | Pending | `quinn` (QUIC, TOFU/LAN cert) + `mdns-sd` | `docs/adr/0011-...` |
| D-11 | Audio crate, multi-listener mix & `assetc` `.binka` decoder | Pending | `kira` + external `vgmstream` invocation | `docs/adr/0012-...` |
| D-12 | Modding WASM runtime & dev-only native Rust mods | Pending | `wasmtime` (WIT component model) | `docs/adr/0013-...` |
| D-13 | Perf hardware targets, `faithful_bugs` defaults, finite→infinite migration, shader/project license, naming | Pending | Confirm with owner | `docs/adr/0014-...` |

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
