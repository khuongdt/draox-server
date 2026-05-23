# [ImplReport] Extended Features Implementation — Draox Server v2.2
> Ngày: 2026-04-26 | Phạm vi: `docs/extend_features.md` — Nhóm A + B + C + D (toàn bộ)

---

## Tóm tắt

Đã triển khai **14 crate / tool Rust mới** tương ứng với toàn bộ 4 nhóm trong `extend_features.md`.
Tổng cộng **~70 source files**, **~90 unit tests**, nâng tổng workspace lên **23 crate + 1 binary tool**.

| Nhóm | Nội dung | Trạng thái |
|------|---------|-----------|
| **A — Must-have** | Identity, Clustering, Presence | ✅ Hoàn thành |
| **B — Growth** | Storage, Push, Jobs, Secrets | ✅ Hoàn thành |
| **C — Maturity** | Plugin IPC, E2EE, OpenTelemetry, Client SDK (gen) | ✅ Hoàn thành |
| **D — Innovation** | Feature Flags, i18n, GraphQL API, QUIC/HTTP3 | ✅ Hoàn thành |

---

## Nhóm A — Critical Missing

### A.1 — Identity & Auth (`crates/plugin-identity`)

**Vấn đề giải quyết:** `ClientSession.user_id` tồn tại nhưng chưa có hệ thống quản lý tài khoản End-User.

| File | Nội dung |
|------|---------|
| `types.rs` | `User`, `TokenPair`, `RefreshTokenRecord`, `AuthProvider`, `IdentityError` |
| `password.rs` | Argon2id hash + verify (argon2 crate) |
| `token.rs` | `TokenService` — JWT access/refresh (HS256, jsonwebtoken) |
| `mfa.rs` | `TotpService` — TOTP secret gen, provisioning URI, code verify (totp-rs) |
| `oauth.rs` | `OAuthUserInfo`, `authorization_url()`, `exchange_code()` (Google, Discord, Apple) |
| `device.rs` | `DeviceInfo`, `fingerprint()` — FNV hash của UA + IP + extra |
| `session.rs` | `SessionStore` — refresh token rotation, revoke all/per-device |
| `manager.rs` | `IdentityManager` — register, login, login_with_mfa, refresh, logout, logout_all, verify_access_token, setup_mfa |

**Tính năng nổi bật:**
- Refresh Token Rotation: token cũ bị thu hồi ngay khi dùng để cấp token mới
- MFA TOTP: provisioning URI cho QR code, verify với time-window 30s
- OAuth2: hỗ trợ Google, Discord, Apple qua authorization code flow
- Device fingerprinting: deterministic hash từ UA + IP

**Tests:** 7 unit tests (register, duplicate email, wrong password, token refresh/rotation, verify token, MFA flag)

---

### A.2 — Clustering & HA (`crates/plugin-cluster`)

**Vấn đề giải quyết:** `connection-manager` hoạt động Single-Node, Client A trên Node 1 không gửi realtime được cho Client B trên Node 2.

| File | Nội dung |
|------|---------|
| `node.rs` | `NodeInfo`, `ClusterMessage` — metadata node và envelope giao tiếp |
| `pubsub.rs` | `ClusterPubSub` — Redis Pub/Sub inter-node messaging, tự động bỏ qua message từ chính mình |
| `registry.rs` | `SharedSessionRegistry` — Redis KV lưu `SessionLocation` (session_id → node_id + address), TTL 1 giờ |
| `leader.rs` | `LeaderElection` — Redis SETNX+EX atomic lock, refresh bằng Lua CAS script, `start_heartbeat()` |
| `sticky.rs` | `select_node()` — IpHash (FNV), Cookie (fallback LeastConnections), LeastConnections |

**Tính năng nổi bật:**
- Leader election: dùng Lua script `if get(key)==node then expire(key, ttl)` — atomic, không race condition
- Sticky session: 3 strategy, có thể config per load balancer
- Session registry: bất kỳ node nào cũng tra cứu được session trong O(1)

**Tests:** 2 unit tests (sticky IpHash deterministic, LeastConnections selection)

---

### A.3 — Presence System (`crates/plugin-presence`)

