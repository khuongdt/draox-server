# Draox Server - Extended Features Proposal

> Dựa trên bản thiết kế kiến trúc **Draox Server v2.1** (`docs/design_en.html`)
> Ngày đề xuất: 2026-04-18

---

## Nhóm A — Thiếu & Cần thiết (Critical Missing)

### 1. Quản lý Người dùng & Xác thực (Identity & Auth Plugin)
* **Vấn đề:** `ClientSession` có `user_id` nhưng chưa có hệ thống quản lý tài khoản End-User.
* **Đề xuất:**
  * Tạo `plugin-identity`: Đăng ký, Đăng nhập (Argon2/bcrypt hash).
  * **OAuth2 / Social Login** (Google, Discord, Apple, Steam).
  * MFA (TOTP/WebAuthn), quản lý phiên (thu hồi token, logout từ xa).
  * Refresh Token rotation, device fingerprinting.

### 2. Clustering & High Availability
* **Vấn đề:** `connection-manager` và `plugin-messaging` hoạt động Single-Node. Client A ở Node 1 không gửi realtime được cho Client B ở Node 2.
* **Đề xuất:**
  * **Inter-node messaging** qua Redis Pub/Sub (đã có `fred`) hoặc NATS.
  * Shared session registry (Redis) để bất kỳ node nào cũng tra cứu được session.
  * Leader election cho các tác vụ singleton (cron, cleanup).
  * Sticky session support cho Load Balancer (IP hash / cookie).

### 3. Presence System (Trạng thái Hiện diện)
* **Vấn đề:** Core chỉ theo dõi Connected/Closed, thiếu trạng thái ứng dụng.
* **Đề xuất:**
  * `plugin-presence`: *Online, Offline, Away, DND, In-Game, Custom Status*.
  * Broadcast thay đổi trạng thái cho friend list / clan members.
  * Last-seen timestamp, auto-away sau N phút idle.

### 4. Friends & Social Graph
* **Vấn đề:** `plugin-clans` quản lý nhóm, nhưng thiếu quan hệ cá nhân 1:1.
* **Đề xuất:**
  * `plugin-friends`: Gửi/chấp nhận/từ chối lời mời kết bạn.
  * Block list (chặn user), mute (tắt thông báo).
  * Mutual friends, friend suggestions dựa trên clan chung.

---

## Nhóm B — Tăng cường Hạ tầng (Infrastructure)

### 5. Media / Object Storage
* **Vấn đề:** Messaging cần gửi ảnh/file, Clans cần avatar/banner.
* **Đề xuất:**
  * `plugin-storage`: Tích hợp AWS S3, Cloudflare R2, MinIO.
  * Pre-signed URL upload (client → S3 trực tiếp, giảm tải server).
  * Image resize/thumbnail on-upload, virus scan hook.
  * Quota per user/clan, content-type validation.

### 6. Push Notifications
* **Vấn đề:** Offline Queue lưu tin nhắn nhưng user không biết có tin mới khi tắt app.
* **Đề xuất:**
  * Tích hợp **FCM** (Android/Web) và **APNs** (iOS).
  * Device token registry per user, badge count management.
  * Notification preferences (mute channel, quiet hours).

### 7. Message Queue / Background Jobs
* **Vấn đề:** Một số tác vụ (gửi email, generate report, cleanup data) không nên chạy đồng bộ.
* **Đề xuất:**
  * Tích hợp background job queue (sử dụng Redis Streams hoặc dedicated queue).
  * Retry logic với exponential backoff, dead-letter queue.
  * Job scheduling (cron-like) cho recurring tasks (daily cleanup, weekly reports).

### 8. Secrets Management
* **Vấn đề:** Config hiện tại dùng env var (`STRIPE_SECRET_KEY`, `DRAOX_JWT_SECRET`). Ở quy mô lớn cần quản lý tập trung.
* **Đề xuất:**
  * Tích hợp **HashiCorp Vault**, AWS Secrets Manager, hoặc Azure Key Vault.
  * Auto-rotate secrets (DB passwords, API keys) không cần restart.
  * Encrypt sensitive config at rest.

---

## Nhóm C — Bảo mật Nâng cao (Security)

