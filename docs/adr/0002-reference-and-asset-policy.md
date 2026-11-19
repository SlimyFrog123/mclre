# 0002. Reference Tree & Asset Isolation Policy

- **Status:** Accepted (agent decision, owner confirmation pending) — confirmed 2026-11-18
- **Date:** 2026-11-17
- **Deciders:** Agent (owner confirmation pending)

## Context
SPEC 3.5/11.3/16 supersede the old clean-room policy: the private repo may contain the reference source and (later) original assets, isolated so distribution scope can shrink later. Verified tree layout recorded in `docs/reference/reference-tree.md`.

## Options considered
See `PLAN.md` §3.2 (D-01) for the full option list.

## Decision (recommended default)
Read-only submodule at `reference/lce-141217/`; proprietary converted assets only in gitignored `game-assets/`; placeholders in `assets-placeholder/`; Git LFS decision deferred until `game-assets/` is first populated.

## Consequences
Reference docs cite file/function names (no pasted code). `game-assets/` regenerated via `assetc` anytime. Git LFS should be enabled before the first large binary commit; if not, history grows fast.
