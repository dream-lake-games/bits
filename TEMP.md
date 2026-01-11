# Multiplayer Architecture Notes

## Overview

We need to evolve from the current architecture (server acts as host with UI) to a production architecture where:
- Server runs headless
- The client that creates a room becomes the "host" (sees host UI, can start game, add AI)
- Room creation/joining is handled via a separate lobby service

---

## 1. Client as Host (Moving Host Logic to Client)

### Architecture Change

Currently:
```
Server (authoritative + host UI) ←→ Clients (just viewers/input)
```

Target:
```
Server (authoritative, headless) ←→ Host Client (host UI + host inputs) + Regular Clients
```

### Implementation Strategy

**Step 1: Mark one client as "host"**

Add a `RoomInfo` component/resource on the server that tracks which peer is the host:

```rust
#[derive(Component, Serialize, Deserialize, Clone, Debug, Reflect)]
pub struct RoomInfo {
    pub host_peer_id: Option<PeerId>,
    pub room_code: String,
}
```

Replicate this to all clients. Then on the client side:

```rust
fn am_i_host(
    room_info: Query<&RoomInfo>,
    local_id: Query<&LocalId, With<Connected>>,
) -> bool {
    let Ok(room) = room_info.single() else { return false };
    let Ok(local) = local_id.single() else { return false };
    room.host_peer_id == Some(local.0)
}
```

**Step 2: Move host-only UI to client**

The existing `host/host_game.rs` code is already well-structured with Bevy states. To migrate:

1. Move it to the client crate
2. Add `.run_if(am_i_host)` conditions to all those systems
3. Non-host clients simply won't run those systems

**Step 3: Host-only inputs via protocol**

For inputs like "Start Game" or "Add AI", two options:

**Option A: Host-only `ClientInput` variants**

```rust
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ClientInput {
    // ... existing ...
    StartGame,                    // Host-only
    AddAI { name: String },       // Host-only  
    RemoveAI { name: String },    // Host-only
}
```

On the server, validate the sender is the host:
```rust
fn handle_host_input(
    room_info: Res<RoomInfo>,
    inputs: Query<(&RemoteId, &ActionState<WrappedClientInput>)>,
) {
    for (remote_id, input) in &inputs {
        match &input.payload {
            ClientInput::StartGame => {
                if room_info.host_peer_id == Some(remote_id.0) {
                    // Do the thing
                }
            }
            // ...
        }
    }
}
```

**Option B: Separate channel for host commands** (cleaner separation of concerns)

```rust
pub struct HostCommandChannel;

#[derive(Serialize, Deserialize, Clone)]
pub enum HostCommand {
    StartGame,
    AddAI { name: String },
    RemoveAI { name: String },
}
```

**Step 4: Make server headless**

```rust
// server/main.rs
fn main() {
    let mut app = App::new();
    
    // Headless - no window, no rendering
    app.add_plugins(MinimalPlugins)
        .add_plugins(bevy::app::ScheduleRunnerPlugin::run_loop(
            Duration::from_secs_f64(1.0 / 60.0)
        ));
    
    // Keep networking, game logic, AI
    app.add_plugins((
        ProtocolPlugin,
        server_ai::server_ai_plugin_fn,
        server_game::server_game_plugin_fn,
        server_lobby::server_lobby_plugin_fn,
        server_simple::server_simple_plugin_fn,
        server_state::server_state_plugin_fn,
        server_question::server_question_plugin_fn,
    ));
    
    app.run();
}
```

**Key insight:** All host UI code already reads from replicated components (`PlayerInfo`, `Question`, `Bets`, etc.). It'll work on the client unchanged - we're just moving where it runs.

---

## 2. Room Creation Architecture

For k8s deployment, use a **matchmaking service** pattern:

### Overall Architecture

```
┌──────────────────┐
│   Lobby/API      │  ← Known address, stable
│   (HTTP/REST)    │
└────────┬─────────┘
         │ manages
    ┌────┴────┐
    ▼         ▼
┌────────┐ ┌────────┐
│ Room 1 │ │ Room 2 │  ← Game servers (WebTransport)
│ (Pod)  │ │ (Pod)  │     Dynamic, ephemeral
└────────┘ └────────┘
```

### Components

**1. Lobby Service (new binary)**

