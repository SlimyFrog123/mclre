# 0009. Identity Launch Args & Splitscreen Guest UUIDs

- **Status:** Proposed (pending owner confirmation)
- **Date:** 2026-11-17
- **Deciders:** Agent draft; owner to confirm

## Context
SPEC 9.1 [DECIDE]. No auth backend exists yet; handshake must carry identity claim + optional proof field (empty) and accept-all verify hook.

## Options considered
See `PLAN.md` §3.2 (D-09).

## Decision (recommended default)
Missing `--username`: refuse in dedicated/multiplayer, fallback `Player` + warning with `--dev-defaults`. `--uuid` must be UUIDv7, rejected otherwise. Guests: UUIDv5 derived deterministically from primary UUID + slot (offline-shaped), documented and collision-safe across installs for LAN play.

## Consequences
UUIDv5 derivation is stable and needs no persistence; swap to persisted UUIDv7s later if launcher integration requires. Owner confirmation pending before M4.
