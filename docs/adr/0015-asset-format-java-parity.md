# 0015. Asset Formats: Use Minecraft Java Edition Formats

- **Status:** Accepted
- **Date:** 2026-11-19
- **Deciders:** Project Owner, Agent

## Context

MCLRE needs to define its modern asset formats for textures, models, audio, localization, UI layouts, structures, and content packs. The project has a long-term goal of reaching feature and multiplayer parity with Minecraft: Java Edition. Using Java Edition's resource/data pack formats provides:

1. Direct compatibility with the vast ecosystem of Java resource packs, data packs, and tools
2. Easier backporting from Java Edition features, worldgen, and content
3. Familiar tooling for modders and content creators
4. Future Java Edition protocol/parity compatibility

The alternative would be to invent custom formats, which would create ongoing maintenance burden and make it harder to leverage community content.

## Options considered

1. **Custom MCLRE-specific formats** — Design new formats tailored to MCLRE's needs
2. **Java Edition resource/data pack formats** — Use Java's JSON models, blockstates, sounds.json, pack.mcmeta, etc.
3. **Hybrid** — Use Java formats for some things, custom for others

## Decision

Use **Minecraft: Java Edition resource/data pack formats** for all modern asset formats:

| Purpose | Format |
|---|---|
| Block/item models | Java JSON models + blockstate JSON (parent models, elements, faces, variants/multipart) |
| Audio | Ogg Vorbis + Java `sounds.json` mappings |
| Localization | Java JSON language files keyed by `namespace:key` |
| UI layouts | RON/JSON (custom, but data-driven) |
| Structures | Java NBT structure files (`structure_template` format) |
| Packs | Java `pack.mcmeta` compatible + `pack.toml`/`pack.json` |

The `assetc` tool will convert LCE legacy formats (DDS, .binka, .arc, Flash UI) to these modern Java-compatible formats.

## Consequences

### Positive
- **Resource pack compatibility:** Java resource packs can be used directly (textures, models, sounds, language)
- **Data pack compatibility:** Java data packs (recipes, loot tables, tags, structures) work with minimal adaptation
- **Tooling:** Existing Java ecosystem tools (Blockbench, resource pack editors, data pack generators) work out of the box
- **Backporting:** Java Edition features, worldgen improvements, and content can be ported more easily
- **Modding:** Modders already familiar with Java formats; lower barrier to entry
- **Future parity:** Easier path to Java Edition protocol and multiplayer compatibility

### Negative
- **Conversion step required:** `assetc` must convert all LCE assets to Java formats (one-time cost)
- **LCE-specific content:** Some LCE-exclusive content may not have direct Java equivalents; these go in `lce:` namespace
- **Format complexity:** Java formats are more complex than minimal custom formats (e.g., blockstate variants vs. simple mappings)

### When to revisit
- If a specific Java format proves incompatible with MCLRE's architecture (e.g., blockstate JSON complexity vs. runtime performance)
- If new Java formats are introduced that break compatibility
- Post-v1.0 when evaluating full Java parity