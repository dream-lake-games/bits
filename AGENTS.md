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

## UI Layout

The UI uses a fixed-size window with manual coordinate math for positioning. The coordinate system is centered at `(0, 0)`, with coordinates ranging from `-WINDOW_SIZE/2` to `WINDOW_SIZE/2` on each axis.

### Helpful Constants

When positioning UI elements, prefer using the available size constants where it makes sense:

- `WINDOW_SIZE` (800) - the fixed window dimensions from `window.rs`
- `BUTTON_SIZE` (64) - standard button size from `bits_ui`
- `LETTER_SIZE` (32) - base letter size from `bits_ui`

Common derived values (often defined locally where needed):

```rust
const HALF_WINDOW: f32 = WINDOW_SIZE as f32 / 2.0;
const PADDING: f32 = 16.0;
const HALF_LETTER: f32 = LETTER_SIZE as f32 / 2.0;
```

### Text Sizing

Text is a bit more involved since `AnimatedText` supports different sizes (`Small`, `Medium`, `Large`) with scale factors of 0.5, 1.0, and 2.0 respectively. The effective letter size is `LETTER_SIZE * scale`. When calculating text widths, multiply the character count by the effective letter size.

### Positioning Examples

```rust
// Position something at the top with padding
let top_y = HALF_WINDOW - PADDING - element_half_height;

// Center something horizontally
let center_x = 0.0;

// Position in the left half
let left_center_x = -HALF_WINDOW / 2.0;
```

This approach keeps layouts predictable and makes it straightforward to reason about where things will appear.
