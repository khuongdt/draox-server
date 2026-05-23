# Draox Server — Code Map

> Last updated: 2026-04-16
> Project: d:\Workspaces\Rust\Rust-MCP

## Overview

Draox Server is a Rust-based plugin-powered multi-protocol socket server. Built as a modular Cargo workspace with 16 crates (14 functional + 2 support). Manages TCP, UDP, WebSocket, HTTP/HTTPS client connections with VS Code-inspired plugin architecture.

- **Runtime**: Tokio async (multi-threaded)
- **Tests**: 598 (+28 ignored), 0 warnings
- **Ports**: TCP=9000, UDP=9001, WS=9002, HTTP=9003, Metrics=9090, Admin=9100

---

## Layer Model

```
Layer 6: Application       crates/draox-server/src/main.rs
Layer 5: API               crates/admin-api/ (40+ REST endpoints + 5 WS streams)
Layer 4: Plugins           crates/plugin-clans/, crates/plugin-messaging/
Layer 3: Plugin Runtime    crates/plugin-host/ (lifecycle, WASM sandbox, marketplace)
Layer 2: Services          crates/connection-manager/, crates/data-store/, crates/cache-layer/, crates/activity-log/, crates/billing/
Layer 1: Networking        crates/socket-server/, crates/traffic-guard/
Layer 0: Foundation        crates/server-core/, crates/server-config/, crates/plugin-sdk/, crates/draox-macros/
```

---

## Crate Map

### Layer 0: Foundation

#### server-core (`crates/server-core/`)
Core types, traits, and errors shared across all crates.
- `src/error.rs` — `Error` enum (thiserror), `Result<T>` type alias
- `src/types.rs` — `SessionId`, `ClientId`, `ConnectionId`, `PluginId`, `Protocol` enum, `ConnectionRole`, `ConnectionState`
- `src/event.rs` — `EventBus` (tokio broadcast), `ServerEvent` enum (18 variants: ConnectionAccepted/Closed/Error, SessionCreated/Destroyed, GuardConnectionBlocked/IpBanned/IpUnbanned/AttackDetected/ThresholdAdjusted, PluginActivated/Deactivated/Enabled/Disabled/Error, ServerStarted/ShuttingDown, Custom)
- `src/shutdown.rs` — `ShutdownSignal` (broadcast-based graceful shutdown)
- `src/server_info.rs` — `ServerInfo` struct
- **Tests**: 39

#### server-config (`crates/server-config/`)
Configuration loading, hot-reload, TOML parsing.
- `src/model.rs` — All config structs: `ServerConfig`, `TcpConfig`, `UdpConfig`, `WebSocketConfig`, `HttpConfig`, `TlsConfig`, `TrafficGuardConfig`, `SessionConfig`, `StorageConfig`, `SqlConfig`, `MongoConfig`, `CacheConfig`, `RedisConfig`, `MemoryCacheConfig`, `BillingConfig`, `AdminApiConfig`, `MarketplaceConfig`
- `src/loader.rs` — `ConfigLoader` (file loading, env var overrides `DRAOX_*`, hot-reload via notify)
- `src/validation.rs` — Config validation
- Default config: `config/default.toml`

#### plugin-sdk (`crates/plugin-sdk/`)
Plugin developer API and types.
- `src/traits.rs` — `Plugin` trait (activate, deactivate, on_enable, on_disable, health_check), `PluginState` enum, `PluginHealth` enum
- `src/manifest.rs` — `PluginManifest` (id, name, version, author, type, activation, permissions)
- `src/context.rs` — `PluginContext` (handles to all server services)
- `src/handles.rs` — Service handle traits: `ConnectionHandle`, `StorageHandle`, `CacheHandle`, `EventBusHandle`, `PluginLoggerHandle`, `RouterHandle`, `SchedulerHandle`

#### draox-macros (`crates/draox-macros/`)
Procedural macros for plugin registration.
- `src/lib.rs` — `#[draox_plugin]` proc-macro

### Layer 1: Networking

#### socket-server (`crates/socket-server/`)
Raw multi-protocol networking. Zero dependencies on plugin crates.
- `src/tcp.rs` — `TcpServer` (accept loop, per-connection tasks, TCP_NODELAY, keepalive, idle timeout)
- `src/udp.rs` — `UdpServer` (recv_from loop, virtual sessions, multicast, broadcast)
- `src/websocket.rs` — `WebSocketServer` (tokio-tungstenite, ping/pong, compression)
- `src/http.rs` — `HttpServer` (axum-based, SSE, CORS, static files)
- `src/tls.rs` — Shared TLS (rustls, mTLS support)
- `src/listener.rs` — `MultiProtocolListener` (starts all enabled protocols)
- `src/tracker.rs` — `ConnectionTracker` (DashMap-based, per-IP limits, write channels)
- `src/handler.rs` — `ConnectionHandler` trait (on_connect, on_data, on_text, on_disconnect, on_error)
- `src/bandwidth.rs` — `BandwidthThrottle` (token bucket per connection)
- `src/net_metrics.rs` — `NetworkMetrics` (active connections, bytes in/out)
- **Tests**: 59

