# Hello Mod Tutorial

This tutorial walks you through creating your first MCLRE mod from scratch.

## Prerequisites

- Rust 1.88+ installed
- MCLRE source code cloned
- Basic understanding of Rust and WebAssembly

## Step 1: Create Mod Directory Structure

```bash
mkdir -p my_first_mod
cd my_first_mod
```

Create the following structure:
```
my_first_mod/
├── mod.toml
├── src/
│   └── lib.rs
├── Cargo.toml
└── build.rs
```

## Step 2: Create mod.toml

```toml
id = "tutorial:first_mod"
name = "First Mod Tutorial"
version = "1.0.0"
description = "A simple tutorial mod that adds a custom block and logs events"
authors = ["Your Name"]
license = "MIT"
mclre_version = "^1.0.0"

dependencies = []

permissions = [
    "world_read",
    "world_write",
    "player_interact",
]

entry_point = "mod_init"
api_version = "1.0.0"

config_schema = {
    "type": "object",
    "properties": {
        "greeting": {
            "type": "string",
            "default": "Hello from First Mod!",
            "description": "Message to display on join"
        },
        "block_hardness": {
            "type": "number",
            "default": 2.0,
            "description": "Hardness of custom block"
        }
    }
}
```

## Step 3: Create Cargo.toml

```toml
[package]
name = "first_mod"
version = "1.0.0"
edition = "2024"
description = "First Mod Tutorial for MCLRE"

[lib]
crate-type = ["cdylib"]

[dependencies]
mclre-mod-api = { path = "../../../crates/mod-api" }

[profile.release]
opt-level = "z"
lto = true
panic = "abort"
```

## Step 4: Create build.rs

```rust
fn main() {
    println!("cargo:rerun-if-changed=src/lib.rs");
    println!("cargo:rustc-link-arg=--export=mod_init");
    println!("cargo:rustc-link-arg=--export=tick");
    println!("cargo:rustc-link-arg=--export=mod_shutdown");
}
```

## Step 5: Create src/lib.rs

```rust
use mclre_mod_api::{
    host,
    BlockPos, BlockStateId, EventType, GameRandom, Identifier,
    LogLevel, Permission, PlayerHandle,
    ModResult, ModApiError,
};

/// Mod initialization - called once on load
#[no_mangle]
pub extern "C" fn mod_init() {
    // Register a custom block
    let block_id = Identifier::new("tutorial", "glowing_stone").unwrap();
    
    let result = host::registry::register_block(
        block_id.clone(),
        "Glowing Stone".to_string(),
        &[
            // Property: glowing (bool)
            host::BlockProperty {
                name: "glowing".to_string(),
                property_type: host::PropertyType::Bool,
                default_value: "true".to_string(),
                allowed_values: vec!["true".to_string(), "false".to_string()],
            },
        ],
    );
    
    match result {
        Ok(state_id) => {
            host::utils::log(LogLevel::Info, &format!("Registered block: {:?} with state ID: {}", block_id, state_id));
        }
        Err(e) => {
            host::utils::log(LogLevel::Error, &format!("Failed to register block: {}", e));
        }
    }
    
    // Register event handlers
    register_event_handlers();
    
    host::utils::log(LogLevel::Info, "First Mod initialized!");
}

/// Register event subscriptions
fn register_event_handlers() {
    // Subscribe to block break events
    let _ = host::events::subscribe(EventType::BlockBreak, |event_data| {
        if let Some(event) = event_data.as_block_break() {
            let msg = format!("Block broken: {:?} at {:?}", event.block_state, event.context.position);
            host::utils::log(LogLevel::Info, &msg);
        }
        Ok(())
    });
    
    // Subscribe to block place events
    let _ = host::events::subscribe(EventType::BlockPlace, |event_data| {
        if let Some(event) = event_data.as_block_place() {
            let msg = format!("Block placed: {:?} at {:?}", event.block_state, event.context.position);
            host::utils::log(LogLevel::Info, &msg);
        }
        Ok(())
    });
    
    // Subscribe to player join events
    let _ = host::events::subscribe(EventType::PlayerJoin, |event_data| {
        if let Some(event) = event_data.as_player_join() {
            let greeting = host::config::get_config("greeting")
                .ok()
                .flatten()
                .unwrap_or_else(|| "Welcome!".to_string());
            
            host::player::send_chat(Some(event.player), &format!("{} {}", greeting, event.username));
            host::utils::log(LogLevel::Info, &format!("Player joined: {}", event.username));
        }
        Ok(())
    });
}

/// Tick function - called every server tick (20 TPS)
#[no_mangle]
pub extern "C" fn tick() {
    // Use deterministic random for game logic
    let random = host::utils::game_random();
    // Example: 1 in 10000 chance per tick to log a message
    if random % 10000 == 0 {
        host::utils::log(LogLevel::Debug, "First Mod tick heartbeat");
    }
}

/// Mod shutdown - called on unload
#[no_mangle]
pub extern "C" fn mod_shutdown() {
    host::utils::log(LogLevel::Info, "First Mod shutting down!");
}
```

