;; Hello Mod - WebAssembly Text Format
;; This is a simple example mod demonstrating the MCLRE mod API

(module
  ;; Import the host functions from mclre:mod-api
  (import "mclre:mod-api/world" "get-block" (func $world_get_block (param i32 i32 i32) (result i32)))
  (import "mclre:mod-api/world" "set-block" (func $world_set_block (param i32 i32 i32 i32)))
  (import "mclre:mod-api/player" "get-player" (func $player_get_player (param i64) (result i64)))
  (import "mclre:mod-api/player" "send-chat" (func $player_send_chat (param i64 string)))
  (import "mclre:mod-api/entity" "get-entity" (func $entity_get_entity (param i64) (result i64)))
  (import "mclre:mod-api/entity" "spawn-entity" (func $entity_spawn_entity (param string i64 i64 i64 string) (result i64)))
  (import "mclre:mod-api/registry" "register-block" (func $registry_register_block (param string string) (result i32)))
  (import "mclre:mod-api/registry" "register-item" (func $registry_register_item (param string string i32 i32) (result string)))
  (import "mclre:mod-api/registry" "register-recipe" (func $registry_register_recipe (param string string) (result)))
  (import "mclre:mod-api/config" "get-config" (func $config_get_config (param string) (result string)))
  (import "mclre:mod-api/config" "set-config" (func $config_set_config (param string string)))
  (import "mclre:mod-api/events" "subscribe" (func $events_subscribe (param i32 i64) (result i64)))
  (import "mclre:mod-api/events" "unsubscribe" (func $events_unsubscribe (param i64)))
  (import "mclre:mod-api/utils" "game-random" (func $utils_game_random (result i64)))
  (import "mclre:mod-api/utils" "log" (func $utils_log (param i32 string)))
  (import "mclre:mod-api/utils" "schedule-task" (func $utils_schedule_task (param i64 i64) (result i64)))

  ;; Memory
  (memory 1)
  (export "memory" (memory 0))

  ;; Data section for strings
  (data (i32.const 0) "hello:custom_block")
  (data (i32.const 100) "Custom Block")
  (data (i32.const 200) "Hello from Hello Mod!")
  (data (i32.const 300) "Block broken: ")
  (data (i32.const 400) "Block placed: ")
  (data (i32.const 500) "Player joined: ")
  (data (i32.const 600) "greeting_message")

  ;; Custom block state ID (will be set by register_block)
  (global $custom_block_state (mut i32) (i32.const 0))
  
  ;; Block break event handler
  (func $on_block_break (param i32 i32 i64)
    (local $block_state i32)
    (local $message_ptr i32)
    (local $message_len i32)
    
    ;; Get block state from event data
    local.get 0
    local.set $block_state
    
    ;; Log the event
    local.get 0
    call $utils_game_random
    drop
    
    ;; Create message: "Block broken: " + block_state
    i32.const 300  ;; "Block broken: "
    i32.const 14
    local.get $block_state
    call $utils_log
  )

  ;; Block place event handler
  (func $on_block_place (param i32 i32 i64)
    local.get 0
    call $utils_game_random
    drop
    
    i32.const 400  ;; "Block placed: "
    i32.const 14
    local.get 0
    call $utils_log
  )

  ;; Player join event handler
  (func $on_player_join (param i64)
    i32.const 500  ;; "Player joined: "
    i32.const 13
    local.get 0
    call $utils_log
  )

  ;; Tick event handler (called every tick)
  (func $on_tick (param i64)
    ;; Could do periodic tasks here
    nop
  )

  ;; Initialize the mod
  (func $mod_init (export "mod_init")
    ;; Register a custom block
    i32.const 0  ;; "hello:custom_block"
    i32.const 100 ;; "Custom Block"
    call $registry_register_block
    global.set $custom_block_state
    
    ;; Register event handlers
    ;; Note: In a real implementation, these would be registered via the events interface
    ;; For now, we just log that the mod is initialized
    i32.const 200  ;; "Hello from Hello Mod!"
    i32.const 22
    i32.const 2   ;; Info level
    call $utils_log
  )

  ;; Tick function (called every server tick)
  (func $mod_tick (export "tick")
    i64.const 0
    call $on_tick
  )

  ;; Shutdown function
  (func $mod_shutdown (export "mod_shutdown")
    i32.const 200  ;; "Hello from Hello Mod!"
    i32.const 22
    i32.const 3   ;; Warn level
    call $utils_log
  )
)