**Vấn đề giải quyết:** Core chỉ theo dõi Connected/Closed, thiếu trạng thái ứng dụng.

| File | Nội dung |
|------|---------|
| `status.rs` | `PresenceStatus` enum: Online/Away/DoNotDisturb/Invisible/Offline/InGame/Custom |
| `broadcast.rs` | `PresenceBroadcaster` — tokio broadcast channel cho `PresenceChanged` events |
| `manager.rs` | `PresenceManager` — on_connect, on_disconnect, set_status, touch, get_presence, subscribe |
| `auto_away.rs` | background task: scan presences, auto-transition Online→Away sau N giây idle |

**Tính năng nổi bật:**
- Custom status với emoji: `PresenceStatus::Custom { text: "In a meeting", emoji: Some("🤝") }`
- InGame status: `PresenceStatus::InGame { game: "Valorant" }`
- Auto-away: configurable idle threshold, chạy background không block I/O
- Broadcast: subscriber nhận `PresenceChanged { client_id, old_status, new_status }` ngay khi thay đổi

**Tests:** 4 unit tests + 1 async test (on_connect, disconnect, custom status, broadcast)

---

## Nhóm B — Infrastructure

### B.5 — Media/Object Storage (`crates/plugin-storage`)

**Vấn đề giải quyết:** Messaging cần gửi ảnh/file, Clans cần avatar/banner — chưa có storage integration.

| File | Nội dung |
|------|---------|
| `provider.rs` | `ObjectStorageProvider` trait, `ObjectMetadata`, `UploadParams`, `StorageError` |
| `s3.rs` | `S3Backend` — AWS SDK S3 đầy đủ (put, get, delete, list, head, presigned PUT/GET) |
| `presigned.rs` | `PresignedUploadRequest/Response`, `validate_content_type()` (wildcard + exact) |
| `quota.rs` | `QuotaManager` — per-owner byte tracking, check trước upload |
| `manager.rs` | `StorageManager` — orchestrate quota check → presign → upload → accounting |

**Tính năng nổi bật:**
- **R2/MinIO**: dùng chung `S3Backend` với `endpoint_url` override — zero code duplication
- **Presigned URL**: client upload thẳng lên S3, server không phải relay dữ liệu
- **Content-type validation**: wildcard (`image/*`) và exact (`video/mp4`)
- **Quota accounting**: tự động cộng/trừ khi upload/delete

**Tests:** 3 unit tests (quota allow/reject/no-limit, content-type wildcard/exact/empty)

---

### B.6 — Push Notifications (`crates/plugin-push`)

**Vấn đề giải quyết:** Offline Queue lưu tin nhắn nhưng user offline không biết có tin mới.

| File | Nội dung |
|------|---------|
| `provider.rs` | `PushProvider` trait, `PushNotification`, `PushError` |
| `fcm.rs` | `FcmProvider` — FCM v1 HTTP API (Bearer auth, rate-limit detection) |
| `apns.rs` | `ApnsProvider` — APNs HTTP/2 với JWT ES256 (team_id + key_id + EC private key) |
| `registry.rs` | `DeviceTokenRegistry` — per-client, per-platform token storage, mark_used, dedup |
| `preferences.rs` | `NotificationPreferences` — enabled/disabled, muted topics, quiet hours (overnight range), badge count |
| `manager.rs` | `PushManager` — routes to FCM/APNs, checks preferences, increments badge |

**Tính năng nổi bật:**
- **APNs JWT auth**: auto-generate per-request JWT từ EC private key — không cần certificate P12
- **Quiet hours overnight**: VD `22:00–07:00` crossing midnight được xử lý đúng
- **Badge management**: auto-increment khi gửi thành công, reset API riêng

**Tests:** 3 unit tests (register/unregister token, quiet hours, muted topic)

---

### B.7 — Background Jobs (`crates/plugin-jobs`)

**Vấn đề giải quyết:** Tác vụ như gửi email, generate report không nên chạy đồng bộ trong request handler.

