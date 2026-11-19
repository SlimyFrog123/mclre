# Feature Flags — Evidence Registry

> One row per `FeatureFlag` in `crates/profiles`. Every flag must cite reference evidence (SPEC 3.6/6.3). Flags are added here as gameplay features are ported; mechanics read flags, never version numbers.

| Flag | Description | Introduced in | User-toggleable? | Evidence (reference file/function) | Parity test |
|---|---|---|---|---|---|
| `lce_crafting_menu` | Console-style tabbed crafting menu (recipe groups/categories, "craftable now" filtering) instead of the Java 3×3 grid UI. | All `lce:` profiles; off for Java-parity profiles. | No (edition trait) | SPEC.md 3.6 + PLAN.md M5. Reference `Minecraft.Client` crafting-menu/recipe-group code to be cited at M5 (UI port). | Pending (M5 crafting scenarios). |
| `lce_host_privileges` | Per-player host-privilege permission toggles (build/mine, use doors/containers, attack players/mobs, fly, ...) instead of Java slash commands. | All `lce:` profiles; off for Java-parity profiles. | No (edition trait); individual privileges are host-toggleable at runtime | SPEC.md 5.8. Reference `HostOptions`/privilege code to be cited at M9. | Pending (M9 LAN scenarios). |

**Registration workflow (when adding a flag):**
1. Confirm the mechanic actually changed across versions (reference comments, wiki, owner captures).
2. Add typed enum variant in `crates/profiles` + row above.
3. Use `ctx.profile().has(Feature::X)` at the decision point(s); centralize reads.
4. Add/extend a parity test in `parity/`.

> **Status 2026-11-18:** the two flags above are edition-level (LCE vs Java) differences that are stated in `SPEC.md` itself. Their *reference code* citations are deliberately deferred to the milestone that ports the mechanic (M5 / M9), as recorded in the row. No flag is set that affects baseline gameplay outcome yet.
