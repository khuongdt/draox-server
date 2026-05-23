# Draox Server — Additional Feature Suggestions

> **Author:** Claude Opus 4.6  
> **Date:** 2026-04-25  
> **Base:** Analysis of `design_en.html` and `missing_features.md`  
> **Scope:** 38 features not covered in the existing design or missing-features list

---

## Table of Contents

1. [Real-Time Messaging Primitives](#1-real-time-messaging-primitives)
2. [Connection & Transport](#2-connection--transport)
3. [Multi-Tenancy](#3-multi-tenancy)
4. [Deployment & Operations](#4-deployment--operations)
5. [Protocol Extensibility](#5-protocol-extensibility)
6. [Developer Experience](#6-developer-experience)
7. [Enterprise & Compliance](#7-enterprise--compliance)
8. [Observability Enhancements](#8-observability-enhancements)
9. [Data & Message Patterns](#9-data--message-patterns)
10. [Priority Matrix](#priority-matrix)
11. [Roadmap Integration](#roadmap-integration)

---

## 1. Real-Time Messaging Primitives

### 1.1 Pub/Sub with Wildcard Topics

| Attribute | Value |
|-----------|-------|
| **Priority** | Critical |
| **Complexity** | Medium |

**Description:**  
Hierarchical topic-based publish/subscribe with wildcard matching. Clients subscribe to topic patterns like `game.lobby.*` or `chat.room.>` (single-level `*` and multi-level `>` wildcards). The server performs efficient topic-tree matching and delivers messages only to matching subscribers.

**Why it's needed:**  
The current design mentions basic message broadcasting but lacks a structured pub/sub system. Every production socket server (NATS, MQTT, Redis Pub/Sub) supports wildcard topics because applications need flexible, hierarchical message routing without maintaining explicit subscription lists for every possible topic.

**Implementation approach:**  
- Build a trie-based topic tree in `connection-manager` for O(log n) topic matching
- Support `*` (single segment) and `>` (multi-segment, tail only) wildcards following NATS conventions
- Integrate with plugin-sdk so plugins can publish/subscribe programmatically
- Add topic-level ACLs in `traffic-guard` to control who can publish/subscribe to which topics

---

### 1.2 Request-Reply Pattern

| Attribute | Value |
|-----------|-------|
| **Priority** | High |
| **Complexity** | Medium |

**Description:**  
Built-in request-reply semantics where a client sends a request message with an auto-generated reply inbox, and the server routes the response back. Supports timeouts and scatter-gather (one request, multiple replies).

**Why it's needed:**  
Pure fire-and-forget messaging is insufficient for RPC-style interactions. Games need request-reply for matchmaking queries, inventory checks, and skill validations. Without built-in support, every plugin reinvents correlation IDs and timeout logic.

**Implementation approach:**  
- Auto-generate ephemeral reply subjects (`_INBOX.<unique-id>`)
- Add `reply_to` field to the base message envelope in `server-core`
- Implement timeout tracking in `connection-manager` with configurable default (30s)
- Support scatter-gather mode: collect N replies or wait until timeout

---

### 1.3 Message Deduplication

| Attribute | Value |
|-----------|-------|
| **Priority** | High |
| **Complexity** | Medium |

**Description:**  
Server-side deduplication of messages using client-provided idempotency keys. The server maintains a sliding window of seen message IDs and silently drops duplicates within the window.

**Why it's needed:**  
Network retransmissions, client reconnections, and at-least-once delivery guarantees all produce duplicate messages. Without server-side dedup, plugins must each implement their own dedup logic, leading to inconsistent behavior and wasted processing.

**Implementation approach:**  
- Add optional `idempotency_key: Option<String>` to the message envelope
- Use a time-windowed Bloom filter or LRU cache in `cache-layer` (configurable window: default 5 minutes)
- Return `duplicate: true` in the acknowledgment so the client knows
- Make the dedup window configurable per-connection and per-topic

---

### 1.4 Message Ordering Guarantees

| Attribute | Value |
|-----------|-------|
| **Priority** | High |
| **Complexity** | High |

**Description:**  
Configurable message ordering guarantees: per-connection total order, per-topic causal order, or per-partition key order. The server assigns monotonically increasing sequence numbers and can buffer/reorder messages when needed.

**Why it's needed:**  
Chat messages must arrive in order. Game state updates must be applied in sequence. Financial transactions require strict ordering. Without explicit ordering guarantees, the server's async nature means messages can arrive out of order, especially under load or after reconnection.

**Implementation approach:**  
- Assign per-topic sequence numbers (atomic counter per topic partition)
- Add `sequence_number` and `partition_key` to the message envelope
- Implement a reorder buffer in `connection-manager` for out-of-order messages
- Support three ordering modes: `none`, `per_topic`, `per_partition_key`

---

### 1.5 Message TTL & Expiry

| Attribute | Value |
|-----------|-------|
| **Priority** | Medium |
| **Complexity** | Low |

**Description:**  
Messages carry a time-to-live (TTL) value. The server automatically discards expired messages before delivery, preventing stale data from reaching clients (e.g., outdated position updates in a game, expired notifications).

**Why it's needed:**  
In real-time applications, delivering a 30-second-old position update is worse than dropping it. Without TTL, queued messages during a client disconnect accumulate and flood the client on reconnection with stale data.

**Implementation approach:**  
- Add `ttl_ms: Option<u64>` and `created_at: u64` to the message envelope
- Check expiry at delivery time in `connection-manager`
- Support per-topic default TTL in configuration
- Emit metrics for expired message count per topic

---

### 1.6 Scheduled / Delayed Delivery

| Attribute | Value |
|-----------|-------|
| **Priority** | Medium |
| **Complexity** | Medium |

**Description:**  
Clients can publish messages with a `deliver_at` timestamp or `delay_ms` offset. The server holds the message in a priority queue and delivers it at the specified time.

**Why it's needed:**  
Game servers need delayed events (bomb timers, cooldown expirations, scheduled tournaments). Notification systems need scheduled reminders. Without this, every plugin runs its own timer wheel, wasting memory and causing clock drift issues.

**Implementation approach:**  
- Use a tokio-based timer wheel or `DelayQueue` in `connection-manager`
- Persist scheduled messages to `data-store` for durability across restarts
- Add `deliver_at: Option<u64>` to the message envelope
- Cap maximum delay (configurable, default 7 days) to prevent unbounded queue growth

---

### 1.7 Event Replay / Rewind

| Attribute | Value |
|-----------|-------|
| **Priority** | Medium |
| **Complexity** | High |

**Description:**  
Clients can request replay of historical messages from a specific sequence number or timestamp. The server maintains a configurable event log (ring buffer or append-only log) per topic and streams historical messages before switching to live delivery.

**Why it's needed:**  
When a client reconnects after a disconnect, it needs to catch up on missed messages. New subscribers joining a topic need recent context (last N messages or messages since timestamp T). This is fundamental for reliable messaging and is a core feature of NATS JetStream, Kafka, and EventStoreDB.

**Implementation approach:**  
- Implement a per-topic ring buffer backed by `data-store` (configurable retention: time or count)
- Support replay modes: `from_sequence(n)`, `from_time(t)`, `last_n(n)`
- Stream historical messages with a `replay: true` flag, then switch to live
- Integrate with message ordering to guarantee consistent replay

---

## 2. Connection & Transport

### 2.1 PROXY Protocol v2 Support

| Attribute | Value |
|-----------|-------|
| **Priority** | Critical |
| **Complexity** | Low |

**Description:**  
Support HAProxy PROXY Protocol v2 on TCP and WebSocket listeners. When enabled, the server reads the PROXY Protocol header to extract the real client IP/port before TLS termination, replacing the load balancer's IP in all logging, rate limiting, and ACL checks.

**Why it's needed:**  
In production, Draox will sit behind load balancers (HAProxy, AWS NLB, Cloudflare). Without PROXY Protocol support, `traffic-guard` sees the load balancer's IP for all clients, making per-client rate limiting, IP reputation, and geo-blocking completely ineffective. This is a deployment blocker.

**Implementation approach:**  
- Parse PROXY Protocol v2 binary header (16-byte signature + TLV) in `socket-server` at connection accept time
- Store real client address in the connection metadata
- Add `proxy_protocol: { enabled: bool, trusted_cidrs: Vec<String> }` to listener config
- Only accept PROXY headers from trusted source IPs to prevent spoofing

---

### 2.2 Connection Multiplexing (Streams)

| Attribute | Value |
|-----------|-------|
| **Priority** | High |
| **Complexity** | High |

**Description:**  
Allow multiple logical streams over a single TCP/WebSocket connection. Each stream has its own flow control, priority, and can be independently opened/closed without affecting other streams on the same connection.

**Why it's needed:**  
The current design supports multiple connections per client with roles (primary, notification, control, streaming), but this requires multiple TCP handshakes and TLS negotiations. Multiplexing reduces connection overhead, avoids port exhaustion, and enables head-of-line blocking avoidance for different message priorities.

**Implementation approach:**  
- Implement a lightweight multiplexing layer (inspired by HTTP/2 or QUIC streams) in `socket-server`
- Each frame includes a `stream_id: u32` header
- Support stream-level flow control with per-stream send/receive windows
- Map connection roles (primary, notification, etc.) to streams instead of separate connections

---

### 2.3 Reconnection Token with State Resume

| Attribute | Value |
|-----------|-------|
| **Priority** | Critical |
| **Complexity** | Medium |

**Description:**  
On successful connection, the server issues a short-lived reconnection token. If the client disconnects and reconnects within the token's validity window, it presents the token to skip full authentication and resume its session state (subscriptions, pending messages, plugin state).

**Why it's needed:**  
Mobile clients frequently lose connectivity. Without state resume, every reconnection requires full authentication, re-subscription to all topics, and re-fetching missed messages. This creates a poor user experience and a thundering herd problem when many clients reconnect simultaneously after a network blip.

**Implementation approach:**  
- Generate a cryptographically random 256-bit token on connection establishment
- Store session snapshot (subscriptions, last sequence numbers, plugin state) in `cache-layer` keyed by token
- Token TTL: configurable (default 5 minutes), stored in Redis for multi-node support
- On reconnection with valid token: restore session, replay missed messages, skip auth
- Invalidate token after single use to prevent replay attacks

---

### 2.4 Delta Sync / Incremental State

| Attribute | Value |
|-----------|-------|
| **Priority** | Medium |
| **Complexity** | High |

**Description:**  
Instead of sending full state snapshots, the server tracks per-client state versions and sends only the delta (changes since the client's last known version). Uses operational transformation or CRDT-based conflict resolution for concurrent modifications.

**Why it's needed:**  
Game servers frequently synchronize large state objects (world state, leaderboards, inventory). Sending full snapshots on every change wastes bandwidth, especially on mobile. Delta sync reduces bandwidth by 80-95% for typical workloads.

**Implementation approach:**  
- Add version vectors to state objects in `plugin-sdk`
- Implement a diffing engine that computes minimal changesets
- Support three modes: `full_snapshot`, `delta`, `crdt_merge`
- Expose delta sync as a plugin-sdk API so plugins can use it for their own state

---

### 2.5 Rate Limit Response Headers

| Attribute | Value |
|-----------|-------|
| **Priority** | High |
| **Complexity** | Low |

**Description:**  
Include standard rate limit headers in responses (following IETF draft `RateLimit-Limit`, `RateLimit-Remaining`, `RateLimit-Reset`) so clients can self-throttle before hitting limits.

**Why it's needed:**  
The current `traffic-guard` silently drops or rejects rate-limited requests. Without feedback, clients keep hammering the server, wasting both client and server resources. Well-behaved clients need rate limit metadata to implement backoff strategies.

**Implementation approach:**  
- Add rate limit metadata to the response envelope: `rate_limit: { limit: u32, remaining: u32, reset_at: u64 }`
- For HTTP: use standard headers `RateLimit-Limit`, `RateLimit-Remaining`, `RateLimit-Reset`
- For WebSocket/TCP: include in the message frame metadata
- Emit `429 Too Many Requests` with `Retry-After` header for HTTP endpoints

---

### 2.6 Protocol Version Negotiation

| Attribute | Value |
|-----------|-------|
| **Priority** | High |
| **Complexity** | Low |

**Description:**  
During the connection handshake, client and server negotiate the protocol version. The server advertises supported versions, and the client selects the highest mutually supported version. Enables backward-compatible protocol evolution.

**Why it's needed:**  
As Draox evolves, the wire protocol will change. Without version negotiation, upgrading the server breaks all existing clients. This is a fundamental requirement for any long-lived server that can't force-upgrade all clients simultaneously.

**Implementation approach:**  
- Add `protocol_version: u16` to the handshake message
- Server sends `supported_versions: Vec<u16>` in the HELLO response
- Client selects from the intersection; connection fails if no overlap
- Maintain compatibility adapters for N-1 and N-2 versions

---

### 2.7 Connection Quality Scoring

| Attribute | Value |
|-----------|-------|
| **Priority** | Medium |
| **Complexity** | Medium |

**Description:**  
The server continuously monitors each connection's health metrics (latency, jitter, packet loss, throughput) and computes a quality score (0-100). This score is used for intelligent routing, matchmaking, and adaptive message delivery.

**Why it's needed:**  
In gaming and real-time applications, connection quality directly impacts user experience. A matchmaking system that pairs a 20ms-latency player with a 500ms-latency player creates a poor experience. Quality scoring enables quality-aware routing and adaptive behavior.

**Implementation approach:**  
- Track RTT via periodic ping/pong in `connection-manager`
- Compute exponentially weighted moving average for latency, jitter, loss
- Score formula: `100 - (latency_penalty + jitter_penalty + loss_penalty)`
- Expose score via plugin-sdk and admin API
- Use score in `traffic-guard` for adaptive throttling decisions

---

### 2.8 WebTransport (HTTP/3 + QUIC) Support

| Attribute | Value |
|-----------|-------|
| **Priority** | Medium |
| **Complexity** | High |

**Description:**  
Add WebTransport as a transport option alongside TCP, UDP, and WebSocket. WebTransport provides reliable and unreliable streams over HTTP/3 (QUIC), with built-in multiplexing, 0-RTT connection establishment, and no head-of-line blocking.

**Why it's needed:**  
WebTransport is the modern replacement for WebSocket in browsers, offering better performance for real-time applications. It supports unreliable datagrams (like UDP but through firewalls), multiple independent streams, and 0-RTT resumption. Browser-based games and applications increasingly expect WebTransport support.

**Implementation approach:**  
- Add `quinn` crate for QUIC support in `socket-server`
- Implement WebTransport session handling following the W3C WebTransport API
- Map WebTransport streams to the existing connection multiplexing system
- Support both reliable streams and unreliable datagrams
- Add `[server.webtransport]` configuration section

---

## 3. Multi-Tenancy

### 3.1 Namespace Isolation

| Attribute | Value |
|-----------|-------|
| **Priority** | Critical |
| **Complexity** | Medium |

**Description:**  
Logical tenant isolation through namespaces. Each tenant's connections, topics, plugins, and data are isolated within their namespace. A tenant cannot see or interact with another tenant's resources unless explicitly federated.

**Why it's needed:**  
Draox as a platform needs to serve multiple customers/games from a single deployment. Without namespace isolation, all clients share the same connection pool, topic space, and plugin instances. This is a security risk, a noisy-neighbor problem, and prevents offering Draox as a multi-tenant service.

**Implementation approach:**  
- Add `namespace: String` to connection metadata and all resource identifiers
- Prefix all topics, keys, and storage paths with the namespace
- Enforce namespace boundaries in `traffic-guard` and `connection-manager`
- Support cross-namespace federation for specific use cases (admin, monitoring)
- Add namespace CRUD to `admin-api`

---

### 3.2 Per-Tenant Plugin Storage Isolation

| Attribute | Value |
|-----------|-------|
| **Priority** | High |
| **Complexity** | Medium |

**Description:**  
Each tenant gets an isolated storage partition in `data-store` and `cache-layer`. Plugins accessing storage through `plugin-sdk` are automatically scoped to the current tenant's partition, preventing data leakage.

**Why it's needed:**  
Even if topics and connections are namespace-isolated, a plugin that stores data in a shared database can accidentally (or maliciously) access another tenant's data. Storage isolation must be enforced at the SDK level, not left to plugin developers.

**Implementation approach:**  
- Wrap `data-store` and `cache-layer` handles in `plugin-sdk` with automatic namespace prefixing
- Use database schemas (PostgreSQL) or database names (MongoDB) per tenant
- Redis key prefixing: `{namespace}:{plugin_id}:{key}`
- WASM plugins get a scoped storage handle that physically cannot access other namespaces

---

### 3.3 Tenant-Level Configuration Override

| Attribute | Value |
|-----------|-------|
| **Priority** | Medium |
| **Complexity** | Low |

**Description:**  
Allow per-tenant configuration overrides for rate limits, connection limits, plugin settings, and feature flags. Tenant config inherits from the global default and overrides specific values.

**Why it's needed:**  
Different tenants have different needs: a free-tier tenant gets 100 connections and 10 msg/s, while an enterprise tenant gets 100,000 connections and unlimited throughput. Without per-tenant config, all tenants get the same limits.

**Implementation approach:**  
- Add `[tenants.<name>]` sections in `server-config`
- Implement config inheritance: tenant config merges over global defaults
- Store dynamic tenant configs in `data-store` with `cache-layer` for fast lookup
- Support hot-reload of tenant configs via admin API

---

## 4. Deployment & Operations

### 4.1 Zero-Downtime Rolling Restart

| Attribute | Value |
|-----------|-------|
| **Priority** | Critical |
| **Complexity** | High |

**Description:**  
Orchestrated server restart that migrates active connections to new server instances before shutting down old ones. Clients experience no disconnection during server upgrades.

**Why it's needed:**  
A socket server with 100,000 active connections cannot simply restart for upgrades. Each disconnection triggers client-side reconnection storms, lost messages, and broken game sessions. Zero-downtime restart is a hard requirement for production.

**Implementation approach:**  
- Implement connection draining: stop accepting new connections, wait for existing connections to naturally close or migrate
- Support connection migration: serialize session state, transfer to new instance via internal protocol
- Use orchestrator signals (SIGTERM → drain → transfer → shutdown)
- Integrate with Kubernetes rolling update strategy
- Configurable drain timeout (default 30s) after which remaining connections are force-migrated

---

### 4.2 Connection Drain API

| Attribute | Value |
|-----------|-------|
| **Priority** | High |
| **Complexity** | Low |

**Description:**  
Admin API endpoint to gracefully drain connections from a server instance. Supports selective draining (by namespace, tag, or connection quality) and reports drain progress in real-time.

**Why it's needed:**  
Operations teams need fine-grained control over connection management during maintenance windows, incident response, and capacity rebalancing. A simple "stop all connections" is too coarse; they need to drain specific segments while keeping others active.

**Implementation approach:**  
- Add `POST /admin/drain` endpoint with filters: `{ namespace, tags, quality_below, timeout_s }`
- WebSocket stream for drain progress: `{ total, drained, remaining, elapsed_s }`
- Support `POST /admin/drain/cancel` to abort an in-progress drain
- Emit drain events to `activity-log` for audit trail

---

### 4.3 Chaos Engineering Hooks

| Attribute | Value |
|-----------|-------|
| **Priority** | Medium |
| **Complexity** | Medium |

**Description:**  
Built-in fault injection capabilities: artificial latency, random disconnections, message corruption, partition simulation. Controlled via admin API and scoped to specific namespaces or connection tags.

**Why it's needed:**  
You cannot build a reliable system without testing failure modes. Chaos engineering hooks allow teams to verify that clients handle disconnections gracefully, that plugins recover from timeouts, and that the cluster rebalances correctly during partitions.

**Implementation approach:**  
- Add fault injection middleware in `socket-server` (after `traffic-guard`)
- Support: `delay(ms)`, `drop(%)`, `corrupt(%)`, `disconnect(%)`, `partition(nodes)`
- Scope faults to specific namespaces, connections, or topics
- Admin API: `POST /admin/chaos/inject`, `DELETE /admin/chaos/clear`
- Safety: require explicit `chaos_enabled: true` in config, disabled by default

---

### 4.4 Canary Plugin Deployment

| Attribute | Value |
|-----------|-------|
| **Priority** | Medium |
| **Complexity** | Medium |

**Description:**  
Deploy a new plugin version to a subset of connections (by percentage, namespace, or tag) before rolling it out to all connections. Monitor error rates and automatically rollback if the canary exceeds error thresholds.

**Why it's needed:**  
A buggy plugin update can crash all connections. Canary deployment limits the blast radius by testing new versions on a small percentage of traffic first. This is standard practice for web services but rarely available for socket server plugins.

**Implementation approach:**  
- Add `deployment_strategy: { canary_percent, rollback_threshold, monitoring_window }` to plugin manifest
- Route connections to canary or stable plugin version based on consistent hashing
- Monitor error rates, latency, and crash counts during canary window
- Auto-rollback if error rate exceeds threshold within monitoring window
- Admin API: `POST /admin/plugins/{id}/canary`, `POST /admin/plugins/{id}/promote`

---

### 4.5 Warm Standby Failover

| Attribute | Value |
|-----------|-------|
| **Priority** | Medium |
| **Complexity** | High |

**Description:**  
A standby Draox instance continuously replicates session state from the primary. On primary failure, the standby promotes itself and accepts connections with minimal disruption (target: < 5s failover).

**Why it's needed:**  
For applications that cannot tolerate the 30-60 seconds of downtime during a crash + restart cycle. Financial applications, critical communications, and competitive gaming require near-instant failover to a pre-warmed standby.

**Implementation approach:**  
- Continuous session state replication from primary to standby via internal streaming
- Standby runs in read-only mode, maintaining warm caches and plugin state
- Failure detection via heartbeat (configurable interval, default 1s)
- Standby promotion: update DNS/load balancer, start accepting connections
- Support active-passive and active-active modes

---

## 5. Protocol Extensibility

### 5.1 Custom Protocol Handler Registry

| Attribute | Value |
|-----------|-------|
| **Priority** | High |
| **Complexity** | Medium |

**Description:**  
Allow plugins to register custom protocol handlers that operate at the transport layer. A plugin can define its own binary protocol, framing, and handshake that runs over raw TCP, enabling integration with existing game protocols (e.g., Minecraft, Source Engine).

**Why it's needed:**  
Many games and applications have existing binary protocols that clients already speak. Forcing them to adapt to Draox's protocol creates an adoption barrier. By allowing custom protocol handlers, Draox can act as a drop-in replacement for existing servers.

**Implementation approach:**  
- Define `ProtocolHandler` trait in `plugin-sdk`: `fn on_bytes(&self, buf: &[u8]) -> Vec<Message>`
- Register handlers via `plugin-host` with a port or protocol-detection heuristic
- Protocol detection: inspect first N bytes to route to the correct handler
- Built-in handlers: Draox native, raw binary, line-delimited JSON

---

### 5.2 Protocol Bridge / Translator

| Attribute | Value |
|-----------|-------|
| **Priority** | Medium |
| **Complexity** | Medium |

**Description:**  
Bidirectional protocol translation between different transport protocols. A message published via WebSocket can be received by a TCP client, and vice versa, with automatic format translation (JSON ↔ binary, protobuf ↔ JSON).

**Why it's needed:**  
In a multi-protocol server, clients on different transports need to communicate seamlessly. A web dashboard (WebSocket + JSON) should receive the same events as a game client (TCP + binary). Without a bridge, plugins must handle format translation themselves.

**Implementation approach:**  
- Define `MessageCodec` trait: `fn encode(&self, msg: &Message, format: Format) -> Bytes`
- Register codecs per protocol in `socket-server`
- Automatic translation at delivery time based on the recipient's protocol
- Support: JSON, MessagePack, Protobuf, FlatBuffers, custom binary

---

### 5.3 Server-to-Server Federation

| Attribute | Value |
|-----------|-------|
| **Priority** | Medium |
| **Complexity** | High |

**Description:**  
Multiple Draox server clusters can federate, allowing clients on one cluster to communicate with clients on another. Federation uses authenticated, encrypted inter-cluster links with selective topic routing.

**Why it's needed:**  
For global deployments where latency requires regional clusters (US, EU, Asia), but some interactions need to cross regions (global chat, cross-region matchmaking, unified admin view). Without federation, each cluster is an isolated silo.

**Implementation approach:**  
- Inter-cluster link protocol: mTLS-authenticated TCP streams between cluster gateways
- Selective routing rules: which topics/namespaces are federated
- Message dedup across federation links (to prevent loops)
- Admin API: `POST /admin/federation/links`, `GET /admin/federation/status`
- Support hub-and-spoke and mesh topologies

---

## 6. Developer Experience

### 6.1 Plugin Scaffold Generator CLI

| Attribute | Value |
|-----------|-------|
| **Priority** | High |
| **Complexity** | Low |

**Description:**  
A CLI tool (`draox plugin new`) that generates a complete plugin project skeleton with Cargo.toml, plugin.toml manifest, src/lib.rs with trait implementations, tests, CI config, and documentation templates.

**Why it's needed:**  
The plugin SDK defines traits and types, but creating a new plugin requires setting up a Cargo project, configuring WASM targets, writing the manifest, and implementing the correct trait methods. A scaffold generator reduces time-to-first-plugin from hours to minutes.

**Implementation approach:**  
- Add `draox-cli` binary crate to the workspace
- Template engine: `minijinja` or `tera` for generating files from templates
- Templates stored in `templates/plugin/` directory
- Support: `draox plugin new <name>`, `draox plugin build`, `draox plugin package`
- Generate: Cargo.toml, plugin.toml, src/lib.rs, tests/, .github/workflows/

---

### 6.2 Debug Protocol Inspector

| Attribute | Value |
|-----------|-------|
| **Priority** | High |
| **Complexity** | Medium |

**Description:**  
A built-in protocol inspector that captures and displays all messages flowing through a connection in real-time. Accessible via admin WebSocket stream with filtering by connection ID, topic, message type, or content pattern.

**Why it's needed:**  
Debugging real-time socket applications is notoriously difficult. Developers need to see exactly what messages are being sent and received, their timing, and their content. Without a built-in inspector, they resort to packet captures (Wireshark) which can't decrypt TLS or understand the application protocol.

**Implementation approach:**  
- Add a tap/mirror capability in `connection-manager`: clone messages matching a filter to an inspector stream
- Admin WebSocket stream: `ws://admin:9100/ws/inspect?connection_id=X&topic=Y`
- Support filters: connection ID, topic pattern, message type, content regex
- Rate limit inspector output to prevent overwhelming the admin client
- Security: require admin auth, log all inspection sessions to `activity-log`

---

### 6.3 Interactive Playground / REPL

| Attribute | Value |
|-----------|-------|
| **Priority** | Medium |
| **Complexity** | Medium |

**Description:**  
A web-based interactive playground (served by `admin-api`) where developers can connect as a client, publish/subscribe to topics, invoke plugin APIs, and see real-time message flow — all from the browser.

**Why it's needed:**  
Lowering the barrier to experimentation is critical for adoption. A playground lets developers explore the server's capabilities without writing a client, test plugin interactions, and demonstrate features to stakeholders.

**Implementation approach:**  
- Serve a single-page application from `admin-api` at `/playground`
- WebSocket connection from the playground to the server (with admin-level access)
- Features: topic browser, message publisher, subscription manager, connection simulator
- Syntax-highlighted message display with JSON/binary toggle
- Share playground sessions via URL (connection state encoded in URL hash)

---

### 6.4 Local Development Emulator

| Attribute | Value |
|-----------|-------|
| **Priority** | Medium |
| **Complexity** | Medium |

**Description:**  
A lightweight, single-binary development mode that runs Draox with in-memory storage, no external dependencies (no Redis, no PostgreSQL), and auto-reload on plugin changes. Designed for `cargo watch` integration.

**Why it's needed:**  
The full Draox stack requires PostgreSQL, Redis, and possibly MongoDB. Setting up this infrastructure just to develop a plugin is a significant barrier. A local emulator with in-memory backends lets developers focus on plugin logic without infrastructure overhead.

**Implementation approach:**  
- Add `--dev` flag to the server binary
- Dev mode: SQLite in-memory for `data-store`, moka for `cache-layer`, no external deps
- Auto-reload: watch plugin WASM files and hot-swap on change
- Seed data: load sample connections and messages from `fixtures/` directory
- Pre-configure a single namespace with relaxed limits

---

## 7. Enterprise & Compliance

### 7.1 SSO / SAML / OIDC Integration

| Attribute | Value |
|-----------|-------|
| **Priority** | High |
| **Complexity** | Medium |

**Description:**  
Support enterprise Single Sign-On for the admin API via SAML 2.0 and OpenID Connect (OIDC). Admin users authenticate through their corporate identity provider (Okta, Azure AD, Google Workspace) instead of local credentials.

**Why it's needed:**  
Enterprise customers require SSO for all internal tools. Without it, Draox admin access requires separate credentials, which violates security policies and creates credential management overhead. SSO is a checkbox item on every enterprise procurement checklist.

**Implementation approach:**  
- Add `openidconnect` crate for OIDC and a SAML library for SAML 2.0
- Configure via `[admin.auth.sso]` with provider URL, client ID, redirect URIs
- Map OIDC claims / SAML attributes to Draox RBAC roles
- Support both SP-initiated and IdP-initiated flows
- Fallback to JWT/API key auth when SSO is not configured

---

### 7.2 API Versioning Strategy

| Attribute | Value |
|-----------|-------|
| **Priority** | High |
| **Complexity** | Low |

**Description:**  
Version the admin API using URL path versioning (`/api/v1/`, `/api/v2/`). Maintain backward compatibility for N-1 versions. Deprecation headers warn clients when they use old API versions.

**Why it's needed:**  
The admin API has ~72 endpoints that will evolve. Without versioning, any breaking change forces all API consumers to update simultaneously. Versioning enables gradual migration and protects automation scripts from breaking unexpectedly.

**Implementation approach:**  
- Add version prefix to all admin API routes: `/api/v1/connections`, `/api/v2/connections`
- Route to version-specific handlers in `admin-api`
- Add `Deprecation` and `Sunset` headers to deprecated versions
- Support version negotiation via `Accept-Version` header as alternative
- Document version lifecycle: active → deprecated (6 months warning) → removed

---

### 7.3 Compliance Data Export

| Attribute | Value |
|-----------|-------|
| **Priority** | Medium |
| **Complexity** | Medium |

**Description:**  
Export all data associated with a specific user/connection for GDPR Article 15 (right of access) and Article 17 (right to erasure) compliance. Generates a structured archive of all stored messages, metadata, plugin data, and activity logs.

**Why it's needed:**  
Any service handling data from EU users must support data portability and deletion requests. Without a built-in export mechanism, compliance requires manual database queries across multiple stores, which is error-prone and slow.

**Implementation approach:**  
- Add `POST /admin/compliance/export/{user_id}` — generates a ZIP archive
- Add `POST /admin/compliance/erase/{user_id}` — deletes all user data across all stores
- Scan all data stores: `data-store`, `cache-layer`, `activity-log`, plugin storage
- Include audit log of the export/erasure itself
- Support scheduled automatic data retention/purge policies

---

### 7.4 SLA Monitoring & Tracking

| Attribute | Value |
|-----------|-------|
| **Priority** | Medium |
| **Complexity** | Medium |

**Description:**  
Track and report Service Level Agreement metrics: uptime percentage, message delivery latency percentiles (p50, p95, p99), error rates, and connection success rates. Generate SLA compliance reports and trigger alerts when SLA thresholds are at risk.

**Why it's needed:**  
Enterprise customers require SLA guarantees (e.g., 99.95% uptime, <100ms p99 latency). Without built-in SLA tracking, proving compliance requires external monitoring tools and manual report generation. Built-in SLA tracking enables self-service SLA dashboards.

**Implementation approach:**  
- Define SLA targets in `server-config`: `[sla] uptime_target = 99.95, latency_p99_ms = 100`
- Continuously compute SLA metrics from `activity-log` data
- Rolling window calculation (1h, 24h, 7d, 30d)
- Admin API: `GET /admin/sla/report?window=30d`
- Alert when error budget consumption rate exceeds threshold

---

## 8. Observability Enhancements

### 8.1 Adaptive Telemetry Sampling

| Attribute | Value |
|-----------|-------|
| **Priority** | High |
| **Complexity** | Medium |

**Description:**  
Dynamically adjust telemetry sampling rates based on server load and error rates. Under normal operation, sample 1% of traces. When errors spike, automatically increase to 100% sampling for affected topics/connections to capture diagnostic data.

**Why it's needed:**  
Full telemetry at scale (100K+ connections, millions of messages/sec) generates enormous data volumes that overwhelm storage and analysis tools. Static sampling misses rare but critical error conditions. Adaptive sampling balances cost with diagnostic coverage.

**Implementation approach:**  
- Implement a sampling decision engine in `activity-log`
- Base rate: configurable (default 1% of traces)
- Error-triggered boost: increase sampling to 100% for connections/topics experiencing errors
- Tail-based sampling: keep traces that contain errors even if initially sampled out
- Integrate with OpenTelemetry's `TraceIdRatioBased` and `ParentBased` samplers
- Admin API: `GET /admin/telemetry/sampling`, `PUT /admin/telemetry/sampling`

---

## 9. Data & Message Patterns

### 9.1 Schema Registry

| Attribute | Value |
|-----------|-------|
| **Priority** | High |
| **Complexity** | Medium |

**Description:**  
A built-in schema registry that stores and versions message schemas (JSON Schema, Protobuf, Avro). The server can optionally validate messages against their registered schema before delivery, catching malformed data at the source.

**Why it's needed:**  
Without schema validation, a plugin publishing malformed messages causes cascading failures across all subscribers. A schema registry enables: contract-first development, automatic client code generation, backward compatibility checking, and runtime validation.

**Implementation approach:**  
- Store schemas in `data-store` with versioning (schema ID + version number)
- Support formats: JSON Schema, Protobuf descriptors, Avro schemas
- Validation modes: `none`, `warn` (log + deliver), `strict` (reject invalid)
- Compatibility checking: backward, forward, full (following Confluent Schema Registry semantics)
- Admin API: `POST /admin/schemas`, `GET /admin/schemas/{id}/versions`
- Plugin SDK: `ctx.validate_message(schema_id, &msg)`

---

### 9.2 Connection Tagging & Metadata

| Attribute | Value |
|-----------|-------|
| **Priority** | High |
| **Complexity** | Low |

**Description:**  
Allow clients and plugins to attach arbitrary key-value tags to connections. Tags are queryable via admin API and usable in routing rules, rate limit policies, and drain filters.

**Why it's needed:**  
Connections are currently identified by ID, protocol, and role. But operations teams need richer metadata: which game version, which region, which A/B test group, which feature flags. Tags enable operational segmentation without protocol changes.

**Implementation approach:**  
- Add `tags: HashMap<String, String>` to connection metadata in `connection-manager`
- Client can set tags during handshake or via a `SET_TAGS` message
- Plugins can set/read tags via `plugin-sdk`
- Admin API: `GET /admin/connections?tag.region=us-east`
- Use tags in `traffic-guard` rules: `rate_limit.rules = [{ match: { tag.tier: "free" }, limit: 10 }]`

---

### 9.3 Inbound Webhook Gateway

| Attribute | Value |
|-----------|-------|
| **Priority** | Medium |
| **Complexity** | Medium |

**Description:**  
An HTTP endpoint that accepts webhook payloads from external services (GitHub, Stripe, Twilio, etc.) and publishes them as messages to specified topics. Includes signature verification, retry handling, and payload transformation.

**Why it's needed:**  
Modern applications integrate with dozens of external services that communicate via webhooks. Without a built-in gateway, each integration requires a separate webhook handler, a way to get the payload into the socket server, and custom signature verification. A unified gateway standardizes this.

**Implementation approach:**  
- Add `POST /webhooks/{topic}` endpoint in `admin-api` (separate from admin auth)
- Signature verification: HMAC-SHA256 (GitHub, Stripe), custom headers
- Payload transformation: JSONPath extraction, field mapping
- Configure via `[webhooks]` section: `{ topic, secret, verify_signature, transform }`
- Dead-letter queue for failed webhook processing
- Replay endpoint: `POST /admin/webhooks/replay/{id}`

---

## Priority Matrix

| Priority | Features | Count |
|----------|----------|-------|
| **Critical** | PROXY Protocol v2, Namespace Isolation, Reconnection Token, Pub/Sub Wildcards, Zero-Downtime Restart | 5 |
| **High** | Request-Reply, Message Dedup, Ordering, Rate Limit Headers, Protocol Negotiation, Multiplexing, Plugin Storage Isolation, Drain API, Protocol Handlers, Scaffold CLI, Debug Inspector, SSO, API Versioning, Adaptive Sampling, Schema Registry, Connection Tagging | 16 |
| **Medium** | Message TTL, Scheduled Delivery, Event Replay, Delta Sync, Quality Scoring, WebTransport, Tenant Config, Chaos Hooks, Canary Plugins, Warm Standby, Protocol Bridge, Federation, Playground, Local Emulator, Compliance Export, SLA Tracking, Webhook Gateway | 17 |

---

## Roadmap Integration

These 38 features integrate into the existing 4-phase roadmap from `missing_features.md`:

### Phase 1 (Weeks 1-8) — Add:
- PROXY Protocol v2 (deployment blocker, low complexity)
- Rate Limit Response Headers (low complexity, high impact)
- Protocol Version Negotiation (low complexity, future-proofs protocol)
- Connection Tagging (low complexity, enables operational tooling)
- API Versioning Strategy (low complexity, must be done before API stabilizes)

### Phase 2 (Weeks 9-16) — Add:
- Pub/Sub with Wildcard Topics (core messaging primitive)
- Request-Reply Pattern (core messaging primitive)
- Message Deduplication (reliability primitive)
- Reconnection Token with State Resume (user experience critical)
- Namespace Isolation (multi-tenancy foundation)
- Plugin Scaffold Generator CLI (developer adoption)
- Debug Protocol Inspector (developer productivity)

### Phase 3 (Weeks 17-24) — Add:
- Message Ordering Guarantees (reliability)
- Connection Multiplexing (performance)
- Per-Tenant Plugin Storage Isolation (multi-tenancy)
- Zero-Downtime Rolling Restart (operational maturity)
- Connection Drain API (operational tooling)
- SSO / SAML / OIDC (enterprise readiness)
- Schema Registry (data governance)
- Adaptive Telemetry Sampling (observability at scale)

### Phase 4 (Weeks 25+) — Add:
- Message TTL, Scheduled Delivery, Event Replay (advanced messaging)
- Delta Sync, WebTransport (advanced transport)
- Chaos Engineering, Canary Plugins, Warm Standby (operational excellence)
- Protocol Bridge, Federation (advanced networking)
- Playground, Local Emulator (developer experience)
- Compliance Export, SLA Tracking (enterprise compliance)
- Webhook Gateway (integration)