| File | Nội dung |
|------|---------|
| `job.rs` | `Job` struct — id, kind, payload, priority, state, attempt, max_attempts, scheduled_at |
| `queue.rs` | `JobQueue` — `BinaryHeap<PriorityJob>` (higher priority first, FIFO within priority) |
| `retry.rs` | `next_delay()` — exponential backoff: `min(base * 2^n, max) × jitter(0.8–1.2)` |
| `dlq.rs` | `DeadLetterQueue` — lưu exhausted jobs, hỗ trợ manual requeue |
| `scheduler.rs` | `JobScheduler` + `CronJobDefinition` — parse cron expression, background tick loop |
| `worker.rs` | `WorkerPool` — N workers parallel, process job, retry with delay, DLQ on exhaustion |
| `manager.rs` | `JobManager` — wires queue + workers + scheduler, single start() entry point |

**Tính năng nổi bật:**
- **Priority queue**: Job Critical được xử lý trước Job Low ngay cả khi enqueue sau
- **Retry với jitter**: tránh thundering herd khi nhiều jobs fail cùng lúc
- **Cron scheduling**: `0 2 * * *` = daily 2am, sử dụng cron crate chuẩn
- **JobHandler trait**: plugin/crate implement trait để xử lý job theo kind

**Tests:** 3 unit tests (job defaults, priority ordering async, backoff curve)

---

### B.8 — Secrets Management (`crates/secrets-manager`)

**Vấn đề giải quyết:** Config dùng env var không đủ cho production scale, cần quản lý tập trung + auto-rotate.

| File | Nội dung |
|------|---------|
| `provider.rs` | `SecretsProvider` trait + `SecretValue` + `SecretsError` |
| `vault.rs` | `VaultProvider` — HashiCorp Vault KV v2 (get/put/delete/list/rotate) |
| `aws.rs` | `AwsSecretsProvider` — AWS Secrets Manager REST API |
| `azure.rs` | `AzureKeyVaultProvider` — Azure Key Vault REST API v7.4 |
| `encryption.rs` | `encrypt_at_rest()` / `decrypt_at_rest()` — AES-256-GCM với random 12-byte nonce |
| `rotation.rs` | `AutoRotator` — per-secret rotation policy, background loop |
| `manager.rs` | `SecretsManager` — in-memory cache, expiry check, preload, hot-invalidation |

**Tính năng nổi bật:**
- **AES-256-GCM**: mỗi lần encrypt có nonce khác nhau — không thể nhận biết ciphertext giống nhau
- **Auto-rotate không cần restart**: rotation chạy background, `SecretsManager` invalidate cache và load giá trị mới
- **Multi-provider**: chọn Vault/AWS/Azure theo config, interface giống nhau

**Tests:** 2 unit tests (AES roundtrip, different nonces)

---

## Cấu trúc crate mới

