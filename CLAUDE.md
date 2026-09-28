# Draox Server

## Project Overview
A Rust-based plugin-powered multi-protocol socket server that manages client connections (TCP, UDP, WebSocket, HTTP, optional gRPC) with a VS Code-inspired plugin architecture, server-authoritative multi-connection sessions, a React admin dashboard, and client SDKs for several platforms.

Repository layout:
```
rust-v2.1/
├── backend/            Cargo workspace (edition 2024) — 29 crates + tools/sdk-gen
│   ├── crates/         all Rust crates (see table below)
│   ├── config/         default.toml, docker.toml
│   ├── deploy/         linux/ (systemd, deb-scripts, install.sh), windows/ (wix, scripts)
│   ├── proto/          draox.proto (gRPC)
│   ├── certs/ scripts/ dev TLS certs + generate-certs.{sh,ps1}
│   ├── docs/           wire-protocol.md, tls-roadmap.md
│   └── tools/          sdk-gen, sdk-ts, sdk-web, sdk-ios, sdk-unity, sdk-wpf, draox-web-demo
├── frontend/           Admin dashboard (UmiJS Max 4 + Ant Design 6 + React 19 + TS)
├── docs/               design docs, plan.md, history.md, chat.md, roadmap/analysis docs, reports
└── knowledge-base/     local KB notes
```

## Implementation Status (verified 2026-09-26)
Legend: ✅ implemented and wired into the server binary · 🔌 real code but NOT wired into `draox-server` · 🟡 partial · 🧪 stub · ❌ missing

**Working in the running binary**
- ✅ TLS on TCP, WS (wss) and HTTP (https) via `TlsListener`; fail-closed if `tls.enabled` and certs missing; per-listener `tls.websocket`/`tls.http` (P0, 2026-09-27)
- ✅ Admin API auth: `admin_auth` on all built-in + plugin routes (JWT re-checked against user store, API keys `role:key`, RBAC viewer/operator/admin, deny-by-default), per-IP rate limit with `trusted_proxies` (P0)
- ✅ TCP (rustls TLS + mTLS CA loading), UDP (virtual sessions, multicast/broadcast), WebSocket (axum, ping/pong, rooms, backpressure, deflate), HTTP (axum, CORS, compression, SSE)
- ✅ Traffic guard: whitelist → blacklist → ban → IP reputation → circuit breaker → concurrent → subnet → governor rate limit, adaptive mode
- ✅ Wire-protocol auth (`AuthHandler`): JWT validation, auth timeout, anonymous gate (`auth`/`ping` only), TCP frame delimiter — plugins receive `Identity`, never the JWT
- ✅ Sessions: multi-connection per client (max one Primary/Control), roles primary/notification/control/streaming, heartbeat, resume window
- ✅ Storage: SQLite (default), PostgreSQL, MySQL (sqlx), MongoDB · Cache: moka in-memory + Redis (fred) with fallback
- ✅ Activity log, audit log, metrics collector; billing usage tracker + Free/Pro/Enterprise plans (in-memory)
- ✅ Plugin lifecycle activate → enable ↔ disable → deactivate, restart policy with backoff, WS action dispatch (`PluginWsDispatcher`)
- ✅ Built-in plugins: `plugin-clans` (roles, invites, alliances, divisions; persisted), `plugin-messaging` (channels persisted; messages/reactions/typing/receipts/presence/offline queue in-memory)
- ✅ Admin API on :9100 — 62 routes in `admin-api/src/routes/mod.rs` + 8 clans + 11 messaging routes; 5 WS streams (`/ws/events|connections|plugins|guard|metrics`); JWT login (`/api/auth/login`, `/api/auth/me`), admin user store
- ✅ Marketplace registry — **in-memory only** (`FullMarketplaceRegistry`: search, reviews, analytics)
- ✅ gRPC (`grpc-api`, tonic) — AuthService, DraoxService, MessagingService; disabled by default (`[grpc].enabled=false`)
- ✅ Frontend: 16 pages all calling real admin-api endpoints, i18n en-US/vi-VN

**Partial / stub / not wired**
- 🟡 `DRAOX_*` env overrides: only ~14 hard-coded variables in `server-config/src/loader.rs`
- 🟡 DB migrations: `CREATE TABLE IF NOT EXISTS` only, no versioned migrations
- 🟡 `ContextBuilder`: real cache/event/logger handles; Connection/Router/Scheduler are no-ops, Storage is per-plugin in-memory
- 🟡 Prometheus text at `GET /api/metrics/prometheus` on :9100 (hand-built); `metrics.port = 9090` is not bound by anything
- 🔌 Config hot-reload (`ConfigWatcher`) never started; `POST /api/config/reload` is a stub
- 🔌 QUIC (`socket-server/src/quic.rs`), `SynTracker`, `SlowlorisDetector`
- 🔌 plugin-host: `permissions`, `dependency_graph`, `dir_watcher`, `state_persistence`, `update_checker`, `version_resolver`
- 🔌 All Phase A–D crates: `plugin-identity`, `plugin-cluster`, `plugin-presence`, `plugin-storage`, `plugin-push`, `plugin-jobs`, `secrets-manager`, `plugin-e2ee`, `otel-layer`, `feature-flags`, `i18n`, `graphql-api` (none is a dependency of `draox-server`/`admin-api`)
- 🧪 Ed25519 plugin signature (`plugin-host/src/signature.rs` only checks key length)
- 🧪 Remote marketplace client (returns "remote marketplace not yet implemented")
- ❌ WASM plugin runtime (wasmtime declared in workspace deps, used by no crate), WIT API, fuel/memory limits
- ❌ `.dxp` zip packaging (manifest TOML parsing only)
- ❌ Swagger UI / utoipa (`swagger_enabled` flag unused)
- ❌ Stripe/PayPal billing, GeoIP, Windows SCM integration in the binary (service is script-only)

