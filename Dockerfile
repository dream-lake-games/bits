FROM rust:1.93-bookworm AS builder

WORKDIR /app

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libasound2-dev \
    libudev-dev \
    libwayland-dev \
    libxkbcommon-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy real Cargo files and build script
COPY Cargo.toml Cargo.lock build.rs ./

# Create minimal stub src that compiles with BITS_SKIP_ANIM_GEN
RUN mkdir -p src/client src/server src/lobby && \
    echo 'pub mod animations; pub use animations::*;' > src/lib.rs && \
    echo 'include!(concat!(env!("OUT_DIR"), "/animations.rs"));' > src/animations.rs && \
    echo 'fn main() {}' > src/client/main.rs && \
    echo 'fn main() {}' > src/server/main.rs && \
    echo 'fn main() {}' > src/lobby/main.rs

# Build dependencies with stub code (this is the slow, cached layer)
RUN BITS_SKIP_ANIM_GEN=1 cargo build --bin lobby --bin server

# Remove stub src but keep compiled deps
RUN rm -rf src

# Copy real source and assets
COPY src ./src
COPY assets ./assets
COPY generated_animations.rs ./

# Build with real code (this is the fast layer on src/asset changes)
RUN cargo build --bin lobby --bin server

FROM debian:bookworm-slim AS lobby

RUN apt-get update && \
    apt-get install -y ca-certificates && \
    rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/debug/lobby /usr/local/bin/lobby

EXPOSE 8080

CMD ["lobby"]

FROM debian:bookworm-slim AS server

# Server needs these libs even for headless (linked at compile time via Bevy)
RUN apt-get update && \
    apt-get install -y ca-certificates libasound2 libwayland-client0 libxkbcommon0 && \
    rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/debug/server /usr/local/bin/server

EXPOSE 9000/udp

ENTRYPOINT ["/usr/local/bin/server"]