### 9. Application-Level Rate Limiting
* **Vấn đề:** `traffic-guard` chống DDoS ở tầng IP. User xác thực dùng proxy vẫn spam được.
* **Đề xuất:**
  * Rate limit theo `user_id` / `session_id` (VD: 5 msg/sec, 1 clan/ngày).
  * Sliding window counter lưu trên Redis, configurable per-action.

### 10. End-to-End Encryption (E2EE) cho Messaging
* **Vấn đề:** Tin nhắn hiện tại server đọc được toàn bộ nội dung.
* **Đề xuất:**
  * Hỗ trợ E2EE cho Direct Messages (Signal Protocol / Double Ratchet).
  * Server chỉ lưu trữ ciphertext, key exchange qua Diffie-Hellman.
  * Opt-in per conversation, key backup/recovery.

### 11. Content Moderation
* **Vấn đề:** `plugin-messaging` có word filter cơ bản, thiếu moderation nâng cao.
* **Đề xuất:**
  * AI-powered content filtering (toxicity detection, spam classification).
  * Image/media scanning (NSFW detection) trước khi deliver.
  * Report queue cho moderators, auto-action rules (warn → mute → ban).
  * Audit trail cho mọi moderation action.

### 12. GDPR & Data Privacy Compliance
* **Vấn đề:** Thiếu cơ chế tuân thủ quy định bảo vệ dữ liệu.
* **Đề xuất:**
  * **Right to be forgotten**: API xoá toàn bộ dữ liệu user (messages, clan membership, logs).
  * Data export (GDPR Article 20): Export toàn bộ dữ liệu user ra JSON/ZIP.
  * Consent management, data retention policies tự động.
  * PII anonymization trong logs.

---

## Nhóm D — Khả năng Mở rộng (Extensibility)

### 13. Outbound Webhooks
* **Vấn đề:** Event Bus chỉ hoạt động nội bộ. Bên thứ 3 phải polling API.
* **Đề xuất:**
  * Admin đăng ký webhook URLs cho các event (`clan.created`, `payment.success`).
  * Retry với exponential backoff, HMAC signature verification.
  * Webhook delivery logs, manual re-trigger từ Admin API.

### 14. GraphQL API (Bổ sung cho REST)
* **Vấn đề:** ~72 REST endpoints + plugin routes → Client phải gọi nhiều request để lấy dữ liệu liên quan.
* **Đề xuất:**
  * GraphQL layer bọc trên các service hiện có (clans + members + channels trong 1 query).
  * Subscriptions qua WebSocket cho real-time data.
  * DataLoader pattern để tránh N+1 queries.

### 15. Client SDK Auto-generation
* **Vấn đề:** Hiện có OpenAPI spec (utoipa), nhưng developer vẫn phải tự viết client code.
* **Đề xuất:**
  * Auto-generate SDK từ OpenAPI: TypeScript, Dart (Flutter), Swift, Kotlin, C#.
  * Publish lên npm, pub.dev, CocoaPods, Maven.
  * Socket protocol SDK với reconnect logic, state sync built-in.

### 16. Plugin Inter-Communication (IPC)
* **Vấn đề:** Plugins giao tiếp qua Event Bus chung, thiếu cơ chế gọi trực tiếp.
* **Đề xuất:**
  * Plugin-to-plugin RPC: `plugin-messaging` gọi trực tiếp `plugin-clans.get_members()`.
  * Typed service contracts trong `plugin-sdk`.
  * Dependency injection tự động dựa trên manifest `[dependencies]`.

---

## Nhóm E — Vận hành & Quan sát (Operations)

### 17. QUIC / HTTP/3 Support
* **Vấn đề:** Chỉ hỗ trợ TCP + HTTP/1.1 + HTTP/2. Mobile users trên mạng không ổn định bị reconnect thường xuyên.
* **Đề xuất:**
  * Thêm QUIC transport (via `quinn` crate) — 0-RTT handshake, connection migration khi đổi mạng WiFi ↔ 4G.
  * HTTP/3 cho Admin API.

### 18. Distributed Tracing (OpenTelemetry)
* **Vấn đề:** `activity-log` có metrics nhưng thiếu distributed tracing xuyên suốt request lifecycle.
* **Đề xuất:**
  * Tích hợp **OpenTelemetry** với trace propagation qua tất cả layers.
  * Export traces sang Jaeger, Zipkin, hoặc Grafana Tempo.
  * Correlation ID xuyên suốt: socket-server → traffic-guard → connection-manager → plugin.