**Known issues** (see `docs/[Report]Draox-Server-Overview_VI_20260926.html`)
- P0 items (admin auth, `/api/config` 500, dev-login bypass, HTTPS/WSS, deb LICENSE path, CI) fixed 2026-09-27 — see report §7.1
- Open `/ws/*` admin streams are not cut when a user is banned/deleted (token checked at upgrade only)
- Invalid config (validation failure) silently falls back to built-in defaults instead of exiting

## Layer Model
```
Layer 6: Application       draox-server (binary: crates/draox-server/src/main.rs)
Layer 5: API               admin-api (REST + 5 WS streams), grpc-api, graphql-api (not wired)
Layer 4: Plugins           plugin-clans, plugin-messaging  [+ not wired: identity, cluster, presence, storage, push, jobs, e2ee]
Layer 3: Plugin Runtime    plugin-host (lifecycle, registry, WS dispatcher, in-memory marketplace)
Layer 2: Services          connection-manager, data-store, cache-layer, activity-log, billing  [+ not wired: secrets-manager, feature-flags, i18n, otel-layer]
Layer 1: Networking        socket-server (TCP, UDP, WS, HTTP/SSE; QUIC not wired), traffic-guard
Layer 0: Foundation        server-core, server-config, plugin-sdk, draox-macros
```

Connection pipeline: `MultiProtocolListener` → `TrafficGuard` → `AuthHandler` → `SessionHandler` / `PluginWsDispatcher` → plugins.

## Crates (`backend/crates/`)
| Crate | Purpose | Status |
|-------|---------|--------|
| `server-core` | Core types (IDs, ConnectionRole, Protocol), EventBus, ShutdownSignal, errors | ✅ |
| `server-config` | TOML config models/loader, env overrides, validation, file watcher | ✅ (watcher 🔌) |
| `plugin-sdk` | Plugin trait, PluginManifest, PluginContext + handle traits, Identity | ✅ |
| `draox-macros` | `#[draox_plugin]` proc-macro | ✅ |
| `socket-server` | TCP, UDP, WS, HTTP/SSE, TLS, QUIC, bandwidth, backpressure | ✅ (QUIC 🔌) |
| `traffic-guard` | Rate limit, bans, IP reputation, circuit breaker, subnet limits | ✅ |
| `connection-manager` | Sessions, AuthHandler, heartbeat, wire protocol | ✅ |
| `data-store` | SQLite/Postgres/MySQL/MongoDB `StorageBackend` | ✅ |
| `cache-layer` | moka + Redis `CacheBackend` | ✅ |
| `activity-log` | Activity log, audit log, metrics, time series | ✅ |
| `billing` | Usage tracker, plans, quota enforcement (in-memory) | ✅ |
| `plugin-host` | Registry, lifecycle, ContextBuilder, WS dispatcher, marketplace | ✅ / 🟡 |
| `admin-api` | Admin REST + WS streams, JWT auth, user store | ✅ |
| `plugin-clans` | Built-in clans/groups plugin | ✅ |
| `plugin-messaging` | Built-in instant messaging plugin | ✅ |
| `grpc-api` | tonic gRPC services (proto: `backend/proto/draox.proto`) | ✅ (off by default) |
| `draox-server` | Server binary | ✅ |
| `plugin-identity` | Argon2id, JWT refresh rotation, OAuth2, TOTP MFA, device fingerprint | 🔌 |
| `plugin-cluster` | Redis pub/sub, shared session registry, leader election, sticky routing | 🔌 |
| `plugin-presence` | Presence status, broadcaster, auto-away | 🔌 |
| `plugin-storage` | S3/R2/MinIO object storage, presigned URLs, quotas | 🔌 |
| `plugin-push` | FCM v1, APNs, device tokens, quiet hours | 🔌 |
| `plugin-jobs` | Priority job queue, workers, retry, DLQ, cron | 🔌 |
| `secrets-manager` | Vault, AWS Secrets Manager, Azure Key Vault, AES-GCM cache | 🔌 |
| `plugin-e2ee` | X25519 + ChaCha20-Poly1305 Double Ratchet | 🔌 |
| `otel-layer` | OpenTelemetry OTLP tracing/metrics | 🔌 |
| `feature-flags` | Flag rules/evaluator | 🔌 |
| `i18n` | Locale detection, templates | 🔌 |
| `graphql-api` | async-graphql schema (skeletal) | 🔌 |

