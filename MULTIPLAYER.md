# Multiplayer Room System

## Architecture

```
Client → Lobby Service (HTTP) → Kubernetes Jobs (Game Servers)
```

**Lobby**: Stable HTTP endpoint that creates and manages game rooms  
**Game Servers**: Ephemeral Kubernetes Jobs, one per room, spawned on-demand

```
┌──────────┐
│  Client  │
└────┬─────┘
     │ POST /rooms/create
     ▼
┌──────────────┐
│    Lobby     │ ← HTTP server, stable address
│  (Pod + Svc) │
└──────┬───────┘
       │ creates
   ┌───┴────┐
   ▼        ▼
┌──────┐ ┌──────┐
│ Job  │ │ Svc  │ ← Game server (ephemeral)
│LAMP  │ │LAMP  │    UDP port 9000, LoadBalancer
└──┬───┘ └──────┘
   │ starts
   ▼
┌──────────────┐
│ Server Pod   │ ← Runs with --room-code LAMP
│              │   Registers cert hash with lobby
└──────────────┘
```

## What's Implemented

### Lobby Service (`src/lobby/`)

Complete HTTP server using Axum that manages game server lifecycle via Kubernetes API.

**Modules:**
- `main.rs` - Axum HTTP server setup with state management
- `handlers.rs` - HTTP endpoint handlers
- `k8s.rs` - Kubernetes client and resource creation (Jobs, Services)
- `room_store.rs` - In-memory room state storage (thread-safe Arc<RwLock>)
- `room_code.rs` - 4-letter room code generation

**HTTP Endpoints:**
- `GET /health` - Health check
- `POST /rooms/create` - Creates new room (spawns k8s Job + LoadBalancer Service)
- `POST /rooms/register` - Game server calls this with cert hash
- `GET /rooms/{code}` - Returns room status and connection info

**Flow:**
1. Client calls `POST /rooms/create`
2. Lobby generates 4-letter room code (e.g., "LAMP")
3. Lobby creates two Kubernetes resources:
   - Job named `server-lamp` with args `--room-code LAMP --lobby-url http://lobby:8080`
   - LoadBalancer Service named `server-lamp` exposing UDP port 9000
4. Lobby spawns background task to poll for LoadBalancer external IP
5. Server pod starts, generates TLS certificate, extracts cert hash
6. Server POSTs to `/rooms/register` with room code and cert hash
7. Lobby updates room status to "ready" with server address and cert hash
8. Client polls `GET /rooms/LAMP` until status changes to "ready"
9. Client connects to game server using returned address and cert hash

### Server Registration (`src/server/`)

Headless game server with CLI args and lobby registration.

**Changes to `main.rs`:**
- Added `clap` CLI parsing with `Args` resource
- Required: `--room-code` 
- Optional: `--lobby-url` (defaults to `http://lobby:8080`)

**Changes to `server_simple.rs`:**
- `CertHash` resource stores certificate fingerprint from lightyear's `.hash()` method
- `register_with_lobby` system runs once after `Started` event
- Sends HTTP POST to lobby with room code and cert hash

**Server lifecycle:**
1. Starts with `--room-code ABCD --lobby-url http://lobby:8080`
2. Generates/loads TLS certificate (existing code)
3. Extracts hash using lightyear's certificate API
4. Stores as `CertHash` resource
5. Waits for `Started` event, then registers with lobby
6. Continues normal operation (accepts connections, runs game logic)

### Helm Chart (`helm/bits/`)

Complete Kubernetes deployment manifests with RBAC.

**Lobby resources:**
- `ServiceAccount` - `bits-lobby`
- `Role` - Permissions to create/delete Jobs and Services, list Pods
- `RoleBinding` - Binds role to service account
- `Deployment` - Single replica lobby server
- `Service` - LoadBalancer exposing port 8080

**Environment variables:**
- `NAMESPACE` - Which namespace to spawn game servers in
- `LOBBY_URL` - URL for game servers to register with
- `RUST_LOG` - Log level

### Dependencies Added

**Lobby-specific:**
- `axum = "0.8"` - HTTP server framework
- `kube = "0.98"` - Kubernetes client library
- `k8s-openapi = "0.24"` - Kubernetes API types
- `tracing = "0.1"` - Structured logging
- `tracing-subscriber = "0.3"` - Log formatting/filtering
- `tokio = { version = "1", features = ["full"] }` - Async runtime

**Server-specific:**
- `clap = { version = "4.5", features = ["derive"] }` - CLI argument parsing
- `reqwest = "0.12"` - HTTP client for registration

## Testing Locally