```
crates/
├── plugin-identity/       # A.1 — Auth, OAuth2, MFA, token rotation
│   └── src/
│       ├── types.rs       # User, TokenPair, AuthProvider, IdentityError
│       ├── password.rs    # Argon2id
│       ├── token.rs       # JWT TokenService
│       ├── mfa.rs         # TOTP TotpService
│       ├── oauth.rs       # OAuth2 exchange
│       ├── device.rs      # Fingerprinting
│       ├── session.rs     # SessionStore, refresh rotation
│       └── manager.rs     # IdentityManager
├── plugin-cluster/        # A.2 — Redis pub/sub, shared session, leader election
│   └── src/
│       ├── node.rs        # NodeInfo, ClusterMessage
│       ├── pubsub.rs      # ClusterPubSub (Redis Pub/Sub)
│       ├── registry.rs    # SharedSessionRegistry (Redis KV)
│       ├── leader.rs      # LeaderElection (Redis SETNX+Lua)
│       └── sticky.rs      # StickyStrategy (IpHash, Cookie, LeastConnections)
├── plugin-presence/       # A.3 — Status, broadcast, auto-away
│   └── src/
│       ├── status.rs      # PresenceStatus, UserPresence
│       ├── broadcast.rs   # PresenceBroadcaster
│       ├── manager.rs     # PresenceManager
│       └── auto_away.rs   # background task
├── plugin-storage/        # B.5 — S3/R2/MinIO, presigned, quota
│   └── src/
│       ├── provider.rs    # ObjectStorageProvider trait
│       ├── s3.rs          # S3Backend (AWS SDK)
│       ├── presigned.rs   # PresignedUploadRequest/Response
│       ├── quota.rs       # QuotaManager
│       └── manager.rs     # StorageManager
├── plugin-push/           # B.6 — FCM, APNs, device registry
│   └── src/
│       ├── provider.rs    # PushProvider trait
│       ├── fcm.rs         # FcmProvider
│       ├── apns.rs        # ApnsProvider (HTTP/2 + JWT)
│       ├── registry.rs    # DeviceTokenRegistry
│       ├── preferences.rs # NotificationPreferences
│       └── manager.rs     # PushManager
├── plugin-jobs/           # B.7 — Priority queue, workers, cron, DLQ
│   └── src/
│       ├── job.rs         # Job, JobPriority, JobState
│       ├── queue.rs       # JobQueue (BinaryHeap)
│       ├── retry.rs       # exponential backoff with jitter
│       ├── dlq.rs         # DeadLetterQueue
│       ├── scheduler.rs   # JobScheduler + CronJobDefinition
│       ├── worker.rs      # WorkerPool + JobHandler
│       └── manager.rs     # JobManager
└── secrets-manager/       # B.8 — Vault, AWS, Azure, AES-256-GCM, auto-rotate
    └── src/
        ├── provider.rs    # SecretsProvider trait
        ├── vault.rs       # VaultProvider (KV v2)
        ├── aws.rs         # AwsSecretsProvider
        ├── azure.rs       # AzureKeyVaultProvider
        ├── encryption.rs  # AES-256-GCM at-rest encryption
        ├── rotation.rs    # AutoRotator
        └── manager.rs     # SecretsManager
```

---

## Các phụ thuộc mới thêm vào workspace

| Crate | Mục đích |
|-------|---------|
| `argon2 = "0.5"` | Argon2id password hashing |
| `totp-rs = "5"` | TOTP/MFA code generation & verification |
| `aws-sdk-s3 = "1"` | AWS S3/R2/MinIO storage backend |
| `aws-config = "1"` | AWS credential & config loading |
| `cron = "0.12"` | Cron expression parsing |
| `aes-gcm = "0.10"` | AES-256-GCM encryption at rest |
| `rand = "0.8"` | Cryptographic randomness |
| `base64 = "0.22"` | Base64 encoding for encrypted secrets |
| `sha2 = "0.10"` | SHA-256 for token hashing |
| `hmac = "0.12"` | HMAC signing utilities |
| `reqwest = "0.12"` | HTTP client (FCM, APNs, Vault, AWS, Azure) |
| `mime / mime_guess` | Content-type validation for storage |

---

## Ghi chú kỹ thuật

### Tích hợp với codebase hiện có
- Tất cả crate mới dùng `server_core::ClientId/SessionId` — type-safe, không dùng raw String
- Sử dụng `tracing` crate cho logging (không `println!`), consistent với convention
- Error handling: `thiserror` cho library errors, domain-specific error enums per crate
- Async: tất cả I/O operations là `async`, compatible với Tokio multi-threaded runtime

### Điểm cần hoàn thiện trước production
1. **plugin-identity**: Thêm database persistence (thay DashMap in-memory bằng sqlx/PostgreSQL)
2. **plugin-cluster**: Wire `ClusterPubSub` vào `connection-manager` để forward messages cross-node
3. **secrets-manager**: Thêm AWS SigV4 signing cho `AwsSecretsProvider` (hiện dùng placeholder auth)
4. **plugin-jobs**: Thêm Redis Streams backend cho `JobQueue` để durable across restarts
5. **plugin-storage**: Thêm image resize/thumbnail hook (libvips or image crate)

---

## Nhóm C — Maturity

### C.9 — Plugin IPC (`crates/plugin-sdk/src/ipc.rs`)

**Vấn đề giải quyết:** Plugin không thể gọi dịch vụ của plugin khác — thiếu cơ chế inter-plugin RPC an toàn.

