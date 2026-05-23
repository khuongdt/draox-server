# ================================================
# Draox Server — Multi-stage Docker Build
# ================================================
# Build:  docker build -t draox-server .
# Run:    docker run -d --name draox -p 9000-9003:9000-9003 -p 9100:9100 draox-server
# ================================================

# ── Stage 1: Build ──
FROM rust:1.88-slim-bookworm AS builder

RUN apt-get update && \
    apt-get install -y --no-install-recommends \
        pkg-config \
        libssl-dev \
        && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/draox-server

# Cache dependencies — copy manifests first
COPY Cargo.toml Cargo.lock ./
COPY crates/server-core/Cargo.toml         crates/server-core/Cargo.toml
COPY crates/server-config/Cargo.toml       crates/server-config/Cargo.toml
COPY crates/plugin-sdk/Cargo.toml          crates/plugin-sdk/Cargo.toml
COPY crates/socket-server/Cargo.toml       crates/socket-server/Cargo.toml
COPY crates/traffic-guard/Cargo.toml       crates/traffic-guard/Cargo.toml
COPY crates/connection-manager/Cargo.toml  crates/connection-manager/Cargo.toml
COPY crates/data-store/Cargo.toml          crates/data-store/Cargo.toml
COPY crates/cache-layer/Cargo.toml         crates/cache-layer/Cargo.toml
COPY crates/activity-log/Cargo.toml        crates/activity-log/Cargo.toml
COPY crates/billing/Cargo.toml             crates/billing/Cargo.toml
COPY crates/plugin-host/Cargo.toml         crates/plugin-host/Cargo.toml
COPY crates/admin-api/Cargo.toml           crates/admin-api/Cargo.toml
COPY crates/plugin-clans/Cargo.toml        crates/plugin-clans/Cargo.toml
COPY crates/plugin-messaging/Cargo.toml    crates/plugin-messaging/Cargo.toml
COPY crates/draox-server/Cargo.toml        crates/draox-server/Cargo.toml
COPY crates/draox-macros/Cargo.toml        crates/draox-macros/Cargo.toml
# Phase A
COPY crates/plugin-identity/Cargo.toml     crates/plugin-identity/Cargo.toml
COPY crates/plugin-cluster/Cargo.toml      crates/plugin-cluster/Cargo.toml
COPY crates/plugin-presence/Cargo.toml     crates/plugin-presence/Cargo.toml
# Phase B
COPY crates/plugin-storage/Cargo.toml      crates/plugin-storage/Cargo.toml
COPY crates/plugin-push/Cargo.toml         crates/plugin-push/Cargo.toml
COPY crates/plugin-jobs/Cargo.toml         crates/plugin-jobs/Cargo.toml
COPY crates/secrets-manager/Cargo.toml     crates/secrets-manager/Cargo.toml
# Phase C
COPY crates/plugin-e2ee/Cargo.toml         crates/plugin-e2ee/Cargo.toml
COPY crates/otel-layer/Cargo.toml          crates/otel-layer/Cargo.toml
# Phase D
COPY crates/feature-flags/Cargo.toml       crates/feature-flags/Cargo.toml
COPY crates/i18n/Cargo.toml                crates/i18n/Cargo.toml
COPY crates/graphql-api/Cargo.toml         crates/graphql-api/Cargo.toml
# Tools
COPY tools/sdk-gen/Cargo.toml              tools/sdk-gen/Cargo.toml

# Create dummy source files to cache dependency compilation
RUN for dir in crates/*/; do \
        mkdir -p "$dir/src" && \
        echo "" > "$dir/src/lib.rs"; \
    done && \
    mkdir -p crates/draox-server/src && \
    echo "fn main() {}" > crates/draox-server/src/main.rs && \
    mkdir -p crates/draox-macros/src && \
    echo "" > crates/draox-macros/src/lib.rs && \
    mkdir -p tools/sdk-gen/src && \
    echo "fn main() {}" > tools/sdk-gen/src/main.rs

RUN cargo build --release --bin=draox-server 2>/dev/null || true

# Copy actual source code
COPY crates/ crates/

# Touch source files so cargo rebuilds them (not the cached deps)
RUN find crates/ -name "*.rs" -exec touch {} +

# Build the real binary
RUN cargo build --release --bin=draox-server && \
    strip target/release/draox-server

# ── Stage 2: Runtime ──
FROM debian:bookworm-slim AS runtime

RUN apt-get update && \
    apt-get install -y --no-install-recommends \
        ca-certificates \
        libssl3 \
        curl && \
    rm -rf /var/lib/apt/lists/* && \
    groupadd --system draox && \
    useradd --system --gid draox --no-create-home --shell /usr/sbin/nologin draox

# Create directory structure
RUN mkdir -p /opt/draox-server/bin \
             /opt/draox-server/plugins \
             /etc/draox-server \
             /var/lib/draox-server \
             /var/log/draox-server && \
    chown -R draox:draox /var/lib/draox-server /var/log/draox-server /opt/draox-server/plugins

# Copy binary
COPY --from=builder /usr/src/draox-server/target/release/draox-server /opt/draox-server/bin/

# Copy config
COPY config/default.toml /etc/draox-server/config.toml

# Adjust config paths for container
RUN sed -i \
    -e 's|"plugins"|"/opt/draox-server/plugins"|' \
    -e 's|sqlite://data/draox.db|sqlite:///var/lib/draox-server/draox.db|' \
    -e 's|"certs/|"/etc/draox-server/certs/|' \
    /etc/draox-server/config.toml

USER draox
WORKDIR /opt/draox-server

# Ports: TCP, UDP, WebSocket, HTTP, Metrics, Admin API
EXPOSE 9000/tcp 9001/udp 9002/tcp 9003/tcp 9090/tcp 9100/tcp

# Volumes for persistent data
VOLUME ["/var/lib/draox-server", "/var/log/draox-server", "/etc/draox-server", "/opt/draox-server/plugins"]

ENV RUST_LOG=info,draox_server=info \
    RUST_BACKTRACE=0 \
    DRAOX_ADMIN_JWT_SECRET=b030f793aed84c215c0399e6284d63a248fa8271deaecabcda683672b9ad9fc9

ENTRYPOINT ["/opt/draox-server/bin/draox-server"]
CMD ["--config", "/etc/draox-server/config.toml"]
