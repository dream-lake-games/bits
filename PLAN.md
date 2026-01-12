# Player Screen Refactor Plan

## Overview

Refactor the player client UI (`src/client/client_game.rs`) to use clean, data-driven screen components with one-shot outputs, matching the pattern established for host screens.

**Goals:**

1. Replace ~900 lines of closure-heavy code with clean data-driven components
2. Use `AnimatedText` pixel-art style (same as host) instead of `FlexSimple`/`ButtonSimple`
3. Make screens testable via standalone examples
4. Enable eventual removal of `FlexSimple`/`ButtonSimple` UI system

## Architecture

### Pattern: Data-Driven Screens with One-Shot Outputs

Each player screen component has three categories of fields:

```rust
#[derive(Component)]
pub struct PlayerGuessingScreen {
    // === DISPLAY DATA (set by update system from game state) ===
    pub question: String,
    pub units: Option<String>,
    pub seconds_remaining: Option<f32>,
    pub already_submitted: Option<u32>,  // None if not yet submitted

    // === LOCAL UI STATE (managed by screen's internal systems) ===
    pub pending_guess: String,  // Current number being typed

    // === OUTPUT (read by drain system, then cleared) ===
    pub submit_requested: bool,
}
```

**Data flow:**

1. **Update system** reads game state → writes to display data fields
2. **Screen's internal systems** handle user input → update local UI state and output fields
3. **Drain system** reads output fields → sends `ClientInput` to network → clears output flags

### Reference: Host Screen Pattern

See `src/client/client_game_host.rs` for the completed host refactor. Key points:

- Screens spawned with `commands.spawn((Name::new("..."), ScreenComponent::new(...)))`
- Observer handles `On<Add, ScreenComponent>` to spawn child UI elements
- Update system syncs display data when `Changed<ScreenComponent>`
- Clean separation between data and rendering

## Player Screens to Implement

### 1. PlayerWaitingScreen

**Location:** `src/player/waiting.rs`

**Display data:**

- None (just shows "Waiting for question..." with animated dots)

**Local state:**

- Animation timer for dots

**Output:**

- None

**Reference:** Current implementation at `src/client/client_game.rs:14-54`

---

### 2. PlayerGuessingScreen

**Location:** `src/player/guessing.rs`

**Display data:**

```rust
pub question: String,
pub units: Option<String>,
pub seconds_remaining: Option<f32>,
pub already_submitted: Option<u32>,  // Player's submitted guess, if any
```

**Local state:**

```rust
pub pending_guess: String,  // Current number being composed (e.g., "123")
```

**Output:**

```rust
pub submit_requested: bool,  // Set true when player taps submit
```

**UI Elements:**

- Question text (top)
- Timer text
- Number display (shows `pending_guess` or "0" if empty)
- Number pad (0-9, CLR, DEL buttons)
- Submit button

**States to handle:**

1. **Input mode:** Player composing guess (show numpad, submit button)
2. **Submitted mode:** Player already submitted (show "Submitted: {value}")
3. **Timed out mode:** Timer expired, no submission (show "No guess submitted")

**Reference:** Current implementation at `src/client/client_game.rs:56-335`

---

### 3. PlayerBettingScreen

**Location:** `src/player/betting.rs`

This is the most complex screen.

**Display data:**

```rust
pub question: String,
pub units: Option<String>,
pub seconds_remaining: Option<f32>,
pub player_score: u32,  // For validating paid bets
pub is_locked: bool,    // Whether this player has locked bets

// Available guesses to bet on
pub guesses: Vec<BettingGuessDisplay>,
```

Where:

```rust
#[derive(Clone, Default, Reflect)]
pub struct BettingGuessDisplay {
    pub value: u32,
    pub owners: Vec<String>,  // Who submitted this guess (empty for lowball)
    pub my_free_bets: u32,    // This player's free bets on this guess
    pub my_paid_bets: u32,    // This player's paid bets on this guess
}
```

**Local state:**

- None needed (bet values come from display data)

**Output:**

```rust
pub pending_bet: Option<PendingBet>,  // Set when +/- pressed
pub lock_requested: bool,              // Set when lock button pressed
```

Where:

```rust
#[derive(Clone, Reflect)]
pub struct PendingBet {
    pub guess: u32,
    pub num_free: u32,
    pub num_paid: u32,
}
```