#### traffic-guard (`crates/traffic-guard/`)
Anti-spam, DDoS protection, rate limiting, IP reputation.
- `src/lib.rs` — `TrafficGuard` (main facade, implements `ConnectionHandler`)
- `src/rate_limiter.rs` — Token bucket rate limiting (per-IP, per-protocol)
- `src/ban_manager.rs` — `BanManager` (escalating bans, time-based expiry)
- `src/ip_reputation.rs` — `IpReputationTracker` (score 0-100, violation penalties, recovery)
- `src/ip_filter.rs` — Whitelist/blacklist with CIDR support
- `src/slowloris.rs` — Slowloris attack detection (min data rate, header/body timeouts)
- `src/adaptive.rs` — Adaptive throttling (CPU/memory threshold-based)
- `src/connection_limiter.rs` — Connection limits (per-IP, half-open)
- **Tests**: 75

### Layer 2: Services

#### connection-manager (`crates/connection-manager/`)
Multi-connection session management.
- `src/session_manager.rs` — `SessionManager` (create/destroy sessions, connection binding)
- `src/session_handler.rs` — `SessionHandler` (implements ConnectionHandler, routes to sessions)
- `src/session.rs` — `Session` struct (server-authoritative state, multi-connection per client)
- **Tests**: 51

#### data-store (`crates/data-store/`)
SQL + NoSQL key-value storage (namespace-scoped).
- `src/backend.rs` — `StorageBackend` trait (get/set/delete/list_keys, BoxFuture-based)
- `src/sqlite.rs` — `SqliteStorage` (sqlx, TEXT column, JSON string storage)
- `src/postgres.rs` — `PostgresStorage` (sqlx, same pattern)
- `src/mysql.rs` — `MySqlStorage` (sqlx, same pattern)
- `src/mongodb.rs` — `MongoStorage` (mongodb crate v3, native BSON storage, compound unique index)
- `src/error.rs` — Error converters (into_sqlx_error, into_mongo_error)
- `src/lib.rs` — `create_storage_backend()` factory (sqlite/postgres/mysql/mongodb)
- **Tests**: 24 (+24 ignored: 8 PostgreSQL + 8 MySQL + 8 MongoDB)

#### cache-layer (`crates/cache-layer/`)
Redis + in-memory caching.
- `src/backend.rs` — `CacheBackend` trait (get/set/delete/flush/health_check)
- `src/redis.rs` — `RedisCacheBackend` (fred crate)
- `src/memory.rs` — `MemoryCacheBackend` (moka crate)
- `src/lib.rs` — `create_cache_backend()` factory
- **Tests**: 30 (+4 ignored: Redis)

#### activity-log (`crates/activity-log/`)
Event logging and metrics collection.
- `src/lib.rs` — `ActivityLog` (ring buffer, event listener), `AuditLog`
- `src/metrics.rs` — `MetricsCollector` (connections_active, bytes_total, requests_total, errors_total)
- **Tests**: 32

#### billing (`crates/billing/`)
Usage tracking and subscription plans.
- `src/plans.rs` — `Plan`, `PlanTier` (Free/Pro/Enterprise), plan definitions
- `src/usage.rs` — `UsageTracker` (per-client request/bandwidth counters)
- **Tests**: 16

### Layer 3: Plugin Runtime

#### plugin-host (`crates/plugin-host/`)
Plugin lifecycle, WASM sandbox, marketplace client.
- `src/registry.rs` — `PluginRegistry` (register/activate/deactivate/enable/disable/restart/health)
- `src/context_builder.rs` — `ContextBuilder` (builds PluginContext for each plugin)
- `src/route_registry.rs` — `RouteRegistry` (dynamic plugin routes)
- `src/marketplace.rs` — `FullMarketplaceRegistry` (search, featured, popular, publish, reviews, analytics)
- `src/marketplace_types.rs` — `MarketplacePlugin`, `PluginVersion`, `PluginReview`, `PluginAnalytics`, `SearchResult`, `PluginCategory`
- `src/wasm/` — WASM sandbox (wasmtime, not yet fully implemented)
- **Tests**: 122

### Layer 4: Plugins

#### plugin-clans (`crates/plugin-clans/`)
Built-in plugin: Clans/Groups management.
- `src/lib.rs` — `ClansPlugin` (implements Plugin trait)
- `src/clan.rs` — Clan CRUD, member management, roles
- **Tests**: 57

#### plugin-messaging (`crates/plugin-messaging/`)
Built-in plugin: Instant messaging.
- `src/lib.rs` — `MessagingPlugin` (implements Plugin trait)
- `src/channel.rs` — Channels, direct messages, broadcast
- **Tests**: 66

### Layer 5: API