### 19. Feature Flags
* **Vấn đề:** Muốn rollout tính năng mới từ từ (canary, A/B test) phải deploy lại server.
* **Đề xuất:**
  * Built-in feature flag system hoặc tích hợp LaunchDarkly / Unleash.
  * Toggle features per user/clan/plan tier tại runtime.
  * Lưu trữ flags trong `server-config` với hot-reload.

### 20. Container & Orchestration
* **Vấn đề:** Chưa có hỗ trợ triển khai chính thức.
* **Đề xuất:**
  * Multi-stage Dockerfile (build + runtime image tối ưu size).
  * Docker Compose cho dev environment (Draox + PostgreSQL + Redis + MongoDB).
  * Kubernetes Helm chart, health/readiness probes mapping tới `/api/v1/app/health`.
  * Horizontal Pod Autoscaler config dựa trên connection count.

### 21. Hỗ trợ Đa ngôn ngữ (i18n)
* **Vấn đề:** System Messages và Error responses chưa có cơ chế đa ngôn ngữ.
* **Đề xuất:**
  * Client gửi `Accept-Language` header hoặc khai báo khi handshake.
  * Message template system cho system notifications.
  * Plugin SDK hỗ trợ i18n cho plugin-contributed messages.

### 22. Backup & Disaster Recovery
* **Vấn đề:** Thiếu chiến lược backup/restore chính thức.
* **Đề xuất:**
  * Scheduled database backup (pg_dump / mongodump) tự động.
  * Point-in-time recovery configuration guide.
  * Config/plugin state export/import cho migration giữa các environments.

---

## Ma trận Ưu tiên

| # | Tính năng | Ưu tiên | Lý do |
|---|-----------|---------|-------|
| 1 | Identity & Auth | 🔴 Critical | Không có thì end-user không đăng nhập được |
| 2 | Clustering & HA | 🔴 Critical | Single-node = single point of failure |
| 3 | Presence System | 🟡 High | Cần cho UX realtime (ai đang online) |
| 4 | Friends & Social | 🟡 High | Bổ sung cho Clans, cần cho DM |
| 5 | Media Storage | 🟡 High | Messaging cần gửi file/ảnh |
| 6 | Push Notifications | 🟡 High | Mobile UX bắt buộc |
| 7 | Background Jobs | 🟢 Medium | Cải thiện hiệu năng |
| 8 | Secrets Management | 🟢 Medium | Production hardening |
| 9 | App-Level Rate Limit | 🟡 High | Chống abuse từ authenticated users |
| 10 | E2EE Messaging | 🟢 Medium | Privacy-focused deployments |
| 11 | Content Moderation | 🟡 High | Trust & Safety bắt buộc |
| 12 | GDPR Compliance | 🟡 High | Legal requirement (EU) |
| 13 | Outbound Webhooks | 🟢 Medium | Integration với hệ thống bên ngoài |
| 14 | GraphQL API | 🔵 Low | Nice-to-have, REST đủ dùng |
| 15 | Client SDK Gen | 🟢 Medium | Developer experience |
| 16 | Plugin IPC | 🟢 Medium | Plugin ecosystem maturity |
| 17 | QUIC/HTTP3 | 🔵 Low | Future-proofing |
| 18 | OpenTelemetry | 🟢 Medium | Production observability |
| 19 | Feature Flags | 🔵 Low | Gradual rollout |
| 20 | Container/K8s | 🟡 High | Deployment readiness |
| 21 | i18n | 🔵 Low | Global reach |
| 22 | Backup/DR | 🟡 High | Data safety |

---

## Đề xuất Roadmap

### Phase A — Must-have (trước Production)
> Identity, Clustering, Presence, App-Level Rate Limiting, Container/K8s, Backup/DR

### Phase B — Growth (sau Production v1)
> Friends, Media Storage, Push Notifications, Content Moderation, GDPR, Webhooks

### Phase C — Maturity
> Background Jobs, Secrets Mgmt, E2EE, Client SDK, Plugin IPC, OpenTelemetry

### Phase D — Innovation
> GraphQL, QUIC/HTTP3, Feature Flags, i18n