**UI Elements:**

- Question text (top)
- Timer text
- Row of guess boxes (one per unique guess value, including lowball=0)
  - Each box shows: owner names, guess value, free bet controls, paid bet controls
- Lock button (bottom)

**Key constraints (for disabling buttons):**

- Can't modify bets if locked
- Free bets: max 2 total across all guesses
- Paid bets: can only add if free bet exists on same guess, total paid ≤ player score
- Minus buttons disabled at 0

**Update behavior:**

- When `guesses` list changes length, regenerate the guess box UI
- When `guesses` contents change (same length), update existing UI in place
- This avoids despawn/spawn churn during normal bet updates

**Reference:** Current implementation at `src/client/client_game.rs:337-804`

---

### 4. PlayerReviewingScreen

**Location:** `src/player/reviewing.rs`

**Display data:**

```rust
pub delta_this_round: i32,      // Points gained/lost
pub already_voted: bool,        // Whether player has voted to continue
pub seconds_until_auto: Option<f32>,  // Auto-continue timer
```

**Local state:**

- None

**Output:**

```rust
pub continue_requested: bool,  // Set when continue button pressed
```

**UI Elements:**

- "Round Summary" header
- Delta display ("+5" or "-3")
- Continue button (disabled if already voted)
- Timer showing seconds until auto-continue

**Reference:** Current implementation at `src/client/client_game.rs:806-885`

---

## Module Structure

Create new module at `src/player/`:

```
src/player/
  mod.rs              # Exports and plugin registration
  waiting.rs          # PlayerWaitingScreen
  guessing.rs         # PlayerGuessingScreen
  betting.rs          # PlayerBettingScreen
  reviewing.rs        # PlayerReviewingScreen
```

The `mod.rs` should export:

- All screen components
- All supporting types (BettingGuessDisplay, PendingBet, etc.)
- Plugin functions for each screen
- A combined `player_screens_plugin_fn`

---

## Integration into client_game.rs

After the player screens are implemented and tested:

### 1. Add plugins

```rust
use bits::player::{
    PlayerWaitingScreen, PlayerGuessingScreen, PlayerBettingScreen, PlayerReviewingScreen,
    player_waiting_plugin_fn, player_guessing_plugin_fn,
    player_betting_plugin_fn, player_reviewing_plugin_fn,
};

pub fn client_game_plugin_fn(app: &mut App) {
    app.add_plugins((
        player_waiting_plugin_fn,
        player_guessing_plugin_fn,
        player_betting_plugin_fn,
        player_reviewing_plugin_fn,
    ));
    // ... state transitions
}
```

### 2. Simplify enter/exit functions

```rust
fn on_enter_guessing(mut commands: Commands) {
    commands.spawn((
        Name::new("PlayerGuessingScreen"),
        PlayerGuessingScreen::new("Loading..."),
    ));
}

fn on_exit_guessing(q: Query<Entity, With<PlayerGuessingScreen>>, mut commands: Commands) {
    for ent in &q {
        commands.entity(ent).despawn();
    }
}
```

### 3. Add update systems (game state → display data)

```rust
fn update_guessing_display(
    mut screen_q: Query<&mut PlayerGuessingScreen>,
    question_q: Query<(&Question, &QuestionActive)>,
    connection_state: Res<State<ClientConnectionState>>,
) {
    let Ok(mut screen) = screen_q.single_mut() else { return };
    let Ok((question, active)) = question_q.single() else { return };

    screen.question = question.question.clone();
    screen.units = question.units.clone();
    screen.seconds_remaining = active.guess_seconds_remaining;

    // Check if this player already submitted
    if let ClientConnectionState::Named { username } = connection_state.get() {
        screen.already_submitted = question.guesses.get(username).copied();
    }
}
```

### 4. Add drain systems (output → network)

```rust
fn drain_guessing_outputs(
    mut screen_q: Query<&mut PlayerGuessingScreen>,
    mut inputs_queue: ResMut<InputsQueue>,
) {
    let Ok(mut screen) = screen_q.single_mut() else { return };

    if screen.submit_requested {
        if let Ok(guess) = screen.pending_guess.parse::<u32>() {
            inputs_queue.push(ClientInput::SubmitGuess { guess });
        }
        screen.submit_requested = false;
        screen.pending_guess.clear();
    }
}
```