#### admin-api (`crates/admin-api/`)
REST API + WebSocket dashboard.
- `src/server.rs` — `AdminServer` (axum, starts on admin port)
- `src/state.rs` — `AppState` (all shared Arc dependencies)
- `src/auth.rs` — JWT + API key + RBAC (admin/operator/viewer roles)
- `src/response.rs` — `ApiResponse<T>` envelope { success, data, message }
- `src/error.rs` — `ApiError` and HTTP status mapping
- `src/routes/mod.rs` — Canonical route table (source of truth for all paths)
- `src/routes/app.rs` — GET /api/health, /api/health/detailed, /api/info
- `src/routes/connections.rs` — GET/DELETE /api/connections, /api/connections/stats
- `src/routes/sessions.rs` — GET/DELETE/POST /api/sessions
- `src/routes/plugins.rs` — GET/POST /api/plugins (lifecycle endpoints)
- `src/routes/guard.rs` — GET/POST /api/guard (stats, ban/unban, whitelist/blacklist, reputation)
- `src/routes/config.rs` — GET/POST /api/config
- `src/routes/billing.rs` — GET/PUT /api/billing
- `src/routes/cache.rs` — GET/POST /api/cache
- `src/routes/audit.rs` — GET /api/audit
- `src/routes/metrics.rs` — GET /api/metrics (JSON, Prometheus, activity)
- `src/routes/marketplace.rs` — GET/POST /api/marketplace (search, featured, popular, publish, reviews, analytics)
- `src/routes/dynamic_routes.rs` — GET/POST/DELETE /api/routes
- `src/routes/ws_streams.rs` — 5 WebSocket streams: /ws/events, /ws/connections, /ws/plugins, /ws/guard, /ws/metrics
- **Tests**: 26

### Layer 6: Application

#### draox-server (`crates/draox-server/`)
Server binary — wires everything together.
- `src/main.rs` — Main entry point: load config → create EventBus → ConnectionTracker → SessionManager → TrafficGuard → ActivityLog/AuditLog/Metrics → UsageTracker → Cache → Storage → PluginRegistry → register built-in plugins → MultiProtocolListener → AdminServer → wait for Ctrl+C → graceful shutdown
- **Tests**: 2

---

## Key Data Flows

### Connection Pipeline
```
Client → socket-server → traffic-guard → connection-manager → session
                              ↓                    ↓
                         IP filter/rate       SessionManager
                         Ban/Reputation       Session binding
```

### Plugin Pipeline
```
PluginRegistry → ContextBuilder → PluginContext → Plugin
      ↓                                            ↓
  activate/deactivate                     StorageHandle, CacheHandle
  enable/disable                          EventBusHandle, RouterHandle
```

### Admin API Pipeline
```
HTTP Request → axum Router → Route Handler → AppState (Arc dependencies) → Response
WebSocket    → ws_streams  → EventBus subscription → JSON frames
```

---

## Configuration

Default: `config/default.toml`

Key sections:
- `[server]` — name, host, max_connections, shutdown_timeout
- `[tcp]` / `[udp]` / `[websocket]` / `[http]` — Protocol configs
- `[tls]` — TLS/mTLS settings
- `[traffic_guard]` — Rate limiting, banning, IP reputation, adaptive throttling
- `[sessions]` — Session limits, heartbeat
- `[storage]` — Backend selection (sqlite/postgres/mysql/mongodb) + SQL/MongoDB configs
- `[cache]` — Redis + memory cache configs
- `[billing]` — Plan settings
- `[admin_api]` — Host, port, auth (JWT/API keys)
- `[marketplace]` — Registry URL, signature verification

---

## Deployment

- **Linux**: `deploy/linux/install.sh` (systemd + firewall + logrotate)
- **Docker**: `docker compose up -d` (multi-stage build)
- **Debian**: `cargo deb -p draox-server`
- **Windows MSI**: `cargo wix` (WiX Toolset)
- **Windows Service**: `deploy/windows/scripts/install-service.ps1`

---

## Design Documents

- Architecture (EN): `docs/design_en.html`
- Architecture (VI): `docs/design_vi.html`
- Backend UI Design (EN): `docs/design_backend_ui_en.html`
- Backend UI Design (VI): `docs/design_backend_ui_vi.html`
- Execution Plan: `docs/plan.md`
- Chat History: `docs/chat.md`
- Change History: `docs/history.md`

---

## Backend UI (Design Phase)

Framework: **Ant Design Pro 6** (React 18 + Ant Design 5 + UmiJS 4)
Theme: Dark (navy #1a1a2e + orange #e05d10 + green #53c28b)
Pages: 22 routes consuming 40+ API endpoints + 5 WebSocket streams
Auth: JWT + RBAC (admin/operator/viewer)
i18n: en-US + vi-VN
Components: 11 reusable (RealTimeMetricsCard, EventTimeline, PluginStatusBadge, IPReputationGauge, etc.)
