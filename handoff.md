# Handoff: Headless Server & Client Role Selection

## Current State (Jan 10, 2026)

### What's Done

**Server is now headless:**
- Uses `DefaultPlugins` with `WinitPlugin` disabled (no window)
- Runs at 60fps via `ScheduleRunnerPlugin`
- Prints certificate hash on startup for client configuration
- Shuts down automatically when host client disconnects

**Host is NOT a player:**
- Host peer is tracked in `RoomInfo.host_peer_id` (NOT in `PlayerInfo`)
- Host skips the name entry flow entirely
- Host's only job: display game state, control flow (add AI, start game)
- Server explicitly skips adding host peer to `unnamed_players`

**Client role selection flow:**
- Client starts with animated "FERMI" title + "H" and "P" buttons
- After selection, client connects to server
- Host → goes straight to Host Lobby (no name entry)
- Player → goes through name entry → Player Lobby

**Animation-based UI (new!):**
- All pre-game UI now uses sprite-based `AnimatedText` + `Button`
- Characters type out with animation effect
- `BgMarker` spawns animated starfield background
- Bloom camera for visual flair

**State machine:**
- `ClientRoleState`: `Selecting` → `Host` | `Player`
- `ClientConnectionState`: 
  - Host: `Disconnected` → `Connecting` → `Connected`
  - Player: `Disconnected` → `Connecting` → `Unnamed` → `Named`
- Server validates all host commands against `RoomInfo.host_peer_id`

### Key Files Changed

| File | Changes |
|------|---------|
| `src/server/main.rs` | Headless setup with disabled WinitPlugin |
| `src/server/server_simple.rs` | RoomInfo spawn, host disconnect detection |
| `src/server/server_lobby.rs` | Host command handling, skip host in unnamed_players |
| `src/protocol.rs` | Added `RoomInfo`, extended `ClientInput` enum |
| `src/client/client_state.rs` | `ClientRoleState`, new `Connected` state for host |
| `src/client/client_simple.rs` | Bloom camera, BgMarker, deferred connection |
| `src/client/client_lobby.rs` | Complete rewrite with AnimatedText + Button |

### How to Test

```bash
# Terminal 1 - Start server (prints cert hash)
cargo run --bin server

# Terminal 2 - Host client
cargo run --bin client
# Click "H" button, see Host Lobby with Add AI (+) and Start (S) buttons

# Terminal 3 - Player client
cargo run --bin client
# Click "P" button, enter 3-letter initials, see Player Lobby
```

### UI Controls

**Role Selection:**
- `H` button → Join as Host
- `P` button → Join as Player

**Name Entry (Player only):**
- Letter buttons → Type initials (max 3)
- `` ` `` (Clear) → Clear all
- `~` (Back) → Backspace
- `+` (Plus) → Submit

**Host Lobby:**
- `+` → Add AI player
- `S` → Start game (requires 2+ players)

### Certificate Note

Server auto-generates self-signed certs in `certs/`. If connection fails with "Expired" error:
1. Delete `certs/local_cert.pem` and `certs/local_key.pem`
2. Restart server (generates new cert, prints new hash)
3. Update hash in `src/client/client_simple.rs` → `certificate_digest`

### Next Steps (Not Yet Implemented)

1. **Room Creation** - Matchmaking service, room registration
2. **Room Joining** - Client discovers/connects to specific rooms
3. **Multiple Rooms** - K8s deployment with room pods

See `TEMP.md` for architectural notes on room creation/joining patterns.
