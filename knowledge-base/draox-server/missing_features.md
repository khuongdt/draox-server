# Draox Server — Missing Production Features Analysis

**Date:** 2026-04-24  
**Version Reference:** Draox Server v0.1.0  
**Methodology:** Static analysis of all `Cargo.toml` dependency manifests, `config/default.toml`, and source structure.

---

## Three Critical Structural Gaps (Key Findings)

**Gap A — Prometheus is a ghost dependency.** Declared in `[workspace.dependencies]` but imported by zero crates. Port 9090 serves nothing. Easiest Critical fix (1-2 days).

**Gap B — mTLS is config-fiction.** `config/default.toml` has `mtls = false` with commented `ca_path`, but `socket-server` has zero `ClientCertVerifier` implementation.

**Gap C — MessagePack/Bincode declared but unused.** `bincode` and `rmp-serde` in workspace deps but protocol remains JSON-only.

---

## Critical Priority Features (Must Fix Before Production)

| Feature | Category | Complexity | Notes |
|---|---|---|---|
| OpenTelemetry Distributed Tracing | Observability | Medium | Zero OTel integration exists |
| Prometheus Metrics (actual wiring) | Observability | Low | Declared but not wired to any crate |
| Multi-node Clustering | Distributed | High | `dashmap` is in-process only, no Redis session sharing |
| Database Migration CLI | Operational | Low | `run_migrations = true` auto-runs — dangerous in production |
| mTLS Implementation | Security | Medium | Config field exists, zero code backing it |

---

## High Priority Features

| Feature | Category | Complexity |
|---|---|---|
| RBAC for Admin API | Security | Medium |
| Audit Log (admin actions) | Security | Low |
| Circuit Breaker | Reliability | Medium |
| Backpressure (bounded channels) | Reliability | Medium |
| Application-Level Rate Limiting (per user) | Security | Medium |
| Message Delivery Guarantees (ACK/retry) | Reliability | High |
| Client SDKs (TypeScript, Python, C#) | DevEx | High |
| Plugin Dependency Resolution | Plugin Ecosystem | High |
| Distributed Rate Limiting (Redis-backed) | Distributed | Medium |
| Service Discovery | Distributed | Medium |
| Secrets Management (Vault integration) | Security | High |
| Data Retention Policies (GDPR) | Data Mgmt | Low |
| GDPR Compliance Tooling | Data Mgmt | Medium |
| Backup & Restore Automation | Operational | Low |
| Dependency-Aware Health Check | Observability | Low |

---

## Medium Priority Features

- SSE Implementation (config exists, no code)
- Graceful Plugin Crash Recovery
- Config Environment Profiles (dev/staging/prod)
- Input Validation & Schema Enforcement
- Plugin Versioning & Rollback
- Plugin Resource Quotas (wasmtime fuel/memory limits — config surface missing)
- Plugin Hot-Reload Without Disconnect (drain-and-reload)
- Binary Protocol / MessagePack (deps declared, unused)
- Zero-Copy I/O with `bytes::Bytes`
- CLI Management Tool (`draoxctl`)
- Config Schema Validation
- Integration Testing Infrastructure
- Log Shipping (Loki/ELK)
- gRPC Server (`crates/grpc-gateway/`)
- HTTP/2 for HTTP Endpoints

---

## Low Priority Features

- QUIC / HTTP/3 (quinn crate)
- MQTT Gateway (IoT use cases)
- Data Archival Strategy (cold storage)

---

## Recommended Roadmap

### Phase 1 — Production Foundation (Weeks 1–8)
1. Wire Prometheus metrics into `activity-log` and `socket-server`
2. Dependency-aware health check in `admin-api`
3. mTLS `ClientCertVerifier` in `socket-server`
4. RBAC claims to JWT + route middleware in `admin-api`
5. `AuditEvent` + append-only storage
6. Database migration CLI (`draoxctl db migrate`)
7. OpenTelemetry + `tracing-opentelemetry` + OTLP export
8. Bounded channels in `connection-manager` message dispatch

### Phase 2 — Resilience & Scale (Weeks 9–16)
9. Circuit breaker for `data-store` and `cache-layer`
10. Redis-backed distributed session registry
11. Distributed rate limiting (Redis sliding window)
12. Application-level rate limiting per `session_id`
13. Message delivery guarantees (sequence + ACK)
14. Plugin dependency resolution (topological sort)
15. Data retention policy engine

### Phase 3 — Ecosystem Growth (Weeks 17–24)
16. TypeScript client SDK
17. GDPR tooling (export + erasure API)
18. Plugin versioning + rollback
19. Plugin resource quotas
20. Binary protocol (MessagePack negotiation)
21. SSE implementation
22. Config environment profiles
23. `draoxctl` CLI tool

### Phase 4 — Advanced Capabilities (Weeks 25+)
24. gRPC gateway crate
25. Plugin hot-reload without disconnect
26. QUIC/HTTP/3
27. Vault integration
28. Data archival strategy
29. MQTT gateway