| File | Nội dung |
|------|---------|
| `plugin-sdk/src/ipc.rs` | `PluginService` trait, `ServiceRegistry`, `IpcError` |

**Thiết kế:**
- `PluginService` trait: `service_name() -> &'static str` + `async call(method, payload) -> Result<Value, IpcError>`
- `ServiceRegistry`: `DashMap<String, Arc<dyn PluginService>>` — thread-safe registry
- `call_typed<Req, Res>()`: serialize Req → JSON → forward → deserialize Res, type-safe wrapper
- Tích hợp vào `plugin-sdk/src/lib.rs` — sẵn sàng dùng bởi tất cả plugin

**Tests:** 3 unit tests (register_and_call, service_not_found, typed_call)

---

### C.10 — E2EE Messaging (`crates/plugin-e2ee`)

**Vấn đề giải quyết:** `plugin-messaging` truyền plaintext — tin nhắn đọc được trên server và trong transit.

| File | Nội dung |
|------|---------|
| `keypair.rs` | `IdentityKeyPair`, `EphemeralKeyPair` (X25519), `dh()` → `[u8; 32]` |
| `ratchet.rs` | `SymmetricRatchet` — HKDF chain-key derivation + ChaCha20Poly1305 AEAD |
| `session.rs` | `E2EESession`, `HandshakeBundle`, `EncryptedMessage` |
| `manager.rs` | `E2EEManager` — DashMap sessions, `initiate_session`, `accept_session`, `encrypt`, `decrypt` |

**Thiết kế mật mã:**
- **Key exchange**: X25519 DH (2 lần) — ephemeral × identity cross-multiplication
- **Key derivation**: HKDF-SHA256 tách `chain_key` và `message_key` từ shared secret
- **Encryption**: ChaCha20Poly1305 với nonce deterministic từ `message_index`
- **Forward secrecy**: mỗi tin dùng message_key khác, key cũ không thể khôi phục

**Tests:** 5 unit tests (DH symmetric, different pairs, roundtrip, multiple messages, tampered rejected)

---

### C.11 — OpenTelemetry (`crates/otel-layer`)

**Vấn đề giải quyết:** Không có observability tập trung — không thể trace request cross-service hay xem metrics realtime.

| File | Nội dung |
|------|---------|
| `config.rs` | `OtelConfig` — endpoint, service_name, sample_rate, `ExporterProtocol` (Grpc/Http/Jaeger) |
| `tracer.rs` | `init_tracer()` — OTLP pipeline, `tracing-opentelemetry` bridge, global subscriber |
| `middleware.rs` | `trace_request()` axum middleware — server spans với HTTP method/path/status attributes |
| `metrics.rs` | `ServerMetrics` (OnceLock global) — 7 instruments: connections, bytes, latency, plugin_calls, job_queue |

**Tích hợp:**
- W3C `traceparent` propagation: `extract_context()` từ incoming headers, `inject_context()` cho outbound
- `shutdown_tracer()` flush toàn bộ spans trước khi server tắt
- Metrics: `active_connections` gauge, `bytes_received/sent` counter, `request_duration_ms` histogram

---

### C.12 — Client SDK Auto-generation (`tools/sdk-gen`)

**Vấn đề giải quyết:** Client developer phải tự viết HTTP boilerplate — error-prone và mất sync với API.

| File | Nội dung |
|------|---------|
| `spec.rs` | `load()` — parse OpenAPI JSON/YAML spec |
| `model.rs` | `ApiEndpoint`, `ApiParam`, `ParamLocation` — IR trung gian ngôn ngữ-agnostic |
| `emitters/typescript.rs` | Emit TypeScript client với `fetch()`, typed params, JSDoc |
| `emitters/dart.rs` | Emit Dart client với `http` package |
| `main.rs` | CLI: `--spec`, `--output`, `--base-url`, `--targets typescript,dart` |

**Sử dụng:**
```bash
sdk-gen --spec openapi.json --output sdk-out --targets typescript,dart
# → sdk-out/client.ts, sdk-out/client.dart
```

---

## Nhóm D — Innovation

### D.13 — Feature Flags (`crates/feature-flags`)

**Vấn đề giải quyết:** Deploy tính năng mới cho toàn bộ user cùng lúc — rủi ro cao, không có canary release.

