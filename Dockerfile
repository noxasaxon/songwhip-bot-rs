## USED FOR FLY.IO Deployments
# https://hub.docker.com/_/rust
FROM rust:1.80.1 as builder

# Make a fake Rust app to keep a cached layer of compiled crates
RUN USER=root cargo new app
WORKDIR /usr/src/app

COPY Cargo.toml Cargo.lock ./

RUN mkdir crates && mkdir -p crates/bot_server/src && mkdir -p crates/songwhip_bot/src
COPY crates/bot_server/Cargo.toml ./crates/bot_server/Cargo.toml
COPY crates/songwhip_bot/Cargo.toml ./crates/songwhip_bot/Cargo.toml

# Needs at least a main.rs file with a main function
RUN echo "fn main(){}" > crates/bot_server/src/main.rs && echo "fn main(){}" > crates/songwhip_bot/src/main.rs
# RUN mkdir src && echo "fn main(){}" > src/main.rs

# Will build all dependent crates in release mode
RUN cargo build --release 

# Copy the rest
COPY . .
# Build (install) the actual binaries
RUN cargo install --locked --path ./crates/bot_server 

# Runtime image
FROM debian:bookworm

RUN apt-get update \
    && apt-get install -y ca-certificates tzdata \
    && rm -rf /var/lib/apt/lists/*

# Run as "app" user
RUN useradd -ms /bin/bash app

USER app
WORKDIR /app

# Get compiled binaries from builder's cargo install directory
COPY --from=builder /usr/local/cargo/bin/ /app/

EXPOSE 3000

# https://docs.docker.com/engine/reference/builder/#healthcheck
HEALTHCHECK --interval=10s --timeout=3s \
    CMD curl -f http://localhost:3000/ || exit 1
