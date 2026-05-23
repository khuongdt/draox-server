# Draox Server — Missing Production Features Analysis

**Date:** 2026-04-24  
**Version Reference:** Draox Server v0.1.0 (Cargo workspace, 16 crates)  
**Methodology:** Static analysis of all `Cargo.toml` dependency manifests, `config/default.toml`, and existing source structure.

---

## What Is Already Present (Confirmed via Cargo.toml)

| Capability | Implementation | Crate |
|---|---|---|
| TLS (server-side) | `tokio-rustls` + `rustls` | `socket-server` |
| Rate limiting (IP-level) | `governor` | `traffic-guard` |
| JWT authentication | `jsonwebtoken` | `admin-api` |
| Plugin signing | `ed25519-dalek` | workspace-level |
| Metrics endpoint | `prometheus` (v0.13) | workspace-level |
| Structured logging | `tracing` + `tracing-subscriber` | all crates |
| API docs | `utoipa` + `utoipa-swagger-ui` | `admin-api` |
| Compression | `flate2`, `tower-http` (gzip + br) | `socket-server` |
| File watcher / hot-reload | `notify` | `server-config`, `plugin-host` |
| WASM sandbox | `wasmtime` | `plugin-host` |
| In-memory cache | `moka` | `cache-layer` |
| Redis | `fred` (v10) | `cache-layer` |
| Graceful shutdown | `shutdown_timeout_secs = 30` in config | `draox-server` |

---

## Key Findings: Three Hidden Structural Gaps

**Gap A — Prometheus is a ghost dependency.** Declared in `[workspace.dependencies]` but imported by zero crates. The metrics port (9090) configured in `default.toml` serves nothing. Easiest Critical fix (1-2 days).

**Gap B — mTLS is config-fiction.** `config/default.toml` has `mtls = false` and a commented `ca_path`, but `socket-server` has no `ClientCertVerifier` implementation. Zero code exists for it beyond the config field.

**Gap C — MessagePack/Bincode are declared but unused.** `bincode` and `rmp-serde` appear in workspace deps and `cache-layer/Cargo.toml`, but protocol remains JSON-only despite binary deps being present.

---

## Category 1 — Observability & Monitoring

### 1.1 OpenTelemetry Distributed Tracing [MISSING — CRITICAL]

**Gap:** No OpenTelemetry integration anywhere in the dependency graph. `tracing` alone gives local spans — not cross-service correlation.

**Why Essential:** In a multi-crate, async system with plugin middleware, a single client request traverses `socket-server` → `traffic-guard` → `connection-manager` → plugin. Without distributed traces with propagated trace IDs, diagnosing latency issues or errors requires guessing.

**Complexity:** Medium | **Priority:** Critical

**Implementation:**
```toml
opentelemetry = { version = "0.27", features = ["trace"] }
opentelemetry-otlp = { version = "0.27", features = ["grpc-tonic"] }
opentelemetry_sdk = "0.27"
tracing-opentelemetry = "0.28"
```
- Add `TraceLayer` middleware in `socket-server`
- Propagate W3C `traceparent` headers through HTTP and custom headers through TCP/WS
- Export to Jaeger, Grafana Tempo, or any OTLP collector
- Add span attributes: `client_id`, `session_id`, `plugin_name`, `protocol`

---

### 1.2 Prometheus Metrics — Incomplete Integration [MISSING — CRITICAL]

**Gap:** `prometheus = "0.13"` is in `[workspace.dependencies]` but not referenced by any individual crate. The config has `[metrics] enabled = true port = 9090` but no crate exposes a `/metrics` scrape endpoint.

**Complexity:** Low | **Priority:** Critical

**Implementation:**
- Add `prometheus` to `activity-log/Cargo.toml` and `socket-server/Cargo.toml`
- Expose metrics: `draox_connections_total{protocol}`, `draox_messages_per_sec{protocol}`, `draox_plugin_errors_total{plugin}`, `draox_session_duration_seconds`, `draox_traffic_guard_blocks_total{reason}`
- Run metrics HTTP server on port 9090

---

### 1.3 Dependency-Aware Health Check [PARTIAL — HIGH]

**Gap:** `/api/v1/app/health` exists but returns a flat `200 OK`. No per-dependency health resolution (DB, Redis, plugin states).

