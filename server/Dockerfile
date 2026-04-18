# syntax=docker/dockerfile:1.7
# Multi-stage build for ach-server.
#
# Stage 1 compiles the Rust binary against the git-pinned achronyme
# workspace crates. Stage 2 ships a debian:bookworm-slim runtime that
# runs the binary as a non-root user with a writable /tmp only.
#
# Build from the repo root:
#   docker build -f server/Dockerfile -t ach-server:local .
#
# Run:
#   docker run --rm -p 127.0.0.1:3100:3100 \
#     -e ACH_CORS_ORIGIN=http://localhost:4321 \
#     ach-server:local

# ── Builder ────────────────────────────────────────────────────────────
FROM rust:1.92-slim-bookworm AS builder

RUN apt-get update \
 && apt-get install -y --no-install-recommends \
        pkg-config \
        libssl-dev \
        git \
        ca-certificates \
 && rm -rf /var/lib/apt/lists/*

WORKDIR /build
COPY server/Cargo.toml server/Cargo.lock ./server/
COPY server/src ./server/src

WORKDIR /build/server

# --locked rejects any Cargo.lock drift introduced between commit and
# build — we want bit-exact reproducibility.
RUN cargo build --release --locked \
 && strip target/release/ach-server

# ── Runtime ────────────────────────────────────────────────────────────
FROM debian:bookworm-slim AS runtime

RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates \
 && rm -rf /var/lib/apt/lists/* \
 && groupadd --system --gid 10001 achsrv \
 && useradd --system --uid 10001 --gid achsrv \
        --no-create-home --shell /usr/sbin/nologin achsrv

COPY --from=builder /build/server/target/release/ach-server \
                    /usr/local/bin/ach-server

USER achsrv
EXPOSE 3100

# Inside the container the network namespace is already isolated, so
# binding to 0.0.0.0 is what docker's port forwarder expects. The bare
# binary still defaults to 127.0.0.1 when ACH_BIND is unset.
ENV ACH_PORT=3100 \
    ACH_BIND=0.0.0.0 \
    RUST_LOG=ach_server=info

ENTRYPOINT ["/usr/local/bin/ach-server"]
