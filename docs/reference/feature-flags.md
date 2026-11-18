# Feature Flags — Evidence Registry

> One row per `FeatureFlag` in `crates/profiles`. Every flag must cite reference evidence (SPEC 3.6/6.3). Flags are added here as gameplay features are ported; mechanics read flags, never version numbers.

| Flag | Description | Introduced in | User-toggleable? | Evidence (reference file/function) | Parity test |
|---|---|---|---|---|---|
| — | (none yet; populated starting M5) | | | | |

**Registration workflow (when adding a flag):**
1. Confirm the mechanic actually changed across versions (reference comments, wiki, owner captures).
2. Add typed enum variant in `crates/profiles` + row above.
3. Use `ctx.profile().has(Feature::X)` at the decision point(s); centralize reads.
4. Add/extend a parity test in `parity/`.