**Complexity:** Low | **Priority:** High

**Implementation:**
```json
{
  "status": "degraded",
  "components": {
    "database": {"status": "ok", "latency_ms": 2},
    "redis": {"status": "ok"},
    "plugins": {"clans": "ok", "messaging": "ok"},
    "traffic_guard": {"status": "ok", "banned_ips": 14}
  }
}
```
- Implement `HealthCheck` trait in `server-core`
- Each service implements and registers with `admin-api`

---

### 1.4 Structured Alerting [MISSING — MEDIUM]

**Gap:** No alerting integration. No webhook, PagerDuty, or Alertmanager integration.

**Complexity:** Low | **Priority:** Medium

**Implementation:**
- Define alert rules as TOML config (threshold + channel)
- Built-in: health degradation triggers webhook POST to configured URL
- Crate: `reqwest` for outbound HTTP alerts

---

### 1.5 Log Shipping — Loki / ELK [PARTIAL — MEDIUM]

**Gap:** `tracing-subscriber` supports JSON format but no log shipping integration (Loki, Elasticsearch, CloudWatch).

**Complexity:** Medium | **Priority:** Medium

**Implementation:**
- `tracing-appender` for non-blocking async log writing
- Optional `[logging] loki_url` config using Grafana Loki push API
- Correlation: add `trace_id` field to every log line

---

## Category 2 — Security

### 2.1 mTLS (Mutual TLS) [PARTIAL — CRITICAL]

**Gap:** `config/default.toml` has `[tls] mtls = false` with `# ca_path` commented out. No `ClientCertVerifier` implementation exists in `socket-server`.

**Complexity:** Medium | **Priority:** Critical

**Implementation:**
- `rustls::ServerConfig::with_client_cert_verifier()` using `webpki` roots
- Add `ca_path` config to `server-config` structs
- Certificate rotation support via `notify` file-watch + hot-reload

---

### 2.2 RBAC — Role-Based Access Control [MISSING — HIGH]

**Gap:** `admin-api` uses JWT but no role/permission system. Any valid JWT has full admin access.

**Complexity:** Medium | **Priority:** High

**Implementation:**
- Add `roles: Vec<Role>` claim to JWT payload
- Role enum: `SuperAdmin`, `Operator`, `ReadOnly`, `PluginManager`, `BillingAdmin`
- Axum middleware extractor checking required role per route

---

### 2.3 Audit Log [MISSING — HIGH]

**Gap:** `activity-log` tracks connection events. No security audit trail for admin actions (plugin install, IP ban, config changes).

**Complexity:** Low | **Priority:** High

**Implementation:**
- New `AuditEvent` type in `server-core`: `actor`, `action`, `resource`, `before`, `after`, `timestamp`, `ip`
- Middleware in `admin-api` emitting audit events for all state-mutating endpoints
- Persist to append-only DB table (cannot be deleted via API)

---

### 2.4 Application-Level Rate Limiting (Per User) [MISSING — HIGH]

**Gap:** `traffic-guard` rate limits by IP. Once authenticated through a CDN, all requests share the CDN IP. User-level abuse is undetected.

**Complexity:** Medium | **Priority:** High

**Implementation:**
- Extend `traffic-guard`: rate limit keyed by `session_id` / `user_id`
- Use Redis (`fred`) for distributed counters (sliding window)
- Per-action limits: messages/sec, API calls/min, file uploads/hour

---

### 2.5 Secrets Management [MISSING — HIGH]

**Gap:** `jwt_secret = ""` with env var comment. No vault integration or secrets rotation mechanism.

**Complexity:** High | **Priority:** High

**Implementation:**
- Phase 1: enforce env var validation at startup
- Phase 2: HashiCorp Vault via `vaultrs` crate
- Phase 3: AWS Secrets Manager / Azure Key Vault

---

### 2.6 Input Validation & Schema Enforcement [MISSING — MEDIUM]

**Gap:** Incoming messages are opaque bytes. No structural validation before plugin dispatch.

**Complexity:** Medium | **Priority:** Medium

---

## Category 3 — Reliability

### 3.1 Circuit Breaker [MISSING — HIGH]

