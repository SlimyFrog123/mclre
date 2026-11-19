# 0010. LAN Transport & Discovery

- **Status:** Accepted (agent decision, owner confirmation pending) — confirmed 2026-11-18
- **Date:** 2026-11-17
- **Deciders:** Agent (owner confirmation pending)

## Context
SPEC 10/14 [DECIDE]. Transport trait stays abstract so UPnP/relay are later transports, not protocol changes. Alternatives: `renet`/`laminar` (smaller surface, no TLS).

## Options considered
See `PLAN.md` §3.2 (D-10).

## Decision (recommended default)
`quinn` (QUIC) as the LAN transport: TLS built in, reliable streams + datagrams, self-signed certs with trust-on-first-use for LAN. Discovery via `mdns-sd` with optional UDP broadcast fallback. Compression (`zstd`/`lz4_flex`) above threshold.

## Consequences
`quinn` adds tokio to the net path only (not simulation). Cert UX: first-connect prompt; TOFU cache keyed by fingerprint. Owner confirmation pending before M9 (protocol/net seams designed in M4).