**Fast iteration (recommended during development):**
```bash
# Terminal 1: Run lobby
cargo run --bin lobby

# Terminal 2: Run server manually
cargo run --bin server -- --room-code TEST --lobby-url http://localhost:8080

# Terminal 3: Test API
curl -X POST http://localhost:8080/rooms/create
curl http://localhost:8080/rooms/TEST
```

**Testing without k8s:**
- Lobby runs but can't create Jobs/Services (logs errors, continues running)
- Useful for testing HTTP endpoints, room code generation, state management
- Server can be run manually with CLI args for testing registration flow

## Future Work

### 1. Docker Build Setup

Create single Dockerfile with multi-stage builds for fast iteration.

**Requirements:**
- Multi-stage: builder stage + separate lobby/server runtime stages
- Debug builds (faster compilation, good enough for development/testing)
- Dependency layer caching - if only `src/` changes, rebuild should be fast (~1-2 min)
- Simple and understandable

**Dockerfile strategy:**
```dockerfile
# Stage 1: Build dependencies only
FROM rust:nightly AS deps
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/lib.rs
RUN cargo build --bin lobby --bin server
# ^ This layer caches all dependencies

# Stage 2: Build actual code
FROM deps AS builder
COPY src ./src
RUN cargo build --bin lobby --bin server
# ^ Only this layer rebuilds when src/ changes

# Stage 3: Lobby runtime
FROM debian:bookworm-slim AS lobby
COPY --from=builder /app/target/debug/lobby /usr/local/bin/
CMD ["lobby"]

# Stage 4: Server runtime
FROM debian:bookworm-slim AS server
COPY --from=builder /app/target/debug/server /usr/local/bin/
CMD ["server"]
```

### 2. End-to-End Testing

Deploy to minikube and test full flow:
1. Deploy with `helm install bits ./helm/bits`
2. Port-forward: `kubectl port-forward svc/lobby 8080:8080`
3. Create room: `curl -X POST http://localhost:8080/rooms/create`
4. Verify Job created: `kubectl get jobs`
5. Verify Pod running: `kubectl get pods`
6. Check logs: `kubectl logs job/server-{code}`
7. Verify registration: `curl http://localhost:8080/rooms/{CODE}`
8. Should see status=ready with server address and cert hash

### 3. Client Integration

Update client to use lobby instead of hardcoded server.

**Client changes needed:**
- New state: `ClientLobbyState` (creating room vs joining existing)
- Room creation UI: button to create new room
- Room joining UI: input field for room code
- HTTP client to call lobby API
- Parse response and extract server address + cert hash
- Connect to game server using returned info

**Flow:**
```rust
// Before: hardcoded
let server_addr = "127.0.0.1:9000";
let cert_hash = "abc123...";

// After: from lobby
let room_info = create_or_join_room(room_code).await?;
let server_addr = room_info.server_address;
let cert_hash = room_info.cert_hash;
```

### 4. Server Lifecycle Management

**Shutdown logic:**
- 3-round limit: server shuts down after 3 rounds complete
- 30-minute timeout: server shuts down if no activity
- Empty room timeout: shut down if no players for 5 minutes
- Send "shutting down" notification to lobby before exit

**Health checks:**
- Server sends periodic heartbeats to lobby
- Lobby marks rooms as "stale" if no heartbeat for 2 minutes
- Lobby garbage collects stale rooms (deletes k8s resources)

**Cleanup:**
- On shutdown, server POSTs to `/rooms/{code}/shutdown`
- Lobby deletes corresponding Job and Service
- Lobby removes room from in-memory store

### 5. Production Readiness

**Not needed for development, but eventually:**
- Persistent storage (Redis/PostgreSQL) instead of in-memory room store
- Metrics and monitoring (Prometheus)
- Resource limits on game server Jobs
- Node affinity / pod anti-affinity for better distribution
- TLS for lobby service
- Rate limiting on room creation
- Room code collision handling (currently generates random codes)

## Development Workflow

**Current status:**
- ✅ Lobby service implemented and compiles
- ✅ Server registration implemented and compiles
- ✅ Helm chart created with RBAC
- ⏳ Docker build strategy needs implementation
- ⏳ End-to-end testing not done yet
- ⏳ Client integration not started
- ⏳ Server lifecycle not implemented

**Blocked on:**
- Docker build setup for minikube deployment
- Once that's working, can test end-to-end flow

**Next immediate steps:**
1. Create Dockerfile with proper layer caching
2. Build images and deploy to minikube
3. Test room creation flow
4. Fix any bugs discovered
5. Move on to client integration