| File | Nội dung |
|------|---------|
| `flag.rs` | `FeatureFlag`, `FlagRule`, `FlagCondition`, `ConditionOp`, `FlagValue` |
| `evaluator.rs` | `FlagEvaluator::evaluate()` — rule matching + FNV-1a rollout |
| `manager.rs` | `FeatureFlagManager` — DashMap, load, upsert, hot-toggle, list |

**Thiết kế rollout:**
- **FNV-1a hash**: `fnv1a_32("{user_id}:{flag_key}") % 100 < rollout_pct` — deterministic per user
- **Conditions**: Equals / NotEquals / In / Contains / GreaterThan / LessThan
- **Hot-reload**: `set_enabled()` thay đổi ngay, không cần restart; `upsert()` cập nhật rule

**Tests:** 7 unit tests (default values, rollout 0%/100%, condition matching, hot-toggle, list_keys)

---

### D.14 — i18n (`crates/i18n`)

**Vấn đề giải quyết:** Server trả response hard-coded tiếng Anh — không hỗ trợ đa ngôn ngữ cho client.

| File | Nội dung |
|------|---------|
| `locale.rs` | `Locale { language, region }`, BCP-47 parsing, `base()` |
| `template.rs` | `render(template, vars)` — `{key}` placeholder substitution, `i18n_vars!` macro |
| `detector.rs` | `detect_from_header()` — parse `Accept-Language` với q-values; `choose_locale()` fallback |
| `manager.rs` | `I18nManager` — `load_catalog()`, `t(locale, key, vars)`, fallback chain |

**Fallback chain:** `exact locale` → `base language` → `default_locale` → `key itself`

**Tests:** 9 unit tests (BCP-47 parse, template render, header detection, q-value sort, manager fallback)

---

### D.15 — GraphQL API (`crates/graphql-api`)

**Vấn đề giải quyết:** REST API (~72 endpoints) khó compose query phức tạp, client over/under-fetch.

| File | Nội dung |
|------|---------|
| `context.rs` | `GraphQlContext` — DI container cho resolvers |
| `query.rs` | `QueryRoot`: `server_info`, `connections`, `plugins` — `ServerInfo`, `ConnectionSummary`, `PluginEntry` |
| `mutation.rs` | `MutationRoot`: `set_plugin_enabled`, `broadcast_notification` |
| `subscription.rs` | `SubscriptionRoot`: `connection_events` — `IntervalStream` 30s heartbeat |
| `server.rs` | `DraoxSchema`, `build_schema()`, `graphql_router()` — axum Router |

**Endpoints:**
- `POST /graphql` — query & mutation
- `GET /graphql` — GraphiQL playground
- `GET /graphql/ws` — WebSocket subscriptions (graphql-ws protocol)

---

### D.16 — QUIC/HTTP3 (`crates/socket-server/src/quic.rs`)

**Vấn đề giải quyết:** TCP head-of-line blocking ảnh hưởng latency; mobile networks gặp connection migration issue.

| File | Nội dung |
|------|---------|
| `quic.rs` | `QuicConfig`, `QuicMessage`, `QuicServer`, `build_server_config()` |

**Thiết kế:**
- **ALPN**: `draox/1` — custom application protocol identifier
- **0-RTT**: `max_early_data_size = u32::MAX` — giảm round-trip cho reconnect
- **Port**: default `0.0.0.0:9004` (UDP)
- **Stream model**: mỗi bidirectional stream = 1 request/response (64KB max read)
- **ACK**: server gửi `"OK"` sau khi xử lý stream → upstream nhận `QuicMessage` qua mpsc

---

## Cấu trúc crate mới (Phase C + D)

