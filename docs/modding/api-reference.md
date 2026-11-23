# MCLRE Modding API Reference

This document provides a complete reference for the MCLRE modding API, available to WASM mods via the `mclre:mod-api` component model interface.

## Table of Contents

1. [Core Types](#core-types)
2. [Permissions](#permissions)
3. [Event System](#event-system)
4. [Host Interfaces](#host-interfaces)
5. [Error Handling](#error-handling)
6. [Mod Entry Points](#mod-entry-points)

---

## Core Types

### Identifier
```wit
type identifier = string;
```
A namespaced identifier in the format `namespace:name` (e.g., `minecraft:diamond`, `hello:custom_block`).

### BlockStateId
```wit
type block-state-id = u32;
```
Dense identifier for a block state (block + property values).

### EntityHandle / PlayerHandle
```wit
type entity-handle = u64;
type player-handle = u64;
```
Opaque handles for entities and players.

### BlockPos / ChunkPos
```wit
type block-pos = record { x: s32, y: s32, z: s32 };
type chunk-pos = record { x: s32, z: s32 };
```
Block and chunk coordinates.

### Vec3
```wit
type vec3 = record { x: f64, y: f64, z: f64 };
```
3D position with double precision.

---

## Permissions

```wit
enum permission {
    world-read,         // Read-only world access (get-block)
    world-write,        // Read-write world access (set-block)
    player-interact,    // Player interaction (chat, inventory)
    entity-manipulate,  // Entity manipulation (spawn, modify)
    world-gen,          // World generation hooks
    network,            // Network packet access
    config,             // Mod configuration access
}
```

Permissions are declared in `mod.toml` and granted by the host at load time.

---

## Event System

### Event Types
```wit
enum event-type {
    tick,              // Every server tick (20 TPS)
    block-break,       // Block broken by player
    block-place,       // Block placed by player
    entity-damage,     // Entity damaged
    player-join,       // Player joined the server
    player-leave,      // Player left the server
    chunk-gen,         // Chunk generated
    item-use,          // Item used by player
    explosion,         // Explosion occurred
    chat,              // Player chat message
    player-respawn,    // Player respawned
    item-drop,         // Item dropped
    entity-spawn,      // Entity spawned
}
```

### Event Context
```wit
type event-context = record {
    event-type: event-type,
    tick: u64,
    player: option<player-handle>,
    position: option<block-pos>,
};
```

### Event Data Structures

Each event type has associated data:

- **BlockBreakEvent**: `context`, `block-state: block-state-id`, `tool-item: option<identifier>`, `cancelled: bool`
- **BlockPlaceEvent**: `context`, `block-state`, `replaced-state`, `cancelled`
- **EntityDamageEvent**: `context`, `target: entity-handle`, `source: option<entity-handle>`, `amount: f32`, `damage-type: identifier`, `cancelled`
- **PlayerJoinEvent**: `context`, `player: player-handle`, `username: string`, `uuid: string`
- **ChunkGenEvent**: `context`, `chunk-pos: chunk-pos`, `biome-data: list<u8>`
- **ItemUseEvent**: `context`, `item: identifier`, `hand: hand`, `cancelled`
- **ExplosionEvent**: `context`, `position: vec3`, `radius: f32`, `cancelled`
- **ChatEvent**: `context`, `message: string`, `cancelled`
- **PlayerRespawnEvent**: `context`, `player: player-handle`, `position: block-pos`
- **ItemDropEvent**: `context`, `item: identifier`, `count: u8`, `position: vec3`
- **EntitySpawnEvent**: `context`, `entity: entity-handle`, `entity-type: identifier`, `position: vec3`
- **TickEvent**: `context` (tick event context)

### Cancellation
Events with `cancelled: bool` can be cancelled by setting `cancelled = true` in the event handler. The host will respect cancellation and prevent the default action.

---

## Host Interfaces

All host functions are accessed via the component model interface `mclre:mod-api`.

### World Interface (`mclre:mod-api/world`)

#### `get-block(pos: block-pos) -> result<block-state-id, host-error>`
Get the block state at a position.

#### `set-block(pos: block-pos, state: block-state-id) -> result<_, host-error>`
Set a block at a position. Requires `world_write` permission.

---

### Player Interface (`mclre:mod-api/player`)

#### `get-player(handle: player-handle) -> result<player-info, host-error>`
Get player information by handle.

#### `send-chat(handle: option<player-handle>, message: string) -> result<_, host-error>`
Send a chat message. If handle is `none`, broadcasts to all players. Requires `player_interact` permission.

**PlayerInfo**:
```wit
type player-info = record {
    handle: player-handle,
    username: string,
    uuid: string,
    position: vec3,
    yaw: f32,
    pitch: f32,
    health: f32,
    max-health: f32,
    food-level: s32,
    saturation: f32,
    game-mode: game-mode,
};
```

---

### Entity Interface (`mclre:mod-api/entity`)

#### `get-entity(handle: entity-handle) -> result<entity-info, host-error>`
Get entity information by handle.

#### `spawn-entity(entity-type: identifier, position: vec3, nbt: option<string>) -> result<entity-handle, host-error>`
Spawn a new entity. Requires `entity_manipulate` permission.

**EntityInfo**:
```wit
type entity-info = record {
    handle: entity-handle,
    entity-type: identifier,
    position: vec3,
    yaw: f32,
    pitch: f32,
    health: f32,
    max-health: f32,
    is-alive: bool,
};
```

---

### Registry Interface (`mclre:mod-api/registry`)

#### `register-block(id: identifier, name: string, properties: list<block-property>) -> result<block-state-id, host-error>`
Register a new block type. Requires `world_write` permission.

**BlockProperty**:
```wit
type block-property = record {
    name: string,
    property-type: property-type,
    default-value: string,
    allowed-values: list<string>,
};
```

**PropertyType**:
```wit
type property-type = variant {
    bool,
    int(tuple<s32, s32>),  // min, max
    enum(list<string>),
};
```

#### `register-item(id: identifier, name: string, max-stack: u8, durability: option<u16>) -> result<identifier, host-error>`
Register a new item type. Requires `world_write` permission.

#### `register-recipe(id: identifier, recipe-type: string, ingredients: list<recipe-ingredient>, result: recipe-result) -> result<_, host-error>`
Register a crafting recipe. Requires `world_write` permission.

**RecipeIngredient**:
```wit
type recipe-ingredient = record {
    item: identifier,
    count: u8,
    tag: option<identifier>,  // Optional tag instead of specific item
};
```

**RecipeResult**:
```wit
type recipe-result = record {
    item: identifier,
    count: u8,
};
```

---

### Config Interface (`mclre:mod-api/config`)

#### `get-config(key: string) -> result<option<string>, host-error>`
Get a configuration value for this mod.

#### `set-config(key: string, value: string) -> result<_, host-error>`
Set a configuration value for this mod.

---

### Events Interface (`mclre:mod-api/events`)

#### `subscribe(event-type: event-type, callback: func(event-data) -> result<_, host-error>) -> result<u64, host-error>`
Subscribe to an event type. Returns a subscription ID.

The callback receives `event-data` (union of all event types) and returns `result<_, host-error>`. Return `err` to cancel cancellable events.

#### `unsubscribe(subscription-id: u64) -> result<_, host-error>`
Unsubscribe from an event by subscription ID.

---

### Utils Interface (`mclre:mod-api/utils`)

#### `game-random() -> u64`
Get a deterministic random value from the server's `GameRandom`.

#### `log(level: log-level, message: string)`
Log a message to the server console.

**LogLevel**:
```wit
enum log-level {
    trace, debug, info, warn, error,
};
```

#### `schedule-task(ticks: u64, callback: func() -> result<_, host-error>) -> result<u64, host-error>`
Schedule a function to run after N ticks. Returns a task ID.

---

## Error Handling

```wit
type host-error = variant {
    unavailable(string),      // Host function not available
    invalid-argument(string), // Invalid argument provided
    permission-denied(string),// Permission not granted
    not-found(string),        // Resource not found
    internal(string),         // Internal host error
};

type result<T, E> = variant { ok(T), err(E) };
```

All host functions return `result<T, host-error>`. Check for `err` variant and handle appropriately.

---

## Mod Entry Points

Mods must export the following functions:

### `init()`
Called once when the mod is loaded. Use for registration and initialization.

### `tick()`
Called every server tick (20 TPS). Use for periodic tasks.

### `shutdown()`
Called when the mod is unloaded. Use for cleanup.

---

## Mod Manifest (`mod.toml`)

```toml
id = "namespace:name"           # Required: Unique mod identifier
name = "Display Name"           # Required: Human-readable name
version = "1.0.0"               # Required: SemVer version
description = "..."             # Required: Mod description
authors = ["Author"]            # Required: Author list
license = "MIT"                 # Required: SPDX license
mclre_version = "^1.0.0"        # Required: MCLRE version requirement

dependencies = []               # Optional: List of { id, version, optional }

permissions = [                 # Optional: Requested permissions
    "world_read",
    "world_write",
]

entry_point = "mod_init"        # Optional: WASM export name (default: "mod_init")

config_schema = { ... }         # Optional: JSON Schema for config

api_version = "1.0.0"           # Optional: Minimum API version
```

---

## Example: Hello Mod

See `examples/hello-mod/` for a complete example that:
- Registers a custom block
- Handles block break/place events
- Logs player join events
- Uses the config system

### Building the Example

```bash
# Install wat2wasm
cargo install wat2wasm

# Compile
wat2wasm examples/hello-mod/hello.wat -o examples/hello-mod/mod.wasm

# Or use the asset converter
cargo run --bin assetc -- convert examples/hello-mod/hello.wat examples/hello-mod/mod.wasm
```

### Installing

Place the `hello-mod` folder (with `mod.toml` and `mod.wasm`) in the server's `mods/` directory.

---

## Version History

| API Version | MCLRE Version | Changes |
|-------------|---------------|---------|
| 1.0.0       | 1.0.0         | Initial release |

---

## See Also

- [Content Packs Guide](content-packs.md)
- [Hello Mod Tutorial](hello-mod-tutorial.md)
- [MCLRE Specification](https://github.com/SlimyFrog123/mclre/blob/main/SPEC.md)