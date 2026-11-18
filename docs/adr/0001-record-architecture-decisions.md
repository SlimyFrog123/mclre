# 0001. Record Architecture Decisions as ADRs

- **Status:** Accepted
- **Date:** 2026-11-17
- **Deciders:** Owner (implied by SPEC §0/§20), agent-initiated

## Context
MCLRE is a long-horizon project (M0 → v1.0 and beyond) implemented across many sessions. SPEC §0.3 requires that ambiguity never be resolved by silent guessing, and §20 lists 11+ open decisions that will shape M1–M12.

## Decision
Every significant architecture or fidelity decision is recorded as a numbered ADR in `docs/adr/NNNN-title.md` using the template (`0000-template.md`): Context, Options considered, Decision, Consequences, Status (Proposed/Accepted/Superseded).

- ADRs drafted by the agent start as **Proposed** and are flagged to the owner.
- Owner confirmation flips them to **Accepted** (owner may edit the decision).
- Superseded ADRs are kept, marked, and never deleted.

## Consequences
- Session handoff is cheap: decisions survive context loss.
- Slower initial setup; significantly fewer reversed decisions later.
- The `[DECIDE]`/`[VERIFY]` ledger in `PROGRESS.md` tracks ADR acceptance status.