Tools (`backend/tools/`): `sdk-gen` (Rust, OpenAPI → TS/Dart), `sdk-ts` (WS + Node gRPC), `sdk-web` (browser WS), `sdk-ios` (Swift + SwiftUI demo, E2EE), `sdk-unity` (C#, WS/TCP/gRPC), `sdk-wpf` (.NET 8, TCP/WS), `draox-web-demo` (React/Vite).

## Key Commands
```bash
# Backend (run inside backend/)
cargo build                                   # Build all crates
cargo test                                    # Run all tests
cargo run -p draox-server                     # Start the server (uses config/default.toml)
cargo run -p draox-server -- --config config/default.toml

# Frontend (run inside frontend/)
npm install && npm run dev                    # Dev server, proxies /api and /ws to localhost:9100
npm run build
```

## Deployment (paths relative to `backend/`)
- **Docker**: `docker compose up -d` — services `draox-server` + `draox-admin` (frontend via nginx, proxies to `draox-server:9100`)
- **Linux (systemd)**: `deploy/linux/install.sh` / `deploy/linux/uninstall.sh [--purge]`
- **Debian/Ubuntu**: `cargo deb -p draox-server` (CI job `deb` in `.github/workflows/ci.yml`)
- **Windows (MSI)**: `cargo wix` (WiX config in `deploy/windows/wix`)
- **Windows (service)**: `deploy/windows/scripts/install-service.ps1` / `uninstall-service.ps1` — registered via `New-Service`/`sc.exe`; the binary has no native SCM handler
- **Ports**: TCP=9000, UDP=9001, WS=9002, HTTP=9003, gRPC=9004 (disabled), Admin=9100 (Prometheus text at `/api/metrics/prometheus`); `metrics.port=9090` is unused

## Configuration
- Default config: `backend/config/default.toml` (Docker: `backend/config/docker.toml`)
- Sections: `[server] [tcp] [udp] [websocket] [http] [grpc] [tls] [traffic_guard.*] [sessions] [storage] [cache] [billing] [admin_api] [logging] [metrics] [marketplace] [wire_protocol]`
- Env var prefix: `DRAOX_` (limited set, see `server-config/src/loader.rs`)
- Hot-reload: implemented in `server-config/src/watcher.rs` but not started by the binary
- Linux env file: `/etc/draox-server/draox-server.env` · Windows config: `C:\ProgramData\DraoxServer\config\default.toml`

## Documentation
- Design: `docs/design_en.html`, `docs/design_vi.html`, admin UI design `docs/design_backend_ui_{en,vi}.html`
- Plan & history: `docs/plan.md`, `docs/history.md`, `docs/chat.md`
- Gap analysis / roadmap inputs: `docs/missing_features.md`, `docs/extend_features.md`, `docs/opus_suggestion_features.md`
- Protocols: `backend/docs/wire-protocol.md`, `docs/grpc_plan.md`, `docs/sdk_plan.md`
- Status report: `docs/[Report]Draox-Server-Overview_VI_20260926.html`
- Athena KB: `draox-server/architecture.md`, `draox-server/features-status.md`

## Conventions
- Error handling: `thiserror` for library errors, `anyhow` for application errors
- Async: All I/O operations must be async (tokio)
- Logging: Use `tracing` crate (not `log` or `println!`)
- Serialization: `serde` + `serde_json` for all protocol messages
- Config: `serde` + `toml` for configuration
- Naming: snake_case for files/modules, CamelCase for types
- Commit messages: `[type] title` format (feat, fix, docs, refactor, test, chore)
- When a feature crate is wired into `draox-server`, update the status markers in this file

## Important Notes
- Plugin manifest: `plugin.toml` (TOML format, reverse-domain ID); `WasmConfig` exists as manifest data only
- Planned (not yet implemented): `.dxp` package (zip), WASM plugins via wasmtime, Ed25519 signing, remote marketplace at marketplace.draox-server.io
- `traffic-guard` wraps `AuthHandler` and sits between socket-server and connection-manager
- `socket-server` has zero dependencies on plugin crates (protocol-agnostic)
- Built-in plugins receive the real `StorageBackend` via `with_storage()`; `PluginContext` storage handle is per-plugin in-memory
- Admin API runs on a separate port (default 9100) with JWT/API-key auth; if `[admin_api].jwt_secret` is empty a random per-process secret is used (tokens die on restart). Dev login `admin`/`draox` only with `DRAOX_ENV=development`
- Plugin HTTP routes (`/api/clans/*`, `/api/channels/*`) are merged into admin-api behind the same `admin_auth` middleware (inserts `Extension<Identity>`)