**Gap:** No circuit breaker. If the database becomes slow, `data-store` calls pile up with async tasks queuing indefinitely.

**Complexity:** Medium | **Priority:** High

**Implementation:**
- Crate: `failsafe` or custom via `tokio::time::timeout` + exponential backoff
- Apply to: `data-store` queries, `cache-layer` Redis calls, WASM invocations, marketplace HTTP calls
- Expose circuit state via health endpoint and metrics

---

### 3.2 Backpressure — Bounded Async Channels [MISSING — HIGH]

**Gap:** Message dispatch to plugins likely uses unbounded channels. Under load, this leads to unbounded memory growth and OOM.

**Complexity:** Medium | **Priority:** High

**Implementation:**
- Use bounded `tokio::sync::mpsc::channel(capacity)` for all plugin message dispatch
- Expose `[server] inbound_queue_capacity` in config (default: 10,000 per session)
- Track queue depth as Prometheus gauge

---

### 3.3 Message Delivery Guarantees [MISSING — HIGH]

**Gap:** WebSocket and TCP message delivery is fire-and-forget. Messages to disconnected clients are lost.

**Complexity:** High | **Priority:** High

**Implementation:**
- Message sequence numbers per session channel
- Client sends ACK per received message (or cumulative ACK)
- Server buffers unacknowledged messages up to configured window
- Persist queue to `data-store` for offline clients

---

### 3.4 Graceful Plugin Crash Recovery [MISSING — MEDIUM]

**Gap:** No restart-without-server-restart for crashed WASM plugins.

**Complexity:** High | **Priority:** Medium

**Implementation:**
- WASM instance isolation: fresh store per plugin invocation
- Supervisor task per plugin with restart delay and max retries
- Plugin health state: `Starting`, `Running`, `Crashed`, `Restarting`, `Disabled`
- Circuit breaker on plugin restarts

---

## Category 4 — Developer Experience

### 4.1 Client SDKs [MISSING — HIGH]

**Gap:** No socket protocol client library. Developers must reverse-engineer the wire protocol.

**Complexity:** High | **Priority:** High

**Implementation:**
- Define canonical wire protocol in `plugin-sdk`
- TypeScript SDK (Node.js + browser), Python, C# (Unity), GDScript (Godot)
- SDK features: connection pooling, auto-reconnect with exponential backoff, event emitter API

---

### 4.2 CLI Management Tool (`draoxctl`) [MISSING — MEDIUM]

**Gap:** No CLI tool for operations. Admin is entirely through REST API.

**Complexity:** Medium | **Priority:** Medium

**Implementation:**
- New binary crate `draoxctl` using `clap`
- Subcommands: `connections list`, `plugin install`, `ban add`, `config validate`, `db migrate`

---

### 4.3 Testing Infrastructure [MISSING — MEDIUM]

**Gap:** No integration test harness. Claims to support 10,000 connections but never verified by load tests.

**Complexity:** Medium | **Priority:** Medium

**Implementation:**
- Integration test binary in `crates/draox-server/tests/` using `tokio::test`
- `TestServer::start()` spins up all crates on random ports
- `criterion` for micro-benchmarks on hot paths

---

### 4.4 Config Schema Validation [MISSING — MEDIUM]

**Gap:** No semantic validation. `jwt_secret` can be empty; port conflicts undetected; CORS `*` in production unwarned.

**Complexity:** Low | **Priority:** Medium

**Implementation:**
- `validate()` method on all config structs using `validator` crate
- `draoxctl config validate` runs without starting the server

---

## Category 5 — Performance

### 5.1 Binary Protocol (MessagePack) [MISSING — MEDIUM]

**Gap:** `bincode` and `rmp-serde` declared in workspace but unused. Protocol is JSON-only.

**Complexity:** Medium | **Priority:** Medium

**Implementation:**
- Protocol negotiation during connection handshake
- `plugin-sdk` message envelope supports: `JSON`, `MessagePack`, `Bincode`
- Benchmark all three with `criterion`

---

### 5.2 Zero-Copy I/O with `bytes::Bytes` [MISSING — MEDIUM]

**Gap:** Message buffers are `Vec<u8>` per message, causing heap churn at scale.

**Complexity:** Medium | **Priority:** Medium

