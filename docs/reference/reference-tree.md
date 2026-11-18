# Reference Tree — `reference/lce-141217/`

> **Status:** Initial survey complete (2026-11-17). Verified from the actual submodule contents.
> **Source:** `https://github.com/furax124/Minecraft-LCE-PS3.git`, branch `main`, pinned at commit `0aa818c2` (shallow clone).
> **Rules (SPEC.md 3.5):** This tree is read-only. Never edited, never compiled into the game, nothing copied into `crates/`. Port *behavior* behind our abstractions.

## 1. Verified repository layout

```
reference/lce-141217/
├─ README.md
├─ MinecraftConsoles-141217/          ← THE Dec 17, 2014 baseline (our reference target)
│  ├─ MinecraftConsoles.sln           # Visual Studio solution (C++ console build)
│  ├─ Minecraft.World/                # ~1,800 .cpp/.h files, ~5.7 MB, almost all flat in this dir
│  └─ Minecraft.Client/               # ~3,572 files, ~598 MB (mostly assets + platform glue)
├─ mc-console-oct2014/                # older snapshot (do not use; older than baseline)
├─ mc-cpp-snapshot/                   # another snapshot (do not use for baseline)
└─ minecraft-legacy-console-edition-source-code_archive.torrent
```

### `Minecraft.World` (server/simulation side)
- Flat directory of `Class.cpp/Class.h` pairs (no deep package tree).
- Contains: entities/mobs/AI (`*Goal.*`, `Entity`, `Mob`, `*Mob.*`), blocks/tiles (`Tile.*`), items (`Item.*`), worldgen (`*LevelSource*`, `Biome*`, `PerlinNoise`, `ImprovedNoise`, `SimplexNoise`, `PerlinSimplexNoise`, `FastNoise`, `IntCache`, `*Feature*`, `*GenLayer*`-style generation), storage (`*LevelSource*`, `RegionFile*`), networking (`Packet*`, `Connection`), recipes, `Random`/`Mth` math, `Attribute`, `Class` (entity type enums), packets with numeric IDs (`getId()` returns e.g. `26`).

### `Minecraft.Client` (rendering/UI/platform side)
- Flat `*Renderer`, `*Model`, `*Screen`, `*Particle` sources at root.
- `Common/`: shared platform glue — `Consoles_App.*`, `App_structs.h`, `App_Defines.h` (world size enum, settings), `BuildVer.h` (version constants), vendored `zlib/`, `Media/` (~728 files: `.png` textures ×391, `.swf` Flash UI ×376, `.xui` UI layouts ×170, `.ttf` fonts ×10, `.arc` archives ×5, `.wav` ×4), `UI/` (~256 C++ UI screens), `res/1_2_2/font/` (bitmap fonts, 225 files).
- `music/`: 40 × `.binka` audio (music discs & tracks, e.g. `blocks.binka`, `cat.binka`).
- `.pck` audio archives ×19, more `.binka` across media.
- `PSVita/`, `PSVitaMedia/`, `sce_sys/`, `redist64/`: console-platform-only glue (ignore per SPEC 3.5 — do not port platform layers).

## 2. Confirmed crate mapping (replaces SPEC.md 3.5 provisional table)

| Reference | Informs |
|---|---|
| `Minecraft.World` (flat C++) | `core`, `world`, `worldgen`, `sim`, `content-vanilla`, `protocol` (semantics only), `profiles` |
| `Minecraft.Client/*Renderer*`, `*Model*`, `*Particle*` | `render`, `client` |
| `Minecraft.Client/*Screen*`, `Common/UI/`, `Common/Media` (.swf/.xui/.png/.ttf) | `ui`, `assets`, `input` (menu nav) |
| `Minecraft.Client/ArchiveFile.*`, `Common/Media` (.arc/.pck/.binka) | `tools/assetc`, `assets` |
| `Minecraft.Client/Common/Consoles_App.*`, `App_Defines.h`, `App_structs.h` | `app` glue, world size presets, host options |
| `Minecraft.Client/*Connection*`, `Minecraft.World/Packet*`, `Connection.*` | `protocol`, `server`, `net` (semantics only) |
| `PSVita/*`, `PSVitaMedia/`, `sce_sys/`, `redist64/` | **Do not port** (platform layer, SPEC 3.5) |

## 3. Asset formats found (V-05 resolved)

| Purpose | Format | Notes |
|---|---|---|
| UI runtime | `.swf` (376) + `.xui` (170) XML layouts | Confirms Flash/Iggy middleware; SPEC 12.4 says rebuild UI, never run SWF |
| Textures | `.png` (391 in Common/Media) | Modern-friendly; `assetc` may pass PNG through |
| Archives | `.arc` (5, e.g. in Common/Media) | `ArchiveFile.h/cpp` implements reader: header + index of (filename, ptr, filesize, isCompressed) |
| Audio | `.binka` (40+, music dir), `.pck` (19) | Needs `.binka`→Ogg conversion (vgmstream or bundled decoder, ADR D-11) |
| Fonts | `.ttf` (10) + bitmap fonts in `Common/res/1_2_2/font/` (225 files) | Supports ADR D-07 (TTF + bitmap pixel font path) |
| SFX | `.wav` (4) | |

## 4. Survey TODO (next passes, not blocking M1)
- Inventory worldgen classes in detail (`RandomLevelSource`, `BiomeSource`, `GenLayer` equivalents, structure classes) for `docs/reference/worldgen.md` (needed for M6).
- Inventory `Packet*` numeric IDs for the legacy-mapping tool (`tools/devtools`).
- Check `RegionFile*`/save format classes for the later `lce-import` tool (post-1.0).