## Step 6: Build the Mod

```bash
# Build for WASM target
cargo build --target wasm32-wasip1 --release

# The output will be at target/wasm32-wasip1/release/first_mod.wasm
# Copy to mod directory
cp target/wasm32-wasip1/release/first_mod.wasm ./mod.wasm
```

## Step 7: Install and Test

```bash
# Create mods directory if not exists
mkdir -p /path/to/mclre/mods

# Copy mod folder
cp -r my_first_mod /path/to/mclre/mods/

# Start MCLRE server
cd /path/to/mclre
cargo run --bin mclre-server -- --mods-dir ./mods
```

## Step 8: Verify It Works

1. Join the server
2. You should see "Hello from First Mod!" in chat
3. Break/place blocks to see event logs
4. Check server console for initialization messages

## Understanding the Code

### Mod Entry Points

- `mod_init()` - Called once on load, use for registration
- `tick()` - Called every server tick (20 times/second)
- `mod_shutdown()` - Called on unload, cleanup resources

### Host Function Namespaces

| Namespace | Functions |
|-----------|-----------|
| `host::world` | `get_block`, `set_block` |
| `host::player` | `get_player`, `send_chat` |
| `host::entity` | `get_entity`, `spawn_entity` |
| `host::registry` | `register_block`, `register_item`, `register_recipe` |
| `host::config` | `get_config`, `set_config` |
| `host::events` | `subscribe`, `unsubscribe` |
| `host::utils` | `game_random`, `log`, `schedule_task` |

### Error Handling

All host functions return `ModResult<T>` which is `Result<T, ModApiError>`.

```rust
match host::world::get_block(pos) {
    Ok(state_id) => { /* success */ }
    Err(ModApiError::PermissionDenied(msg)) => { /* no permission */ }
    Err(ModApiError::NotFound(msg)) => { /* block not found */ }
    Err(e) => { /* other error */ }
}
```

### Event Handling

```rust
host::events::subscribe(EventType::BlockBreak, |event_data| {
    // Downcast to specific event type
    if let Some(event) = event_data.as_block_break() {
        // Access event fields
        let state = event.block_state;
        let pos = event.context.position;
        let tool = event.tool_item;
        
        // Cancel the event
        // event.cancelled = true; // Note: requires mutable access
    }
    Ok(())
});
```

### Configuration

```rust
// Get config value
let hardness = host::config::get_config("block_hardness")
    .ok()
    .flatten()
    .and_then(|s| s.parse::<f32>().ok())
    .unwrap_or(2.0);

// Set config value
host::config::set_config("last_player", "PlayerName");
```

## Next Steps

1. **Add more blocks/items** - Experiment with different block properties
2. **Create recipes** - Register crafting recipes for your items
3. **Handle more events** - Try EntityDamage, Explosion, Chat events
4. **Add configuration** - Make your mod customizable
5. **Package as mod** - Include mod.wasm and mod.toml in release

## Common Issues

| Issue | Solution |
|-------|----------|
| "Permission denied" | Add required permission to mod.toml |
| "Function not found" | Check export names in build.rs match function names |
| "Invalid argument" | Check parameter types match host function signature |
| Mod not loading | Check mod.toml syntax and file structure |

## Resources

- [API Reference](api-reference.md)
- [Content Packs Guide](content-packs.md)
- [MCLRE Specification](https://github.com/SlimyFrog123/mclre/blob/main/SPEC.md)
- [Example Mods](https://github.com/SlimyFrog123/mclre/tree/main/examples)

## Next Tutorial

Continue to [Advanced Modding](advanced-modding.md) to learn about:
- Custom entities and AI
- World generation hooks
- Network packets
- Client-side rendering