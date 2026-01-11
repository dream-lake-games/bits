# Agent Instructions

## Stack

- Bevy 0.17.3 with lightyear 0.25.5 networking
- Rust 2024 edition
- Client (WASM) + headless server architecture

## Style

### Comments

- Avoid inline comments within function bodies
- Use comments sparingly — only on important functions and data structures
- Keep comments concise and clear

## Code Patterns

### Plugins

Prefer `*_plugin_fn` function style for simple plugins. Use Plugin structs when configuration is needed:

```rust
pub fn my_feature_plugin_fn(app: &mut App) {
    app.add_systems(Update, my_system);
}
```

### Naming

- Systems: descriptive verbs (`update_*`, `handle_*`, `check_*`)
- Queries: `*_q` suffix (`player_info_q`, `room_info_q`)
- Observers: `handle_*` for `On<Add, T>` triggers
- State enums: `*State` suffix

### Components

Marker components for state transitions:

```rust
#[derive(Component, Debug, Clone, Serialize, Deserialize, Reflect, PartialEq)]
pub struct QuestionActive { ... }
```

### Builders

Use `with_*` chainable methods:

```rust
ButtonSimple::medium("Click").with_on_press(|| { ... })
```

### Queries

Use `.single()` for singletons, early return on `Err`:

```rust
let Ok(room_info) = room_info_q.single() else { return; };
```

Add `warn!` if a singleton is expected to exist but doesn't. In critical paths, panic is acceptable:

```rust
let Ok(room_info) = room_info_q.single() else {
    warn!("RoomInfo should exist at this point");
    return;
};
```

### Examples

Standalone game logic, rendering, or UI features should have a corresponding example in `examples/` for fast iteration. See `examples/bg/` for reference.