### 5. Remove old code

Delete:

- All `*Cleanup` marker components
- All `*_text` system functions
- All inline closure button handlers
- Helper functions like `get_bet_for_user`, `is_user_locked`, etc.

---

## Examples

Create examples at `examples/player/`:

```
examples/player/
  guessing.rs
  betting.rs
  reviewing.rs
```

Each example should:

1. Spawn a screen with mock data
2. Show different states (e.g., before/after submission)
3. Log output requests to verify button interactions work

**Example structure:**

```rust
fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(bits_ui_plugin_fn)
        .add_plugins(player_guessing_plugin_fn)
        .add_systems(Startup, setup)
        .add_systems(Update, log_outputs)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
    commands.spawn((
        Name::new("PlayerGuessingScreen"),
        PlayerGuessingScreen::new("How many miles is the moon from Earth?")
            .with_units("miles")
            .with_seconds_remaining(45.0),
    ));
}

fn log_outputs(mut screen_q: Query<&mut PlayerGuessingScreen>) {
    let Ok(mut screen) = screen_q.single_mut() else { return };
    if screen.submit_requested {
        info!("Submit requested: {}", screen.pending_guess);
        screen.submit_requested = false;
    }
}
```

---

## UI Rendering Notes

### Use AnimatedText, not FlexSimple

All player screens should use `AnimatedText` and `SentenceList` for text display, matching the host screens. For buttons, we'll need to create interactive elements that work with the pixel-art style.

**Button approach options:**

1. Create a new `AnimatedButton` component that uses letter sprites
2. Use `AnimatedText` with click detection via Bevy's `Interaction` component
3. Simple approach: rectangular sprites with `AnimatedText` labels that detect clicks

The exact button implementation can be determined during development, but the goal is consistency with the pixel-art aesthetic.

### Sizing Reference

From `AGENTS.md`:

- `WINDOW_SIZE` = 800 (fixed window dimensions)
- `BUTTON_SIZE` = 64 (standard button size)
- `LETTER_SIZE` = 32 (base letter size)
- Small text: 16px (scale 0.5)
- Medium text: 32px (scale 1.0)
- Large text: 64px (scale 2.0)

### Update-in-place for Betting

For the betting screen, when the `guesses` vector changes:

- If length changes: despawn old guess box children, spawn new ones
- If length same: update existing `AnimatedText` content in place

This matches the pattern in `src/host/betting.rs:245-262`:

```rust
if current_guess_count != new_guess_count {
    // Despawn and respawn
} else {
    // Update existing by index
}
```

---

## Testing Checklist

Before integrating into `client_game.rs`, verify each screen:

- [ ] **PlayerWaitingScreen**
  - Renders animated "Waiting..." text
- [ ] **PlayerGuessingScreen**
  - Shows question, timer, number display
  - Number pad buttons update `pending_guess`
  - CLR clears, DEL removes last digit
  - Submit sets `submit_requested = true`
  - Shows "Submitted: X" when `already_submitted` is Some
- [ ] **PlayerBettingScreen**
  - Shows question, timer
  - Renders correct number of guess boxes
  - +/- buttons set `pending_bet`
  - Lock button sets `lock_requested`
  - Buttons disabled appropriately (locked, at limits)
  - Updates in place when bet values change
- [ ] **PlayerReviewingScreen**
  - Shows delta (+/- formatting)
  - Continue button sets `continue_requested`
  - Button disabled when `already_voted`

---

## Migration Path

1. **Phase 1:** Implement player screen components in `src/player/`
2. **Phase 2:** Create examples, verify each screen works standalone
3. **Phase 3:** Integrate into `client_game.rs`, keeping old code as fallback
4. **Phase 4:** Remove old `FlexSimple`/`ButtonSimple` code from `client_game.rs`
5. **Phase 5:** (Future) Remove `FlexSimple`/`ButtonSimple` from codebase entirely

---

## Questions for Implementation

1. **Button rendering:** What's the preferred approach for clickable buttons with pixel-art style?
2. **Number pad layout:** Keep the 3x4 grid, or simplify?
3. **Betting box layout:** Horizontal row (current) or different arrangement for small screens?

These can be decided during implementation based on what looks/feels best.