Simple HTTP server that:
- Maintains list of available rooms
- Creates rooms (spins up game server pods)
- Returns connection info to clients

```rust
// lobby/main.rs - could be axum, actix-web, or even just hyper
#[derive(Serialize)]
struct RoomInfo {
    room_code: String,
    server_address: String,
    cert_hash: String,  // For WebTransport
}

async fn create_room() -> Result<RoomInfo> {
    // 1. Generate room code
    let room_code = generate_room_code();
    
    // 2. Spin up game server (k8s Job or Deployment)
    let server_address = spawn_game_server(&room_code).await?;
    
    // 3. Wait for it to be ready, get cert hash
    let cert_hash = wait_for_ready(&server_address).await?;
    
    Ok(RoomInfo { room_code, server_address, cert_hash })
}

async fn join_room(room_code: String) -> Result<RoomInfo> {
    // Look up room in your store (Redis, in-memory, etc.)
    get_room_info(&room_code)
}
```

**2. Game Server Registration**

When a game server starts, it registers with the lobby:

```rust
// In server startup
async fn register_with_lobby(room_code: &str, my_address: &str, cert_hash: &str) {
    let client = reqwest::Client::new();
    client.post("http://lobby-service/register")
        .json(&json!({
            "room_code": room_code,
            "address": my_address,
            "cert_hash": cert_hash
        }))
        .send()
        .await?;
}
```

**3. Health checks & cleanup**

Game servers should:
- Send heartbeats to lobby
- Report when room is empty/game is done
- Lobby cleans up stale rooms

### K8s Options for Spawning Game Servers

**Option A: Pre-scaled pool**
- Keep N game server pods running
- When room is requested, assign an idle one
- Simpler, predictable costs

**Option B: On-demand Jobs**
- Create a k8s Job for each room
- More resource-efficient
- Slightly more complex

**Option C: Agones** (recommended for games!)
- https://agones.dev/
- Purpose-built for game server orchestration on k8s
- Handles allocation, health, scaling

---

## 3. Room Joining

### Client Flow

```
1. Client → Lobby: POST /rooms/create (if creating)
   OR
   Client → Lobby: GET /rooms/{code} (if joining)

2. Lobby → Client: { server_address, cert_hash, room_code }

3. Client → Game Server: WebTransport connect (using cert_hash)

4. Game Server: First client that connects and matches room_code becomes host
```

### Client Code

```rust
// Before connecting to game server
async fn get_room_connection(room_code: Option<String>) -> Result<ConnectionInfo> {
    let client = reqwest::Client::new();
    
    let info = if let Some(code) = room_code {
        // Joining existing room
        client.get(&format!("{}/rooms/{}", LOBBY_URL, code))
            .send().await?
            .json().await?
    } else {
        // Creating new room
        client.post(&format!("{}/rooms/create", LOBBY_URL))
            .send().await?
            .json().await?
    };
    
    Ok(info)
}
```

---

## Recommended Reading

1. **Gaffer On Games** - [Networking for Game Programmers](https://gafferongames.com/categories/game-networking/)
   - Glenn Fiedler's classic series. Essential reading.

2. **Agones Documentation** - https://agones.dev/site/docs/
   - Even if you don't use Agones, their architecture docs explain the patterns well.

3. **Bevy Cheatbook Networking** - https://bevy-cheatbook.github.io/fundamentals/networking.html
   - Covers common patterns

4. **"I Made a Multiplayer Game"** posts on r/gamedev
   - Real war stories about room/lobby architecture

5. **Photon Realtime docs** (Unity-focused, but patterns apply)
   - Their matchmaking/room architecture is well-documented

---

## Suggested Order of Implementation

1. **First: Make server headless** (easy win, clears the path)
2. **Second: Move host UI to client with `am_i_host` checks** (most complex, but contained)
3. **Third: Build minimal lobby service** (start with in-memory storage)
4. **Fourth: K8s deployment** (once you have the pieces working locally)

The client-as-host work is isolated and won't break local dev flow. Can keep running with a windowed "debug server" that acts as host for development while building toward the real architecture.

---

## Notes

- The existing host UI in `host/host_game.rs` already reads from replicated components, so migration to client should be straightforward
- Consider keeping a `--debug` flag on server to optionally enable windowed mode for local development
- WebTransport cert hash needs to be communicated from lobby to clients for secure connections