**Implementation:**
- `bytes = "1"` in workspace dependencies
- `socket-server` read buffers → `BytesMut` → `Bytes`
- Plugin `on_message` receives `Bytes` to avoid copy per plugin hop

---

### 5.3 HTTP/2 for HTTP Endpoints [PARTIAL — MEDIUM]

**Gap:** No explicit HTTP/2 config. HTTP server on port 9003 serves HTTP/1.1 only.

**Complexity:** Low | **Priority:** Medium

---

### 5.4 QUIC / HTTP/3 [MISSING — LOW]

**Gap:** Fully absent. Mobile clients suffer TCP head-of-line blocking during network switching.

**Complexity:** High | **Priority:** Low

**Implementation:** `quinn` crate, port 9004

---

## Category 6 — Operational

### 6.1 Database Schema Migration CLI [PARTIAL — CRITICAL]

**Gap:** `run_migrations = true` auto-runs on startup — dangerous in production (non-rollbackable, runs before health checks).

**Complexity:** Low | **Priority:** Critical

**Implementation:**
- Disable `run_migrations = true` by default in production
- `draoxctl db migrate run` / `draoxctl db migrate rollback` / `draoxctl db migrate status`

---

### 6.2 Backup & Restore Automation [MISSING — HIGH]

**Gap:** No backup scripts in `deploy/linux/` or `deploy/windows/`.

**Complexity:** Low | **Priority:** High

**Implementation:**
- `systemd.timer` for daily `pg_dump` / `sqlite3 .backup` + `mongodump`
- Optional S3/Backblaze B2 upload via `rclone`
- Retention policy: 7 daily, 4 weekly, 12 monthly
- `draoxctl backup create` / `draoxctl backup restore <snapshot>`

---

### 6.3 Config Environment Profiles [MISSING — MEDIUM]

**Gap:** Single `config/default.toml`. No dev / staging / production environment separation.

**Complexity:** Low | **Priority:** Medium

**Implementation:**
- Load chain: `default.toml` → `{DRAOX_ENV}.toml` → env var overrides
- `DRAOX_ENV` environment variable

---

## Category 7 — Missing Protocols

### 7.1 gRPC [MISSING — MEDIUM]

**Gap:** No gRPC server. Service-to-service communication must use REST.

**Complexity:** High | **Priority:** Medium

**Implementation:** `tonic` crate, new `crates/grpc-gateway/`, port 9005

---

### 7.2 Server-Sent Events (SSE) [PARTIAL — MEDIUM]

**Gap:** `config/default.toml` has `sse_enabled = true` under `[http]` but no `EventStream` implementation exists.

**Complexity:** Low | **Priority:** Medium

**Implementation:**
- `axum::response::Sse` (already in axum 0.8)
- SSE endpoint: `/events?topics=clan.updated,message.new`
- Heartbeat via SSE comment lines every 30s

---

### 7.3 MQTT [MISSING — LOW]

**Gap:** Fully absent. Needed for IoT use cases.

**Complexity:** High | **Priority:** Low

---

## Category 8 — Plugin Ecosystem

### 8.1 Plugin Dependency Resolution [MISSING — HIGH]

**Gap:** No dependency resolution engine. Plugin load order is undefined. Circular dependencies cause cryptic runtime failures.

**Complexity:** High | **Priority:** High

