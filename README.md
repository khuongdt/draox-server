# Draox Server

> A plugin-powered, multi-protocol socket server built in Rust — with a VS Code-inspired plugin architecture, WASM sandboxing, and a built-in marketplace.

[![Rust](https://img.shields.io/badge/rust-2024%20edition-orange.svg)](https://www.rust-lang.org)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Tokio](https://img.shields.io/badge/async-tokio-green.svg)](https://tokio.rs)
[![WASM](https://img.shields.io/badge/plugins-WASM%20sandboxed-purple.svg)](https://wasmtime.dev)

---

## Overview

Draox Server is a high-performance, modular server built as a Cargo workspace of 14+ specialized crates. It accepts connections over TCP, UDP, WebSocket, and HTTP/HTTPS from the same process, with all traffic passing through a configurable plugin pipeline.

**Key design principles:**

- **Server-authoritative sessions** — clients may hold multiple concurrent connections (primary, notification, control, streaming)
- **Hybrid plugin model** — built-in Rust plugins compiled at build time, plus external WASM plugins loaded at runtime in a wasmtime sandbox
- **Zero-dependency networking** — `socket-server` has no dependency on any plugin crate; protocol stays agnostic
- **Marketplace-first** — plugins are packaged as `.dxp` (Draox Plugin) archives, signed with Ed25519, and distributed via the plugin registry

---

## Architecture

```
┌─────────────────────────────────────────────────────┐
│  Layer 6 — Application      draox-server (main.rs)  │
├─────────────────────────────────────────────────────┤
│  Layer 5 — API              admin-api                │
│                             REST (~72 endpoints)     │
│                             WebSocket (5 streams)    │
│                             Swagger UI               │
├─────────────────────────────────────────────────────┤
│  Layer 4 — Plugins          plugin-clans             │
│                             plugin-messaging         │
│                             plugin-identity          │
│                             plugin-presence          │
│                             plugin-storage           │
│                             plugin-push              │
│                             plugin-jobs              │
│                             plugin-e2ee              │
│                             [external WASM plugins]  │
├─────────────────────────────────────────────────────┤
│  Layer 3 — Plugin Runtime   plugin-host              │
│                             lifecycle, WASM sandbox  │
│                             marketplace client       │
├─────────────────────────────────────────────────────┤
│  Layer 2 — Services         connection-manager       │
│                             data-store               │
│                             cache-layer              │
│                             activity-log             │
│                             billing                  │
│                             secrets-manager          │
├─────────────────────────────────────────────────────┤
│  Layer 1 — Networking       socket-server            │
│                             traffic-guard            │
├─────────────────────────────────────────────────────┤
│  Layer 0 — Foundation       server-core              │
│                             server-config            │
│                             plugin-sdk               │
│                             draox-macros             │
└─────────────────────────────────────────────────────┘
```

---

## Crates

| Crate | Purpose |
|---|---|
| `server-core` | Core types, traits, shared errors |
| `server-config` | TOML config loading, hot-reload via file watcher |
| `plugin-sdk` | Plugin developer API, macros, `PluginContext` |
| `draox-macros` | Procedural macros for the plugin system |
| `socket-server` | Raw networking: TCP, UDP, WebSocket, HTTP/HTTPS |
| `traffic-guard` | Anti-spam, DDoS protection, rate limiting, IP reputation scoring |
| `connection-manager` | Multi-connection pool, server-authoritative sessions |
| `data-store` | SQL (PostgreSQL / MySQL / SQLite via sqlx) + NoSQL (MongoDB) |
| `cache-layer` | Redis (fred) + in-memory (moka) caching |
| `activity-log` | Connection logging, data metrics |
| `billing` | Usage billing, subscription plans |
| `secrets-manager` | Secret storage, vault integration |
| `plugin-host` | Plugin lifecycle, WASM sandbox (wasmtime), marketplace client |
| `admin-api` | REST API + WebSocket admin dashboard + OpenAPI/Swagger UI |
| `plugin-clans` | Built-in: Clans / Groups management |
| `plugin-messaging` | Built-in: Instant messaging |
| `plugin-identity` | Built-in: Authentication, identity, OAuth2, TOTP/MFA |
| `plugin-presence` | Built-in: Online presence, user status |
| `plugin-storage` | Built-in: File storage, AWS S3 integration |
| `plugin-push` | Built-in: Push notifications |
| `plugin-jobs` | Built-in: Background jobs, cron scheduling |
| `plugin-e2ee` | Built-in: End-to-end encryption (X25519 + ChaCha20-Poly1305) |
| `otel-layer` | OpenTelemetry tracing + metrics export |
| `feature-flags` | Runtime feature flag evaluation |
| `i18n` | Internationalization / localization |
| `graphql-api` | GraphQL API gateway (async-graphql) |
| `tools/sdk-gen` | SDK code generator from OpenAPI spec |

---

## Features

### Networking
- **TCP** on port 9000 — raw socket with keepalive, nodelay, configurable buffers
- **UDP** on port 9001 — stateless and sessionized packets, broadcast, multicast
- **WebSocket** on port 9002 — frame-level, ping/pong, optional per-message compression
- **HTTP/HTTPS** on port 9003 — REST endpoints, SSE, static file serving, CORS
- **TLS** via `rustls` — certificate hot-reload, optional mTLS

### Traffic Guard
- Per-IP connection limits and new-connection rate limiting
- Token bucket and sliding window rate limiting
- Progressive banning with exponential backoff (5m → 30m → 3h → 18h → 24h)
- IP reputation scoring (0–100)
- Static IP/CIDR blacklist and whitelist
- Slowloris attack detection
- Adaptive throttling based on CPU and memory pressure

### Session Management
- Server-authoritative sessions with multiple concurrent connections per client
- Connection roles: **primary**, **notification**, **control**, **streaming**
- Session heartbeat with configurable intervals and timeouts
- Configurable grace period for reconnects

### Plugin System
- **Built-in plugins** — Rust crates compiled into the binary at build time
- **External plugins** — WASM packages (`.dxp`) sandboxed by wasmtime with memory and CPU limits
- **Lifecycle** — `activate → enable ↔ disable → deactivate` (VS Code-inspired)
- **Plugin manifest** — `plugin.toml` with reverse-domain ID, version, dependencies
- **Plugin signing** — Ed25519 signatures verified before installation
- **Hot-reload** — file watcher triggers reload on plugin update

### Admin API (port 9100)
- ~72 REST endpoints for connections, sessions, plugins, config, billing, and metrics
- 5 WebSocket streams for real-time dashboard data
- JWT and API key authentication
- OpenAPI 3.0 specification with Swagger UI at `/swagger-ui`

### Storage
- SQL backends: **PostgreSQL**, **MySQL / MariaDB**, **SQLite**
- NoSQL: **MongoDB**
- Query layer: `sqlx` with compile-time checked queries
- Auto-migration on startup (configurable)

### Cache
- **Redis** via `fred` — pub/sub, scripting, subscriber client
- **In-memory** via `moka` — async, TTL, size-bound

### Observability
- Structured logging via `tracing` + `tracing-subscriber` (pretty or JSON format)
- Prometheus metrics endpoint on port 9090
- OpenTelemetry distributed tracing (`otel-layer` crate)

### Security
- JWT authentication with configurable secret (env var required)
- Ed25519 plugin signature verification
- Argon2 password hashing
- AES-GCM and ChaCha20-Poly1305 encryption support
- HMAC-SHA2 integrity signing

---

## Quick Start

### Prerequisites

- Rust toolchain (2024 edition): https://rustup.rs
- SQLite (default) or PostgreSQL / MySQL for production

### Build and Run

```bash
# Clone the repository
git clone https://github.com/draox/draox-server
cd draox-server

# Build all crates
cargo build

# Run with default config (SQLite, no Redis, no TLS)
cargo run

# Run with a specific config file
cargo run -- --config config/default.toml
```

### Docker

```bash
# Start with Docker Compose (multi-stage build)
docker compose up -d
```

The Compose file provides health checks, resource limits, and volume mounts for config, data, plugins, and certs.

---

## Configuration

The server is configured via TOML files. The default config is at `config/default.toml`.

**Load order:**
```
config/default.toml  →  {DRAOX_ENV}.toml  →  DRAOX_* environment variables
```

**Key sections:**

| Section | Description |
|---|---|
| `[server]` | Global server settings, plugin directory |
| `[tcp]` | TCP port, buffers, keepalive |
| `[udp]` | UDP port, buffers, multicast |
| `[websocket]` | WebSocket port, frame limits, ping |
| `[http]` | HTTP port, body limit, CORS, SSE |
| `[tls]` | TLS cert/key paths, mTLS |
| `[traffic_guard]` | Rate limits, banning, IP reputation |
| `[sessions]` | Max connections per session, timeouts |
| `[storage]` | Database backend and connection pool |
| `[cache]` | Redis and in-memory cache config |
| `[admin_api]` | Admin port, JWT secret, Swagger |
| `[logging]` | Log level, format, rotation |
| `[metrics]` | Prometheus endpoint, port |
| `[marketplace]` | Plugin registry URL, signature verification |
| `[billing]` | Stripe integration, free tier limits |
| `[plugins.*]` | Per-plugin configuration blocks |

**Critical environment variables:**

```bash
DRAOX_ADMIN_JWT_SECRET=<strong-random-secret>   # Required — admin API auth
DRAOX_ENV=production                             # Config environment profile
```

---

## Ports

| Port | Protocol | Purpose |
|---|---|---|
| 9000 | TCP | Raw TCP connections |
| 9001 | UDP | UDP packets |
| 9002 | TCP/WS | WebSocket connections |
| 9003 | TCP/HTTP | HTTP / REST / SSE |
| 9090 | TCP/HTTP | Prometheus metrics scrape |
| 9100 | TCP/HTTP | Admin REST API + WebSocket |

---

## Deployment

### Linux (systemd)

```bash
# Automated install with systemd unit, logrotate, firewall
sudo bash deploy/linux/install.sh

# Uninstall (add --purge to delete data)
sudo bash deploy/linux/uninstall.sh
```

### Debian / Ubuntu Package

```bash
cargo install cargo-deb
cargo deb -p draox-server
sudo dpkg -i target/debian/draox-server_*.deb
```

### Windows Service

```powershell
# Register Windows Service + firewall rules
.\deploy\windows\scripts\install-service.ps1

# Uninstall
.\deploy\windows\scripts\uninstall-service.ps1
```

### Windows MSI

```powershell
# Requires WiX Toolset
cargo install cargo-wix
cargo wix
# Produces target/wix/draox-server-*.msi
```

MSI features: **Core**, **Windows Service**, **Firewall Rules**

---

## Plugin Development

Plugins are developed against the `plugin-sdk` crate.

### Plugin Manifest (`plugin.toml`)

```toml
id = "io.example.my-plugin"
name = "My Plugin"
version = "1.0.0"
description = "Does something useful"
authors = ["Your Name"]

[dependencies]
"io.draox.messaging" = ">=1.0"
```

### Plugin Package (`.dxp`)

A `.dxp` file is a ZIP archive containing:
```
plugin.toml       # Manifest
plugin.wasm       # Compiled WASM module
assets/           # Static assets (optional)
plugin.sig        # Ed25519 signature
```

### Plugin Lifecycle

```
activate()   → on server start / install
enable()     → plugin is running and handling events
disable()    → plugin paused but loaded
deactivate() → plugin unloaded, cleanup
```

---

## Design Documents

- English: [`docs/design_en.html`](docs/design_en.html)
- Vietnamese: [`docs/design_vi.html`](docs/design_vi.html)

---

## Development Roadmap

### Phase 1 — Production Foundation
- [ ] Prometheus metrics wired into `activity-log` and `socket-server`
- [ ] Dependency-aware health check in `admin-api`
- [ ] mTLS `ClientCertVerifier` in `socket-server`
- [ ] RBAC for Admin API JWT claims
- [ ] Append-only audit log
- [ ] Database migration CLI (`draoxctl db migrate`)
- [ ] OpenTelemetry distributed tracing via `otel-layer`
- [ ] Bounded async channels in `connection-manager`

### Phase 2 — Resilience & Scale
- [ ] Circuit breaker for `data-store` and `cache-layer`
- [ ] Redis-backed distributed session registry
- [ ] Distributed rate limiting (Redis sliding window)
- [ ] Per-session application-level rate limiting
- [ ] Message delivery guarantees (sequence numbers + ACK)
- [ ] Plugin dependency resolution with topological sort
- [ ] Data retention policy engine

### Phase 3 — Ecosystem Growth
- [ ] TypeScript client SDK (WebSocket + Admin REST)
- [ ] GDPR tooling (data export API, erasure)
- [ ] Plugin versioning and rollback
- [ ] Plugin resource quotas (wasmtime fuel + memory)
- [ ] Binary protocol (MessagePack negotiation)
- [ ] SSE implementation (`/events?topics=...`)
- [ ] Config environment profiles
- [ ] `draoxctl` CLI management tool

### Phase 4 — Advanced Capabilities
- [ ] gRPC gateway (`crates/grpc-gateway`, port 9005)
- [ ] Plugin hot-reload without client disconnect
- [ ] QUIC / HTTP/3 transport (`quinn`, port 9004)
- [ ] HashiCorp Vault / AWS Secrets Manager integration
- [ ] MQTT gateway for IoT use cases

---

## Key Dependencies

| Crate | Purpose |
|---|---|
| `tokio` | Async runtime (multi-threaded) |
| `axum` | HTTP + WebSocket framework |
| `sqlx` | Async SQL with compile-time checks |
| `mongodb` | MongoDB async driver |
| `fred` | Redis client with pub/sub |
| `moka` | In-memory async cache |
| `wasmtime` | WASM plugin sandbox |
| `rustls` | TLS implementation |
| `tracing` | Structured logging and spans |
| `prometheus` | Metrics exposition |
| `governor` | Rate limiting |
| `ed25519-dalek` | Plugin signature verification |
| `jsonwebtoken` | JWT auth |
| `utoipa` | OpenAPI spec generation |
| `notify` | Config and plugin hot-reload |
| `serde` + `serde_json` | Serialization |
| `thiserror` + `anyhow` | Error handling |

---

## Contributing

```bash
# Run all tests
cargo test

# Check for lint issues before committing
cargo clippy --all-targets -- -D warnings

# Format code
cargo fmt
```

Commit message format: `[type] title`  
Types: `feat`, `fix`, `docs`, `style`, `refactor`, `test`, `chore`

---

## License

MIT — see [LICENSE](LICENSE)


--- Notes

AI coding 