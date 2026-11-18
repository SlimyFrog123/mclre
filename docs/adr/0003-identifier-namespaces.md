# 0003. Namespace Policy for Identifiers

- **Status:** Proposed (pending owner confirmation)
- **Date:** 2026-11-17
- **Deciders:** Agent draft; owner to confirm

## Context
SPEC 5.1 left the `lce:` vs `minecraft:` split open. The reference uses numeric IDs; our registry needs a stable namespace rule from M1 (first registries).

## Options considered
See `PLAN.md` §3.2 (D-02).

## Decision (recommended default)
`minecraft:` for anything with a Java-equivalent ID (e.g. `minecraft:stone`); `lce:` only for LCE-exclusive content (mini-games, console UI assets, console-only items). Mod namespaces per `mod.toml` id.

## Consequences
LCE content that later became vanilla keeps the `minecraft:` ID (eases future Java parity); `lce:` namespace stays small and semantically clean. Renaming later would break saves, so confirm before M1 freezes registries.