**Implementation:**
- `[dependencies]` section in `plugin.toml` with version constraints
- Topological sort (Kahn's algorithm) for load order
- `semver` crate for compatibility checking

---

### 8.2 Plugin Versioning & Rollback [MISSING — MEDIUM]

**Gap:** Installing a new plugin version overwrites the old one. No rollback path.

**Complexity:** Medium | **Priority:** Medium

**Implementation:**
- Store versions in `plugins/{plugin-id}/{version}/`
- `draoxctl plugin rollback <plugin-id>`
- Keep last N versions (configurable)

---

### 8.3 Plugin Resource Quotas [MISSING — MEDIUM]

**Gap:** `wasmtime` supports memory/CPU fuel limits but no config surface exists in `default.toml`.

**Complexity:** Medium | **Priority:** Medium

**Implementation:**
```toml
[resources]
max_memory_mb = 64
max_cpu_micros_per_call = 10000
max_execution_time_ms = 100
```
- `wasmtime::Store` fuel limits + `ResourceLimiter` trait

---

### 8.4 Plugin Hot-Reload Without Disconnect [MISSING — MEDIUM]

**Gap:** File-watch triggers plugin reload but no drain-and-reload mechanism. In-flight requests during reload will be corrupted.

**Complexity:** High | **Priority:** Medium

**Implementation:**
- Reload phases: `RequestDrain` → `WaitDrain` → `Unload` → `Load New` → `Resume`
- `Arc<RwLock<PluginInstance>>` with version tag

---

## Category 9 — Cluster / Distributed

### 9.1 Multi-Node Clustering [MISSING — CRITICAL]

**Gap:** `connection-manager` uses in-process `dashmap`. Sessions exist only in memory of the accepting node. Client on Node 1 cannot receive messages from client on Node 2.

**Complexity:** High | **Priority:** Critical

**Implementation:**
- Session Registry: publish active sessions to Redis Hash
- Inter-node Messaging: Redis Pub/Sub per session or NATS
- Leader Election: `redis-mutex` pattern or `etcd-rs`
- Node Discovery: heartbeat keys in Redis with TTL

---

### 9.2 Distributed Rate Limiting [MISSING — HIGH]

**Gap:** `traffic-guard` uses in-process `governor`. Each node has independent counters — clients bypass limits by connecting to different nodes.

**Complexity:** Medium | **Priority:** High

**Implementation:**
- Redis-backed sliding window via `fred` (already in `cache-layer`)
- Redis Lua script for atomic counter increment + expiry
- Local in-memory L1 cache, Redis as L2 source of truth

---

### 9.3 Service Discovery [MISSING — HIGH]

**Gap:** No service discovery. Admin API address is hardcoded in config.

**Complexity:** Medium | **Priority:** High

**Implementation:**
- Kubernetes: K8s DNS + `StatefulSet`
- Self-registration in Redis on startup, deregistration on shutdown
- Optional Consul via `consulrs` crate

---

## Category 10 — Data Management

### 10.1 Data Retention Policies [MISSING — HIGH]

**Gap:** No automatic data cleanup. All records accumulate indefinitely. GDPR requires time-bound retention.

**Complexity:** Low | **Priority:** High

**Implementation:**
```toml
[data_retention]
message_history_days = 90
connection_logs_days = 30
activity_metrics_days = 365
audit_logs_days = 2555  # 7 years for compliance
```
- Daily scheduled cleanup job
- PostgreSQL table partitioning for efficient bulk delete

---

### 10.2 GDPR Compliance Tooling [MISSING — HIGH]

**Gap:** No data subject access, deletion, or export capabilities. Required for EU users (Articles 17 & 20 GDPR).

**Complexity:** Medium | **Priority:** High

**Implementation:**
- `DELETE /api/v1/users/{id}/personal-data` — cascading delete
- `GET /api/v1/users/{id}/data-export` — ZIP of all user data as JSON
- PII anonymization (tombstone pattern for messages)
- Consent records table

---

### 10.3 Data Archival Strategy [MISSING — MEDIUM]

**Gap:** No cold storage archival. Old data either stays in hot DB or gets deleted.

**Complexity:** Medium | **Priority:** Medium

---

## Priority Summary Matrix

| # | Feature | Category | Complexity | Priority |
|---|---|---|---|---|
| 1 | OpenTelemetry Distributed Tracing | Observability | Medium | **Critical** |
| 2 | Prometheus Metrics (actual wiring) | Observability | Low | **Critical** |
| 3 | Multi-node Clustering | Distributed | High | **Critical** |
| 4 | Database Migration CLI | Operational | Low | **Critical** |
| 5 | mTLS Implementation | Security | Medium | **Critical** |
| 6 | RBAC for Admin API | Security | Medium | High |
| 7 | Audit Log | Security | Low | High |
| 8 | Circuit Breaker | Reliability | Medium | High |
| 9 | Backpressure (bounded channels) | Reliability | Medium | High |
| 10 | Application-Level Rate Limiting | Security | Medium | High |
| 11 | Message Delivery Guarantees | Reliability | High | High |
| 12 | Client SDKs | DevEx | High | High |
| 13 | Plugin Dependency Resolution | Plugin Ecosystem | High | High |
| 14 | Distributed Rate Limiting | Distributed | Medium | High |
| 15 | Service Discovery | Distributed | Medium | High |
| 16 | Secrets Management | Security | High | High |
| 17 | Data Retention Policies | Data Mgmt | Low | High |
| 18 | GDPR Compliance Tooling | Data Mgmt | Medium | High |
| 19 | Backup & Restore Automation | Operational | Low | High |
| 20 | Health Check (dependency-aware) | Observability | Low | High |
| 21 | SSE Implementation | Protocols | Low | Medium |
| 22 | Graceful Plugin Crash Recovery | Reliability | High | Medium |
| 23 | Config Environment Profiles | Operational | Low | Medium |
| 24 | Input Validation & Schema | Security | Medium | Medium |
| 25 | Plugin Versioning & Rollback | Plugin Ecosystem | Medium | Medium |
| 26 | Plugin Resource Quotas | Plugin Ecosystem | Medium | Medium |
| 27 | Plugin Hot-Reload Without Disconnect | Plugin Ecosystem | High | Medium |
| 28 | Binary Protocol (MessagePack) | Performance | Medium | Medium |
| 29 | Zero-Copy Buffers (bytes::Bytes) | Performance | Medium | Medium |
| 30 | CLI Management Tool (draoxctl) | DevEx | Medium | Medium |
| 31 | Config Schema Validation | Operational | Low | Medium |
| 32 | Testing Infrastructure | DevEx | Medium | Medium |
| 33 | Log Shipping (Loki/ELK) | Observability | Medium | Medium |
| 34 | Data Archival Strategy | Data Mgmt | Medium | Medium |
| 35 | Structured Alerting | Observability | Low | Medium |
| 36 | HTTP/2 for HTTP Endpoints | Performance | Low | Medium |
| 37 | gRPC Server | Protocols | High | Medium |
| 38 | Plugin Hot-Reload Without Disconnect | Plugin Ecosystem | High | Medium |
| 39 | QUIC / HTTP/3 | Protocols | High | Low |
| 40 | MQTT Gateway | Protocols | High | Low |

---

## Recommended Roadmap

### Phase 1 — Production Foundation (Weeks 1–8)
> Without these, no production deployment is safe.

1. Wire Prometheus metrics into `activity-log` and `socket-server`
2. Implement dependency-aware health check in `admin-api`
3. Implement mTLS `ClientCertVerifier` in `socket-server`
4. Add RBAC claims to JWT + route middleware in `admin-api`
5. Implement `AuditEvent` + append-only storage
6. Database migration CLI via `draoxctl` subcommand
7. OpenTelemetry integration with `tracing-opentelemetry` + OTLP export
8. Bounded channels throughout `connection-manager` message dispatch

### Phase 2 — Resilience & Scale (Weeks 9–16)
> Needed before traffic growth.

9. Circuit breaker for `data-store` and `cache-layer` calls
10. Redis-backed distributed session registry in `connection-manager`
11. Distributed rate limiting (Redis sliding window in `traffic-guard`)
12. Application-level rate limiting per `session_id`
13. Message delivery guarantees (sequence numbers + ACK protocol)
14. Plugin dependency resolution + topological sort in `plugin-host`
15. Data retention policy engine + scheduled cleanup

### Phase 3 — Ecosystem Growth (Weeks 17–24)
> Developer adoption and compliance.

16. TypeScript client SDK (WebSocket + REST admin)
17. GDPR tooling (data export, erasure API)
18. Plugin versioning + rollback
19. Plugin resource quotas (wasmtime fuel + memory limits)
20. Binary protocol support (MessagePack negotiation)
21. SSE implementation
22. Config environment profiles (dev/staging/production)
23. `draoxctl` CLI tool

### Phase 4 — Advanced Capabilities (Weeks 25+)
> Competitive differentiation.

24. gRPC gateway crate
25. Plugin hot-reload without disconnect
26. QUIC/HTTP/3 transport
27. OpenTelemetry traces → Grafana dashboards package
28. Secrets management (HashiCorp Vault integration)
29. Data archival strategy
30. MQTT gateway
