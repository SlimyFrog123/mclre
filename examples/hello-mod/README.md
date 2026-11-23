# Hello Mod Example

A simple example mod for MCLRE demonstrating the modding API.

## Structure

```
hello-mod/
├── mod.toml      # Mod manifest
├── hello.wat     # WebAssembly Text Format source
└── README.md     # This file
```

## Building

To build the WASM module from the WAT file:

```bash
# Install wasm tools if not already installed
cargo install wat2wasm

# Compile WAT to WASM
wat2wasm hello.wat -o mod.wasm
```

Or use the MCLRE asset converter:
```bash
cargo run --bin assetc -- convert hello.wat mod.wasm
```

## Mod Manifest

The `mod.toml` file contains:
- Mod metadata (id, name, version, description, authors, license)
- MCLRE version requirement
- Dependencies (empty for this example)
- Requested permissions
- Entry point function name
- Configuration schema

## Mod Code

The `hello.wat` file demonstrates:
- Importing host functions from `mclre:mod-api`
- Registering a custom block
- Event handlers for block break, block place, player join
- Tick handler
- Logging via the host

## Host Functions Used

- `world.get-block` / `world.set-block` - Block access
- `player.get-player` / `player.send-chat` - Player interaction
- `entity.get-entity` / `entity.spawn-entity` - Entity manipulation
- `registry.register-block` / `registry.register-item` / `registry.register-recipe` - Content registration
- `config.get-config` / `config.set-config` - Mod configuration
- `events.subscribe` / `events.unsubscribe` - Event subscription
- `utils.game-random` / `utils.log` / `utils.schedule-task` - Utilities

## Running

1. Build the mod: `wat2wasm hello.wat -o mod.wasm`
2. Place the `hello-mod` folder in your MCLRE `mods` directory
3. Start MCLRE server or singleplayer
4. The mod will load automatically and log "Hello from Hello Mod!"

## Permissions

This mod requests:
- `world_read` - Read block states
- `world_write` - Place/break blocks
- `player_interact` - Send chat messages

## API Version

This mod targets MCLRE mod API version 1.0.0.