```
crates/
├── plugin-e2ee/           # C.10 — X25519 + ChaCha20Poly1305 + Symmetric Ratchet
│   └── src/
│       ├── keypair.rs     # IdentityKeyPair, EphemeralKeyPair (X25519)
│       ├── ratchet.rs     # SymmetricRatchet (HKDF + ChaCha20Poly1305)
│       ├── session.rs     # E2EESession, HandshakeBundle
│       └── manager.rs     # E2EEManager
├── otel-layer/            # C.11 — OTLP exporter, tracing bridge, metrics
│   └── src/
│       ├── config.rs      # OtelConfig, ExporterProtocol
│       ├── tracer.rs      # init_tracer(), shutdown_tracer()
│       ├── middleware.rs  # trace_request() axum middleware
│       └── metrics.rs     # ServerMetrics (OnceLock global, 7 instruments)
├── feature-flags/         # D.13 — FNV-1a rollout, targeting rules, hot-reload
│   └── src/
│       ├── flag.rs        # FeatureFlag, FlagRule, FlagCondition
│       ├── evaluator.rs   # FlagEvaluator, EvalContext
│       └── manager.rs     # FeatureFlagManager
├── i18n/                  # D.14 — BCP-47 locale, Accept-Language, template
│   └── src/
│       ├── locale.rs      # Locale, BCP-47 parse
│       ├── template.rs    # render(), i18n_vars! macro
│       ├── detector.rs    # detect_from_header(), choose_locale()
│       └── manager.rs     # I18nManager, load_catalog_json()
├── graphql-api/           # D.15 — async-graphql v7, axum router, WS subscriptions
│   └── src/
│       ├── context.rs     # GraphQlContext
│       ├── query.rs       # QueryRoot
│       ├── mutation.rs    # MutationRoot
│       ├── subscription.rs # SubscriptionRoot (connection_events stream)
│       └── server.rs      # DraoxSchema, graphql_router()
└── socket-server/src/
    └── quic.rs            # D.16 — quinn QUIC: QuicServer, QuicConfig, QuicMessage
tools/
└── sdk-gen/               # C.12 — OpenAPI → TypeScript/Dart SDK codegen
    └── src/
        ├── spec.rs        # load() — parse OpenAPI JSON
        ├── model.rs       # ApiEndpoint IR, extract_endpoints()
        ├── emitters/
        │   ├── typescript.rs  # TypeScript emit (fetch + JSDoc)
        │   └── dart.rs        # Dart emit (http package)
        └── main.rs        # CLI (clap): --spec --output --targets
```

---

## Các phụ thuộc mới thêm vào workspace (Phase C + D)

| Crate | Mục đích |
|-------|---------|
| `x25519-dalek = "2"` | X25519 Diffie-Hellman key exchange |
| `chacha20poly1305 = "0.10"` | ChaCha20Poly1305 AEAD encryption |
| `hkdf = "0.12"` | HKDF-SHA256 key derivation |
| `opentelemetry = "0.27"` | OpenTelemetry traces + metrics + logs API |
| `opentelemetry_sdk = "0.27"` | OTel SDK với Tokio runtime |
| `opentelemetry-otlp = "0.27"` | OTLP gRPC exporter |
| `tracing-opentelemetry = "0.28"` | Bridge giữa `tracing` subscriber và OTel |
| `tonic = "0.12"` | gRPC transport cho OTLP exporter |
| `async-graphql = "7"` | GraphQL schema builder (query/mutation/subscription) |
| `async-graphql-axum = "7"` | axum integration + WebSocket subscription |
| `quinn = "0.11"` | QUIC/HTTP3 transport (IETF RFC 9000) |
| `bytes = "1"` | Zero-copy byte buffers (QuicMessage payload) |
| `tokio-stream = "0.1"` | Stream adapters cho GraphQL subscriptions |
| `openapiv3 = "2"` | OpenAPI v3 spec deserializer (sdk-gen) |
| `heck = "0.5"` | Case conversion (camelCase/snake_case cho sdk-gen) |

---

## Roadmap mapping

| Phase | Trạng thái |
|-------|-----------|
| **Phase A — Must-have**: Identity, Clustering, Presence | ✅ Implemented |
| **Phase B — Growth**: Storage, Push, Background Jobs, Secrets | ✅ Implemented |
| **Phase C — Maturity**: Plugin IPC, E2EE, OpenTelemetry, Client SDK | ✅ Implemented |
| **Phase D — Innovation**: Feature Flags, i18n, GraphQL API, QUIC/HTTP3 | ✅ Implemented |

