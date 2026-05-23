# Document/Code Change History

## 2026-05-03

### Phase 19 — gRPC + Protobuf Transport (implement + build fix)
- `backend/proto/draox.proto` — Created (3 services, 15 message types)
- `backend/crates/grpc-api/Cargo.toml` — Created (+ protox build-dep, tokio-stream sync feature)
- `backend/crates/grpc-api/build.rs` — Created (protox + tonic_build::compile_fds, no protoc needed)
- `backend/crates/grpc-api/src/lib.rs` — Created
- `backend/crates/grpc-api/src/state.rs` — Created (GrpcState)
- `backend/crates/grpc-api/src/server.rs` — Created (GrpcServer::start)
- `backend/crates/grpc-api/src/interceptor.rs` — Created (auth interceptor)
- `backend/crates/grpc-api/src/service/mod.rs` — Created
- `backend/crates/grpc-api/src/service/auth.rs` — Created (AuthServiceImpl)
- `backend/crates/grpc-api/src/service/draox.rs` — Created (DraoxServiceImpl)
- `backend/crates/grpc-api/src/service/messaging.rs` — Created (MessagingServiceImpl)
- `backend/Cargo.toml` — Modified (tonic, prost, grpc-api member)
- `backend/crates/server-config/src/model.rs` — Modified (GrpcConfig struct + DraoxConfig.grpc)
- `backend/config/default.toml` — Modified ([grpc] section)
- `backend/crates/draox-server/src/main.rs` — Modified (GrpcServer wire-up + session_manager_for_grpc clone fix)
- `backend/crates/draox-server/Cargo.toml` — Modified (grpc-api dep)
- `backend/tools/sdk-ts/draox-client/src/transports/GrpcTransport.ts` — Created
- `backend/tools/sdk-ts/draox-client/src/types.ts` — Modified ('grpc' protocol + GrpcConfig)
- `backend/tools/sdk-ts/draox-client/src/DraoxClient.ts` — Modified (grpc default port)
- `backend/tools/sdk-ts/draox-client/src/index.ts` — Modified (GrpcTransport export)
- `backend/tools/sdk-ts/draox-client/package.json` — Modified (@grpc/grpc-js + proto-loader)
- `docs/grpc_plan.md` — Created (technical design report)
- `docs/plan.md` — Updated (Phase 19 entry)

### SDK Report — Tổng hợp tài liệu tất cả SDK
- `backend/tools/SDK_REPORT.md` — Created (comprehensive SDK documentation report, 9 sections)

### SDK — C# WPF (DraoxClientWpf + DraoxWpfDemo)
- `backend/tools/sdk-wpf/DraoxClientWpf/DraoxClientWpf.csproj` — Created
- `backend/tools/sdk-wpf/DraoxClientWpf/Core/DraoxConfig.cs` — Created (enums, DTOs, wire types)
- `backend/tools/sdk-wpf/DraoxClientWpf/Core/IConnection.cs` — Created
- `backend/tools/sdk-wpf/DraoxClientWpf/Core/Serializer.cs` — Created (System.Text.Json)
- `backend/tools/sdk-wpf/DraoxClientWpf/Core/RequestBroker.cs` — Created
- `backend/tools/sdk-wpf/DraoxClientWpf/Core/Reconnector.cs` — Created
- `backend/tools/sdk-wpf/DraoxClientWpf/Core/WebSocketConnection.cs` — Created (System.Net.WebSockets)
- `backend/tools/sdk-wpf/DraoxClientWpf/Core/TcpConnection.cs` — Created
- `backend/tools/sdk-wpf/DraoxClientWpf/Core/DraoxClient.cs` — Created (Task-based, SynchronizationContext)
- `backend/tools/sdk-wpf/DraoxClientWpf/Plugins/MessagingPlugin.cs` — Created (full messaging API + DTOs)
- `backend/tools/sdk-wpf/DraoxWpfDemo/DraoxWpfDemo.csproj` — Created (net8.0-windows WPF)
- `backend/tools/sdk-wpf/DraoxWpfDemo/App.xaml` / `App.xaml.cs` — Created
- `backend/tools/sdk-wpf/DraoxWpfDemo/MainWindow.xaml` — Created (dark chat UI)
- `backend/tools/sdk-wpf/DraoxWpfDemo/MainWindow.xaml.cs` — Created (connect/auth/send/receive logic)
- `backend/tools/sdk-wpf/README.md` — Created

### SDK — TypeScript (draox-client + draox-ts-demo)
- `backend/tools/sdk-ts/draox-client/package.json` — Created
- `backend/tools/sdk-ts/draox-client/tsconfig.json` — Created
- `backend/tools/sdk-ts/draox-client/src/types.ts` — Created
- `backend/tools/sdk-ts/draox-client/src/Serializer.ts` — Created
- `backend/tools/sdk-ts/draox-client/src/RequestBroker.ts` — Created
- `backend/tools/sdk-ts/draox-client/src/Reconnector.ts` — Created
- `backend/tools/sdk-ts/draox-client/src/transports/ITransport.ts` — Created
- `backend/tools/sdk-ts/draox-client/src/transports/WebSocketTransport.ts` — Created (ws package)
- `backend/tools/sdk-ts/draox-client/src/DraoxClient.ts` — Created (EventEmitter, async/await)
- `backend/tools/sdk-ts/draox-client/src/plugins/MessagingPlugin.ts` — Created
- `backend/tools/sdk-ts/draox-client/src/index.ts` — Created
- `backend/tools/sdk-ts/draox-ts-demo/package.json` — Created
- `backend/tools/sdk-ts/draox-ts-demo/tsconfig.json` — Created
- `backend/tools/sdk-ts/draox-ts-demo/src/index.ts` — Created (CLI chat with ANSI colours)
- `backend/tools/sdk-ts/draox-ts-demo/README.md` — Created
- `backend/tools/sdk-ts/README.md` — Created

### Docs updated
- `docs/chat.md` — Added 2026-05-03 entry
- `docs/history.md` — Added 2026-05-03 entries

---

## 2026-04-29

### DraoxDemo Unity App — Complete Project
- `backend/tools/sdk-unity/DraoxDemo/Assets/Scripts/EventLog.cs` — Created (singleton log panel)
- `backend/tools/sdk-unity/DraoxDemo/Assets/Scripts/DemoManager.cs` — Created (root controller, tab navigation)
- `backend/tools/sdk-unity/DraoxDemo/Assets/Scripts/ConnectionPanel.cs` — Created (host/port/protocol UI)
- `backend/tools/sdk-unity/DraoxDemo/Assets/Scripts/AuthPanel.cs` — Created (authenticate + addConnection)
- `backend/tools/sdk-unity/DraoxDemo/Assets/Scripts/RequestPanel.cs` — Created (raw send/request/ping/subscribe)
- `backend/tools/sdk-unity/DraoxDemo/Assets/Scripts/ClansPanel.cs` — Created (ClansPlugin full coverage)
- `backend/tools/sdk-unity/DraoxDemo/Assets/Scripts/MessagingPanel.cs` — Created (MessagingPlugin full coverage)
- `backend/tools/sdk-unity/DraoxDemo/Assets/Scripts/PresencePanel.cs` — Created (PresencePlugin full coverage)
- `backend/tools/sdk-unity/DraoxDemo/Assets/Scripts/*.cs.meta` — Created 8 meta files with stable GUIDs
- `backend/tools/sdk-unity/DraoxDemo/Assets/Scripts/DraoxDemo.asmdef` — Created
- `backend/tools/sdk-unity/DraoxDemo/Assets/Scripts/Editor/DemoSceneBuilder.cs` — Created (MenuItem to build scene via Editor API)
- `backend/tools/sdk-unity/DraoxDemo/Assets/Scripts/Editor/DemoSceneBuilder.cs.meta` — Created
- `backend/tools/sdk-unity/DraoxDemo/Assets/Scenes/DemoScene.unity` — Created (YAML skeleton: 4 settings + Camera + EventSystem)
- `backend/tools/sdk-unity/DraoxDemo/Assets/Scenes/DemoScene.unity.meta` — Created
- `backend/tools/sdk-unity/DraoxDemo/README.md` — Created
- `backend/tools/sdk-unity/DraoxClientUnity/Runtime/Core/DraoxClient.cs` — Modified: added `public DraoxConfig Config => config;` property

---

## 2026-04-26

### Compile Error Fixes — 7 New Crates
- `crates/plugin-identity/Cargo.toml` — Added sha2, hex dependencies
- `crates/plugin-identity/src/mfa.rs` — Fixed totp-rs v5 `get_url()` return type (String, not Result)
- `crates/plugin-cluster/src/pubsub.rs` — RedisClient→Client, RedisConfig→Config, fixed msg.value.as_str()
- `crates/plugin-cluster/src/registry.rs` — RedisClient→Client, Config, fixed expire() 3rd arg, del/set type params
- `crates/plugin-cluster/src/leader.rs` — Client/Config, fixed eval() 4 generic params, Value return type
- `crates/plugin-cluster/src/sticky.rs` — Removed unused NodeId import
- `crates/plugin-push/Cargo.toml` — Added `http2` reqwest feature
- `crates/plugin-jobs/src/retry.rs` — Fixed u64 literal with `wrapping_mul`
- `crates/plugin-jobs/src/queue.rs` — Removed unused `Reverse` import
- `Cargo.toml` — Added `i-scripts` to fred features for LuaInterface

---

### Frontend Admin UI — Phase 4 (Advanced Pages) + Phase 5 (Polish & QA)

**Updated files — Phase 4:**
- **Updated** `frontend/src/pages/Marketplace/index.tsx` — useRequest(searchPlugins, getFeatured, getPopular), debounced search, 3-tab layout, PluginCard component, skeleton loading, empty states, RBAC publish button
- **Updated** `frontend/src/pages/Marketplace/Detail.tsx` — 4 parallel useRequest (getPlugin/getVersions/getReviews/getAnalytics), postReview form, analytics stats cards + Line chart, per-tab Skeleton/Empty
- **Updated** `frontend/src/pages/Marketplace/Publish.tsx` — publishPlugin FormData building from StepsForm, file upload tracking, RBAC 403 fallback for non-publishers
- **Updated** `frontend/src/pages/Billing/Plans.tsx` — useRequest(getPlans), assignPlan Modal+Form, dynamic plan cards, RBAC guard
- **Updated** `frontend/src/pages/Billing/Usage.tsx` — useRequest(getUsage, manual), bandwidth Progress bar with formatBytes, RBAC guard, error Alert
- **Updated** `frontend/src/pages/Routes/index.tsx` — listRoutes (30s polling) + registerRoute + deleteRoute real API, RBAC conditional columns, empty state

**New files — Phase 5:**
- **Created** `frontend/src/components/ErrorBoundary/index.tsx` — Class ErrorBoundary with getDerivedStateFromError, componentDidCatch, Result recovery UI (Try Again + Reload)

**Updated files — Phase 5:**
- **Updated** `frontend/src/app.tsx` — import ErrorBoundary, add `childrenRender: (children) => <ErrorBoundary>{children}</ErrorBoundary>` to layout config
- **Updated** `frontend/src/global.less` — ProTable/ProCard dark overrides, Empty description color, Collapse dark, Pagination dark, Input/DatePicker dark, Form label, `@media (max-width: 768px)` responsive rules, `@keyframes pulse` for WS indicator, Skeleton gradient override
- **Updated** `frontend/config/routes.ts` — added comment about lazy loading strategy (UmiJS native code splitting)
- **Updated** `frontend/docs/frontend_implement_report.md` — added Phase 4 & 5 sections with full change log

### Frontend Admin UI — Phase 2 (HTTP Integration) + Phase 3 (WebSocket Real-time)

**New files**:
- **Created** `frontend/src/components/WsHeaderIndicator/index.tsx` — Shows live status dots for all 5 WS streams in header; keeps sockets alive via no-op subscribers + 1.5s status polling
- **Created** `docs/frontend_implement_report.md` — Full implementation report with architecture, patterns, and change log

**Updated files**:
- **Updated** `frontend/src/app.tsx` — Import and render `<WsHeaderIndicator />` in `rightContentRender()`, replacing placeholder span
- **Updated** `frontend/src/pages/Dashboard/index.tsx` — Phase 2: useRequest(getDetailedHealth + getMetrics). Phase 3: subscribe /ws/metrics → addSnapshot, /ws/events → addEvent; charts render from ring buffer
- **Updated** `frontend/src/pages/Connections/index.tsx` — Phase 2: useRequest(listConnections + getConnectionStats) + real disconnectConnection. Phase 3: /ws/connections → refresh()
- **Updated** `frontend/src/pages/Sessions/index.tsx` — Phase 2: listSessions + destroySession + drainSession real API calls, stats row added
- **Updated** `frontend/src/pages/Plugins/index.tsx` — Phase 2: listPlugins + all 5 lifecycle service calls. Phase 3: /ws/plugins → refresh()
- **Updated** `frontend/src/pages/TrafficGuard/index.tsx` — Phase 2: getGuardStats + listBans + banIp/unbanIp/whitelist/blacklist API. Phase 3: /ws/guard → refresh stats+bans. New IP Reputation tab
- **Updated** `frontend/src/pages/Config/index.tsx` — Phase 2: getConfig (dynamic sections), reloadConfig with loading state, error Alert
- **Updated** `frontend/src/pages/Cache/index.tsx` — Phase 2: getCacheStats + getCacheHealth polling, flushCache with manual loading state, formatBytes
- **Updated** `frontend/src/pages/Audit/index.tsx` — Phase 2: getAuditLogs with pagination + refreshDeps, severity filter, error Alert
- **Updated** `frontend/src/pages/Metrics/index.tsx` — Phase 3: useModel('metrics') ring buffer + /ws/metrics subscription; all 4 charts live from buffer
- **Updated** `frontend/src/pages/EventStream/index.tsx` — Phase 3: useModel('events') FIFO buffer + /ws/events subscription; pause/resume, category filters, status bar

---

## 2026-04-25

### Extended Features — Group A (1–3) + Group B: 7 new Rust crates

**New crates — Group A (Critical Missing)**:
- **Created** `crates/plugin-identity/` — Identity & Auth: Argon2id, JWT, TOTP/MFA, OAuth2 (Google/Discord/Apple), Device fingerprinting, Refresh Token rotation, Session revocation (7 src files)
- **Created** `crates/plugin-cluster/` — Clustering & HA: Redis Pub/Sub inter-node messaging, SharedSessionRegistry, LeaderElection (Lua CAS), Sticky routing (IpHash/Cookie/LeastConn) (5 src files)
- **Created** `crates/plugin-presence/` — Presence System: Online/Away/DND/Invisible/Offline/InGame/Custom statuses, broadcast channel, auto-away background task (4 src files)

**New crates — Group B (Infrastructure)**:
- **Created** `crates/plugin-storage/` — Object Storage: AWS S3/R2/MinIO via AWS SDK, presigned PUT/GET URLs, content-type validation, per-owner quota management (5 src files)
- **Created** `crates/plugin-push/` — Push Notifications: FCM v1 HTTP API, APNs HTTP/2 with JWT auth, DeviceTokenRegistry, NotificationPreferences (quiet hours, muted topics, badge count) (5 src files)
- **Created** `crates/plugin-jobs/` — Background Jobs: priority queue (BinaryHeap), WorkerPool, exponential backoff with jitter, DeadLetterQueue, cron scheduling, JobHandler trait (7 src files)
- **Created** `crates/secrets-manager/` — Secrets Management: HashiCorp Vault KV v2, AWS Secrets Manager, Azure Key Vault, AES-256-GCM encryption at rest, AutoRotator background task (7 src files)

**Updated**:
- **Updated** `Cargo.toml` — added 7 new workspace members + 14 new shared dependencies
- **Updated** `docs/plan.md` — added Phase 18 with full feature breakdown
- **Created** `implementation_report.md` — detailed implementation report with architecture notes

---

## 2026-04-17

### Frontend Scaffold — Ant Design Pro 6 (Phase 1+2: Foundation + UI Prototypes)
Framework: **Ant Design Pro 6** (React 18 + Ant Design 5 + UmiJS 4). Dark theme with orange (#e05d10) accents. 87 source files, zero TypeScript errors.

**Project Setup (6 files)**:
- **Created** `frontend/package.json` — @umijs/max, @ant-design/pro-components, @ant-design/charts, reconnecting-websocket
- **Created** `frontend/tsconfig.json` — strict mode, paths: @/* → src/*
- **Created** `frontend/.npmrc` — registry config
- **Created** `frontend/.gitignore` — node_modules, .umi, dist, .env.local
- **Created** `frontend/.env` — DRAOX_API_BASE, DRAOX_WS_URL

**Config (4 files)**:
- **Created** `frontend/config/config.ts` — UmiJS defineConfig: antd (dark algorithm, cssVar), locale, access, model, request, layout
- **Created** `frontend/config/routes.ts` — 22 routes with RBAC access flags
- **Created** `frontend/config/proxy.ts` — /api → localhost:9100, /ws → ws://localhost:9100
- **Created** `frontend/config/defaultSettings.ts` — navTheme: realDark, primaryColor: #e05d10, layout: mix

**App Core (5 files)**:
- **Created** `frontend/src/app.tsx` — getInitialState, layout config, request interceptors, JWT Bearer token
- **Created** `frontend/src/access.ts` — RBAC factory: 3 roles (admin/operator/viewer), 11 access flags
- **Created** `frontend/src/global.less` — Dark theme CSS overrides, ProLayout token customization
- **Created** `frontend/src/typings.d.ts` — Global type declarations
- **Created** `frontend/public/logo.svg` — Draox Server logo (orange hexagonal)

**Services (15 files)**:
- **Created** `frontend/src/services/typings.d.ts` — API response types: Connection, Session, Plugin, Ban, AuditEntry, Metric, MarketplacePlugin, etc.
- **Created** `frontend/src/services/auth.ts` — login, getCurrentUser
- **Created** `frontend/src/services/connections.ts` — list, getById, disconnect
- **Created** `frontend/src/services/sessions.ts` — list, getById, destroy, drain
- **Created** `frontend/src/services/plugins.ts` — list, getById, activate, deactivate, enable, disable, restart
- **Created** `frontend/src/services/trafficGuard.ts` — ban, unban, whitelist, blacklist, getBans, getStats, getReputation
- **Created** `frontend/src/services/config.ts` — getConfig, reloadConfig
- **Created** `frontend/src/services/billing.ts` — getPlans, getUsage, updatePlan
- **Created** `frontend/src/services/cache.ts` — getStats, flush, getHealth
- **Created** `frontend/src/services/audit.ts` — list, getById
- **Created** `frontend/src/services/metrics.ts` — getSnapshot, getTimeSeries, getTopEndpoints
- **Created** `frontend/src/services/marketplace.ts` — browse, getDetail, install, publish, getReviews
- **Created** `frontend/src/services/routes.ts` — list, register, delete
- **Created** `frontend/src/services/health.ts` — check
- **Created** `frontend/src/services/wsManager.ts` — WebSocket manager: 5 streams (metrics, events, connections, sessions, plugins), auto-reconnect

**Models (3 files)**:
- **Created** `frontend/src/models/auth.ts` — Auth state model (currentUser, token, role)
- **Created** `frontend/src/models/metrics.ts` — Metrics ring buffer model
- **Created** `frontend/src/models/events.ts` — Events FIFO queue model

**Utils (2 files)**:
- **Created** `frontend/src/utils/constants.ts` — API endpoints, WS streams, protocol colors, status colors
- **Created** `frontend/src/utils/formatters.ts` — formatBytes, formatDuration, formatTimestamp, formatIP

**Mock Data (10 files)**:
- **Created** `frontend/mock/auth.ts` — Login + getCurrentUser mock (admin role)
- **Created** `frontend/mock/connections.ts` — 8 mock connections (TCP/UDP/WS/HTTP)
- **Created** `frontend/mock/sessions.ts` — 5 mock sessions
- **Created** `frontend/mock/plugins.ts` — 4 mock plugins (2 built-in + 2 WASM)
- **Created** `frontend/mock/guard.ts` — Bans, whitelist, blacklist, reputation mock data
- **Created** `frontend/mock/metrics.ts` — 60 data points for time series
- **Created** `frontend/mock/audit.ts` — 10 audit log entries
- **Created** `frontend/mock/marketplace.ts` — 6 marketplace plugins
- **Created** `frontend/mock/billing.ts` — 3 billing plans (Free/Pro/Enterprise)
- **Created** `frontend/mock/routes.ts` — 5 registered routes

**Components (11 files)**:
- **Created** `frontend/src/components/DarkStatisticCard/index.tsx` — Dark-themed stat card with icon and trend
- **Created** `frontend/src/components/RealTimeMetricsCard/index.tsx` — Auto-refreshing stat card via polling
- **Created** `frontend/src/components/ConnectionTable/index.tsx` — ProTable for connection data with actions
- **Created** `frontend/src/components/PluginStatusBadge/index.tsx` — Color-coded plugin lifecycle badge
- **Created** `frontend/src/components/IPReputationGauge/index.tsx` — Gauge chart (0-100) with color zones
- **Created** `frontend/src/components/EventTimeline/index.tsx` — Server event timeline with auto-scroll and filtering
- **Created** `frontend/src/components/HealthStatusBar/index.tsx` — Service health status dots row
- **Created** `frontend/src/components/BandwidthChart/index.tsx` — Area chart with gradient fill for bandwidth
- **Created** `frontend/src/components/WebSocketIndicator/index.tsx` — WS connection state badge
- **Created** `frontend/src/components/ConfirmActionModal/index.tsx` — Dangerous action confirmation modal
- **Created** `frontend/src/components/SearchableIPTable/index.tsx` — IP table with search for whitelist/blacklist

**Pages (22 files)**:
- **Created** `frontend/src/pages/Login/index.tsx` — ProForm login, JWT flow, dark centered card
- **Created** `frontend/src/pages/Dashboard/index.tsx` — 4 stat cards, BandwidthChart, Pie, Line, EventTimeline
- **Created** `frontend/src/pages/Connections/index.tsx` — ProTable (8 rows), stat cards, disconnect action
- **Created** `frontend/src/pages/Connections/Detail.tsx` — ProDescriptions, danger zone disconnect
- **Created** `frontend/src/pages/Sessions/index.tsx` — ProTable, destroy/drain actions
- **Created** `frontend/src/pages/Sessions/Detail.tsx` — ProDescriptions, 3 stat cards, lifecycle actions
- **Created** `frontend/src/pages/Plugins/index.tsx` — ProTable, PluginStatusBadge, lifecycle dropdown
- **Created** `frontend/src/pages/Plugins/Detail.tsx` — ProDescriptions, health card, action buttons
- **Created** `frontend/src/pages/TrafficGuard/index.tsx` — 3 tabs (Overview/Bans/IP Lists), ban form, IP tables
- **Created** `frontend/src/pages/TrafficGuard/Reputation.tsx` — IP search, IPReputationGauge, risk label
- **Created** `frontend/src/pages/Config/index.tsx` — Collapsible JSON tree, reload button (admin only)
- **Created** `frontend/src/pages/Billing/Plans.tsx` — 3 ProCards (Free/Pro/Enterprise)
- **Created** `frontend/src/pages/Billing/Usage.tsx` — Client search, ProDescriptions, stat cards
- **Created** `frontend/src/pages/Cache/index.tsx` — 3 stat cards, health badge, flush button
- **Created** `frontend/src/pages/Audit/index.tsx` — ProTable (10 rows), severity Tags, date filter
- **Created** `frontend/src/pages/Audit/Detail.tsx` — ProDescriptions, JSON payload viewer
- **Created** `frontend/src/pages/Metrics/index.tsx` — 6 stat cards, 4 charts (Line/Area/Column/Line)
- **Created** `frontend/src/pages/Marketplace/index.tsx` — Search bar, ProCard grid (6 mock plugins)
- **Created** `frontend/src/pages/Marketplace/Detail.tsx` — Plugin header, tabs (Overview/Versions/Reviews/Analytics)
- **Created** `frontend/src/pages/Marketplace/Publish.tsx` — StepsForm (4 steps), file upload
- **Created** `frontend/src/pages/Routes/index.tsx` — ProTable, register route modal, delete confirm
- **Created** `frontend/src/pages/EventStream/index.tsx` — Full-page EventTimeline, filters, pause/clear

**i18n (10 files)**:
- **Created** `frontend/src/locales/en-US.ts` — Barrel export aggregating all en-US namespaces
- **Created** `frontend/src/locales/en-US/menu.ts` — Menu item translations (22 items)
- **Created** `frontend/src/locales/en-US/pages.ts` — Page-specific translations
- **Created** `frontend/src/locales/en-US/component.ts` — Component label translations
- **Created** `frontend/src/locales/en-US/global.ts` — Global UI translations
- **Created** `frontend/src/locales/vi-VN.ts` — Barrel export aggregating all vi-VN namespaces
- **Created** `frontend/src/locales/vi-VN/menu.ts` — Bản dịch mục menu (22 mục)
- **Created** `frontend/src/locales/vi-VN/pages.ts` — Bản dịch theo trang
- **Created** `frontend/src/locales/vi-VN/component.ts` — Bản dịch nhãn thành phần
- **Created** `frontend/src/locales/vi-VN/global.ts` — Bản dịch giao diện chung

**Reports (2 files)**:
- **Created** `docs/scaffold_report_en.html` — English scaffold report (13 sections, dark theme CSS)
- **Created** `docs/scaffold_report_vi.html` — Vietnamese scaffold report (13 mục, tiếng Việt có dấu)

**Docs**:
- **Updated** `docs/plan.md` — Added Phase 17: Frontend Scaffold (13 items, all checked)
- **Updated** `docs/chat.md` — Added scaffold session entry
- **Updated** `docs/history.md` — This entry

## 2026-04-16

### Backend UI Admin Dashboard Design
- **Created** `docs/design_backend_ui_en.html` — 1,915-line English HTML design report. Ant Design Pro 6 + dark theme. 19 sections: Overview, Architecture, Theme tokens, Page Map (22 routes), Dashboard, Connections & Sessions, Plugins, Traffic Guard, Metrics, Marketplace, Other Pages, Event Stream, WebSocket (5 streams), Service Layer (13 files), Auth & RBAC (3 roles), Component Library (11 components), i18n, Implementation Phases (5), Summary.
- **Created** `docs/design_backend_ui_vi.html` — 2,179-line Vietnamese HTML design report. Same structure and CSS as English version, all text in Vietnamese with proper diacritical marks.
- **Updated** `docs/plan.md` — Added Phase 16: Backend UI Admin Dashboard Design (20 items, all checked)

### MongoDB Storage Backend
NoSQL storage backend via MongoDB (mongodb crate v3). Stores JSON values as native BSON documents — no stringification. Compound unique index on `(namespace, key)` replaces SQL PRIMARY KEY. Regex-based prefix matching for `list_keys`.

**data-store** (8 new ignored tests, 24 active + 24 ignored):
- **Created** `crates/data-store/src/mongodb.rs` — Full `MongoStorage` implementation: `mongodb::Client` with configurable pool (`max_pool_size`), native BSON storage (`serde_json::Value` ↔ `Bson` conversion), upsert via `update_one` with `upsert(true)`, `$regex`-based key prefix listing, idempotent unique compound index on `(namespace, key)`. 8 tests (`#[ignore]`, env `MONGO_TEST_URL`)
- **Enhanced** `crates/data-store/src/error.rs` — Added `into_mongo_error()` for `mongodb::error::Error` → `server_core::Error::Storage` conversion
- **Enhanced** `crates/data-store/src/lib.rs` — Added `pub mod mongodb`, `pub use self::mongodb::MongoStorage`, factory function extended with `"mongodb"` → `MongoStorage::new(&config.mongodb)` branch
- **Enhanced** `crates/data-store/Cargo.toml` — Added `mongodb.workspace = true`, `futures-util.workspace = true`

**config/server**:
- **Enhanced** `config/default.toml` — Updated backend options comment to include `"mongodb"`
- **Enhanced** `crates/draox-server/src/main.rs` — Updated storage backend comment to include MongoDB

**docs**:
- **Updated** `docs/plan.md` — MongoDB items checked off, test counts updated (598 + 28 ignored)
- **Updated** `docs/design_en.html` — MongoDB implementation status, test counts, summary table
- **Updated** `docs/design_vi.html` — MongoDB implementation status, test counts, summary table

## 2026-04-15

### PostgreSQL & MySQL/MariaDB Storage Backends
Config-driven storage backend switching: users can switch between SQLite, PostgreSQL, and MySQL/MariaDB via `config/default.toml` (`storage.backend` + `storage.sql.url`). All backends implement the same `StorageBackend` trait with consistent behavior.

**data-store** (16 new ignored tests, 24→24 active + 16 ignored):
- **Created** `crates/data-store/src/postgres.rs` — Full `PostgresStorage` implementation: `PgPool` with configurable pool options, PostgreSQL-native bind params (`$1, $2`), upsert via `ON CONFLICT DO UPDATE SET ... EXCLUDED`, auto-migration. 8 tests (`#[ignore]`, env `POSTGRES_TEST_URL`)
- **Created** `crates/data-store/src/mysql.rs` — Full `MySqlStorage` implementation: `MySqlPool` with configurable pool options, MySQL bind params (`?`), upsert via `ON DUPLICATE KEY UPDATE`, backtick-quoted `key` column (MySQL reserved word), `VARCHAR(255)` + `LONGTEXT` DDL. 8 tests (`#[ignore]`, env `MYSQL_TEST_URL`)
- **Enhanced** `crates/data-store/src/sqlite.rs` — Added `from_config(&SqlConfig)` constructor with full pool options (max/min connections, idle timeout, max lifetime)
- **Enhanced** `crates/data-store/src/lib.rs` — Added `pub mod postgres`, `pub mod mysql`, re-exports, factory function `create_storage_backend(&StorageConfig)` returning `Result<Arc<dyn StorageBackend>>` with match on backend string

**admin-api** (storage field in AppState):
- **Enhanced** `crates/admin-api/src/state.rs` — Added `storage: Arc<dyn StorageBackend>` field
- **Enhanced** `crates/admin-api/Cargo.toml` — Added `data-store.workspace = true`

**draox-server** (main.rs):
- **Enhanced** `crates/draox-server/src/main.rs` — Added `create_storage_backend()` call after cache init, passes `storage` to `AppState`

**config**:
- **Enhanced** `config/default.toml` — Added backend option comments (`sqlite`, `postgres`, `mysql`/`mariadb`), PostgreSQL/MySQL URL examples

**Test fixes** (3 test helpers updated):
- **Enhanced** `crates/admin-api/src/lib.rs` — `make_state()` → `async fn`, added `SqliteStorage::new_in_memory()` + `storage` field
- **Enhanced** `crates/admin-api/src/routes/marketplace.rs` — Same async `make_state()` update
- **Enhanced** `crates/admin-api/src/routes/dynamic_routes.rs` — Same async `make_state()` update

### Switchable Cache Backend — Memory ↔ Redis
Config-driven cache backend switching: users can switch between in-memory (moka) and Redis (fred v10) cache via `config/default.toml` without recompiling. Auto-fallback from Redis to Memory on connection failure.

**cache-layer** (7 new tests, 23→30; +4 ignored Redis tests):
- **Enhanced** `crates/cache-layer/src/backend.rs` — Extended `CacheBackend` trait with 4 optional methods: `backend_name()`, `entry_count_async()`, `health_check()`, `flush()` (all with default implementations)
- **Enhanced** `crates/cache-layer/src/memory.rs` — Implemented `backend_name` ("memory"), `entry_count_async`, `health_check`, `flush` (invalidate_all + run_pending_tasks) for MemoryCache
- **Created** `crates/cache-layer/src/redis.rs` — Full `RedisCache` implementation: `fred::clients::Pool` with configurable pool_size, `connect()` with init+ping, per-key TTL via `SET ... EX`, `flush` via `flushall`, `entry_count_async` via `dbsize`, `health_check` via `ping`. 5 tests (1 active, 4 `#[ignore]` requiring Redis)
- **Enhanced** `crates/cache-layer/src/lib.rs` — Added `pub mod redis`, `pub use RedisCache`, factory function `create_cache_backend(&CacheConfig)` returning `(Arc<dyn CacheBackend>, &str)` with Redis→Memory fallback. 6 new tests
- **Enhanced** `crates/cache-layer/Cargo.toml` — Added `fred.workspace = true`

**admin-api** (3 new cache admin endpoints):
- **Created** `crates/admin-api/src/routes/cache.rs` — `GET /api/cache/stats` (backend name, entry count), `GET /api/cache/health` (ping latency), `POST /api/cache/flush` (clear all entries)
- **Enhanced** `crates/admin-api/src/routes/mod.rs` — Registered cache routes (`pub mod cache`, 3 routes)
- **Enhanced** `crates/admin-api/src/state.rs` — Added `cache: Arc<dyn CacheBackend>` field to `AppState`
- **Enhanced** `crates/admin-api/Cargo.toml` — Added `cache-layer.workspace = true`

**plugin-host** (BackendCacheHandle replaces InMemoryCacheHandle):
- **Enhanced** `crates/plugin-host/src/handles.rs` — Replaced `InMemoryCacheHandle` (DashMap) with `BackendCacheHandle` wrapping `Arc<dyn CacheBackend>`, namespace prefix `plugin:{id}:{key}`
- **Enhanced** `crates/plugin-host/src/context_builder.rs` — `ContextBuilder::new()` now accepts 3 args (added `cache_backend: Arc<dyn CacheBackend>`), builds `BackendCacheHandle` instead of `InMemoryCacheHandle`
- **Enhanced** `crates/plugin-host/Cargo.toml` — Added `cache-layer.workspace = true`

**draox-server** (main.rs):
- **Enhanced** `crates/draox-server/src/main.rs` — Added `create_cache_backend()` call after usage_tracker init, passes `cache` to ContextBuilder (3 args) and AppState

**Test fixes** (6 files):
- **Enhanced** `crates/admin-api/src/lib.rs` — Updated `make_state()` with MemoryCache + cache field
- **Enhanced** `crates/admin-api/src/routes/dynamic_routes.rs` — Updated `make_state()` with cache
- **Enhanced** `crates/admin-api/src/routes/marketplace.rs` — Updated `make_state()` with cache
- **Enhanced** `crates/plugin-host/src/registry.rs` — Updated `make_registry()` with cache backend
- **Enhanced** `crates/plugin-clans/src/plugin.rs` — Updated `make_context()` with cache backend
- **Enhanced** `crates/plugin-messaging/src/plugin.rs` — Updated `make_context()` with cache backend
- **Enhanced** `crates/plugin-clans/Cargo.toml` — Added `cache-layer`, `server-config` to dev-deps
- **Enhanced** `crates/plugin-messaging/Cargo.toml` — Added `cache-layer`, `server-config` to dev-deps

**Test results**: 598 tests passed, 0 failures, 9 ignored (4 Redis tests + 5 doc tests), 0 warnings.

## 2026-04-14

### Windows MSI Installer (cargo-wix)
- **Created** `deploy/windows/wix/main.wxs` — WiX v3 XML installer definition: 3 selectable features (Core, Windows Service, Firewall Rules), MajorUpgrade auto-removal, WixUI_FeatureTree dialog, firewall extensions (WixFirewallExtension), service config with failure recovery (WixUtilExtension)
- **Created** `deploy/windows/config/default.toml` — Windows-specific configuration with ProgramData paths (`C:\ProgramData\DraoxServer\` for data, logs, plugins, certs)
- **Created** `deploy/windows/scripts/install-service.ps1` — PowerShell service installer: auto-detect binary, register via `New-Service`, automatic startup, failure recovery (5s/30s/60s), environment variables via registry, creates ProgramData directories
- **Created** `deploy/windows/scripts/uninstall-service.ps1` — PowerShell service uninstaller: stop + remove service, remove firewall rules, `-Purge` flag with `-KeepConfig`/`-KeepData`/`-KeepLogs` selective cleanup
- **Created** `deploy/windows/scripts/manage-firewall.ps1` — PowerShell firewall manager: Add/Remove rules for TCP 9000, UDP 9001, TCP 9002, TCP 9003, TCP 9090, TCP 9100 (localhost default, `-AdminRemoteAccess` flag)
- **Created** `deploy/windows/wix/license.rtf` — MIT license in RTF format for MSI installer dialog
- **Created** `deploy/windows/README.md` — Build prerequisites (WiX Toolset, cargo-wix), MSI build commands, silent install, manual install alternative, directory structure, service management commands
- **Enhanced** `crates/draox-server/Cargo.toml` — Added `[package.metadata.wix]` section (upgrade-guid, path-guid) alongside existing `[package.metadata.deb]`
- **Enhanced** `docs/plan.md` — Added Phase 15: Deployment & Packaging with Linux + Docker + Windows checklist items
- **Enhanced** `docs/design_en.html` — Added Section 20 "Deployment & Packaging" (port table, Linux/Windows deployment details, directory layouts, build commands); updated MultiProtocolListener status to Implemented; renumbered Summary to Section 21
- **Enhanced** `docs/design_vi.html` — Added Section 20 "Triển khai & Đóng gói" (Vietnamese mirror of EN Section 20); updated MultiProtocolListener status; renumbered Tổng kết to Section 21

### Enable Multi-Protocol Listener
- **Enhanced** `crates/socket-server/src/listener.rs` — Added `MultiProtocolListener::with_tracker()` constructor to accept an externally-provided `ConnectionTracker` shared with `SessionHandler` and `AppState`
- **Enhanced** `crates/draox-server/src/main.rs` — Enabled multi-protocol listener: loads config from `--config` CLI arg (fallback to `config/default.toml`), creates shared `ConnectionTracker` from config values, starts TCP/UDP/WebSocket/HTTP listeners via `MultiProtocolListener::with_tracker()`, replaced hardcoded defaults with config-driven initialization

### Linux Deployment & Docker
- **Created** `deploy/linux/draox-server.service` — systemd unit with security hardening (NoNewPrivileges, ProtectSystem=strict, MemoryDenyWriteExecute), auto-restart, resource limits (NOFILE=65536)
- **Created** `deploy/linux/draox-server.env` — environment variables template (RUST_LOG, JWT secret, optional port/DB overrides)
- **Created** `deploy/linux/install.sh` — automated installer: create user, install binary/config, systemd service, firewall rules (ufw/firewalld), logrotate; supports `--prefix`, `--unattended`, `--no-service` options
- **Created** `deploy/linux/uninstall.sh` — clean uninstaller with `--purge`, `--keep-data`, `--keep-config` options
- **Created** `deploy/linux/logrotate.conf` — daily rotation, 14-day retention, compressed
- **Created** `deploy/linux/deb-scripts/postinst` — Debian post-install: create user, set permissions, adjust config paths
- **Created** `deploy/linux/deb-scripts/prerm` — Debian pre-remove: stop service
- **Created** `deploy/linux/deb-scripts/postrm` — Debian post-remove: purge data/user on `dpkg --purge`
- **Created** `Dockerfile` — multi-stage build (rust:1.87-bookworm → debian:bookworm-slim), dependency caching, stripped binary, non-root user, health check
- **Created** `docker-compose.yml` — draox-server service with volume mounts, resource limits (512M/2 CPU), health check, optional Redis/Prometheus/Grafana services
- **Created** `.dockerignore` — exclude target/, .vscode/, docs/, data/, .git/
- **Enhanced** `crates/draox-server/Cargo.toml` — added `[package.metadata.deb]` for cargo-deb packaging (assets, conf-files, systemd-units, maintainer-scripts)
- **Enhanced** `CLAUDE.md` — added Deployment section with Linux/Docker/Debian commands

### VS Code Workspace Configuration
- **Created** `.vscode/launch.json` — 19 debug/run configurations: Debug Server, Release Server, Debug All Tests, per-crate test debuggers (16 crates), Debug Current Test (cursor selection)
- **Created** `.vscode/settings.json` — rust-analyzer (clippy, proc-macros, inlay hints, lens), editor format-on-save, file/search exclusions (target/data/logs), terminal env vars (RUST_LOG, RUST_BACKTRACE), TOML formatter
- **Created** `.vscode/tasks.json` — 24 tasks: build (server/workspace/release), test (all + 16 per-crate), lint (clippy/fmt), run server, doc generation, clean, CI composite task
- **Created** `.vscode/extensions.json` — 12 recommended extensions (rust-analyzer, CodeLLDB, Even Better TOML, Error Lens, GitLens, dependi, etc.)

### HTML Reports — Detailed Feature Checklist (Section 19E)
- **Enhanced** `docs/design_en.html` — Added Section 19E "Detailed Feature Checklist by Phase" with 341 individual feature items across 14 phases (314✓ implemented, 27✗ not implemented), per-phase summary table, grouped by sub-module with status icons and test counts
- **Enhanced** `docs/design_vi.html` — Added Section 19E "Danh sách chi tiết tính năng theo Phase" (Vietnamese equivalent with full diacritics), added missing CSS classes (`.badge-impl`, `.badge-not-impl`, `.badge-external`, `.check-icon`, `.cross-icon`, `.phase-header`, `.impl-group-header`, `.inline-code`)

### HTML Reports — External Dependencies Detailed Notes
- **Enhanced** `docs/design_en.html` — Added Section 19D "External Dependencies — Detailed Implementation Notes" with 6 detailed cards (WASM, DB drivers, Redis, Stripe/PayPal, GeoIP, OpenAPI), summary effort table (19 items, ~10 days)
- **Enhanced** `docs/design_vi.html` — Added Section 19D "Phụ thuộc dịch vụ bên ngoài — Ghi chú triển khai chi tiết" (Vietnamese equivalent with diacritics)

### Full Feature Implementation — Optional Features + Marketplace (592 tests total)

**Mega-Batch 1: Foundation + Networking + Security** (+77 tests: 354→431)

*server-core* (5 new tests):
- **Enhanced** `types.rs` — Added `Transport`, `Handler`, `Middleware` async traits via `async-trait`

*draox-macros* (NEW crate, 6 tests):
- **Created** `crates/draox-macros/` — `#[draox_plugin]` proc-macro for plugin registration, generates factory functions

*socket-server* (30 new tests, 29→59):
- **Created** `compression.rs` — `MessageCompressor` via flate2 for per-message deflate
- **Created** `sse.rs` — `SseEvent`, `SseStream`, `SseManager` for Server-Sent Events
- **Created** `net_metrics.rs` — `NetworkMetrics` with Prometheus format export
- **Enhanced** `udp.rs` — `join_multicast`/`leave_multicast` (socket2), `UdpRateLimiter`
- **Enhanced** `ws.rs` — `SubprotocolNegotiator` for WebSocket subprotocol negotiation
- **Enhanced** `http.rs` — `KeepAliveConfig` + `apply_keep_alive_headers`

*traffic-guard* (10 new tests, 65→75):
- **Created** `syn_tracker.rs` — `SynTracker` for TCP half-open connection tracking
- **Created** `guard_metrics.rs` — `GuardMetrics` with Prometheus export

*connection-manager* (29 new tests, 22→51):
- **Created** `authority.rs` — `SessionAuthority` server-authoritative state with versioning
- **Created** `heartbeat_manager.rs` — `HeartbeatManager` per-connection heartbeat tracking
- **Created** `failover.rs` — `FailoverManager` + `FailoverPolicy` for connection failover
- **Created** `session_rate_limit.rs` — `SessionRateLimiter` per-session rate limiting
- **Created** `session_auth.rs` — `SessionAuthenticator` + `AuthInfo` auth-once-per-session
- **Created** `handoff.rs` — `HandoffManager` + `HandoffToken` for connection handoff protocol

**Mega-Batch 2: Data + Plugins** (+108 tests: 431→539)

*data-store* (14 new tests, 10→24):
- **Created** `transaction.rs` — `Transaction` + `execute_transaction` with rollback
- **Created** `routing.rs` — `ReadReplicaRouter` with round-robin selection
- **Created** `schema.rs` — 10 SQL table schemas with SQLite validation tests

*cache-layer* (16 new tests, 7→23):
- **Created** `patterns.rs` — cache-aside, read-through, write-through patterns
- **Created** `serialization.rs` — JSON/Bincode/MessagePack via `CacheSerializer` trait
- **Created** `keys.rs` — `CacheKeys` factory for 9 cache key patterns

*plugin-host* (46 new tests, 52→122 with marketplace):
- **Enhanced** `registry.rs` — `RestartPolicy` + `restart_with_policy` with cooldown/backoff
- **Created** `route_registry.rs` — `RouteRegistry` + `RouteDefinition` for dynamic routes
- **Created** `state_persistence.rs` — `StatePersistence` JSON file-based plugin state save/load
- **Created** `permissions.rs` — `PermissionEnforcer` + `PluginPermission` enum (8 variants)

*plugin-clans* (17 new tests, 40→57):
- **Created** `divisions.rs` — `DivisionManager` with full CRUD
- **Created** `channels.rs` — `ClanChannelManager` with role-gated access, auto-create defaults
- **Created** `events.rs` — `ClanEvent` enum (12 variants)
- **Created** `manifest.rs` — `clans_manifest()` for PluginManifest construction
- **Created** `api_routes.rs` — 28 REST endpoint definitions
- **Created** `db_schema.rs` — 8 SQL table schemas

*plugin-messaging* (35 new tests, 31→66):
- **Created** `receipts.rs` — `ReadReceiptTracker` with channel-level read tracking
- **Created** `files.rs` — `FileRegistry` + `FileReference`
- **Created** `delivery.rs` — `MessageDelivery` engine with online/offline routing
- **Created** `http_api.rs` — 18 REST endpoint definitions
- **Created** `events.rs` — `MessagingEvent` enum (11 variants)
- **Created** `manifest.rs` — `messaging_manifest()` for PluginManifest construction
- **Created** `db_schema.rs` — 8 SQL table schemas

**Mega-Batch 3: Marketplace + Admin API** (+53 tests: 539→592)

*plugin-host marketplace* (full implementation):
- **Created** `marketplace_types.rs` — `MarketplacePlugin`, `PublisherInfo`, `PluginCategory`, `PluginVersion`, `PluginReview`, `SearchQuery`, `SearchResult`, `PluginAnalytics`
- **Created** `marketplace_registry.rs` — `MarketplaceRegistry` (full in-memory registry with search, reviews, analytics, publishers, featured/popular lists)
- **Created** `marketplace_client.rs` — `RegistryClient` with local + remote modes
- **Created** `version_resolver.rs` — `VersionResolver` supporting ^, ~, >=, <=, >, <, =, * operators + dependency resolution
- **Created** `update_checker.rs` — `UpdateChecker` periodic update detection

*admin-api* (7 new tests, 19→26):
- **Created** `routes/marketplace.rs` — 11 marketplace endpoints (search, publish, reviews, analytics, featured, popular)
- **Created** `routes/dynamic_routes.rs` — 4 route management endpoints
- **Enhanced** `state.rs` — Added `marketplace`, `route_registry` fields to AppState
- **Enhanced** `routes/mod.rs` — Registered 15 new routes

*draox-server*:
- **Enhanced** `main.rs` — Wired `FullMarketplaceRegistry`, `RouteRegistry`, `NetworkMetrics` into startup

**Documentation Updates**:
- **Enhanced** `docs/design_en.html` — Added Section 19 "Implementation Status" with implemented/not-implemented feature tables, updated phase summary, renumbered Summary to 20
- **Enhanced** `docs/design_vi.html` — Added Section 19 "Trạng Thái Triển Khai" (Vietnamese equivalent), updated phase summary
- **Enhanced** `docs/plan.md` — Checked off ~40 newly implemented items, updated summary table to 592 tests / 16 crates

### Phase 5 Enhancements: Data Store + Cache Layer (47 new tests)

**data-store** (3 new files):
- **Created** `crates/data-store/src/transaction.rs` — `Transaction` + `TransactionOp` (Set/Delete) + `execute_transaction()`: batches (namespace, key, value) operations and rolls back applied sets on failure. 5 tests.
- **Created** `crates/data-store/src/routing.rs` — `ReadReplicaRouter`: wraps a primary `StorageBackend` + replica list; writes target primary, reads round-robin across replicas (falls back to primary when no replicas). Implements `StorageBackend` itself. 4 tests.
- **Created** `crates/data-store/src/schema.rs` — `SchemaDefinition` + `SCHEMAS` const slice with 10 table schemas (sessions, audit_logs, messages, channels, clans, clan_members, connection_history, api_keys, config_snapshots, plugin_state) + `find_schema()` helper. 5 tests (SQLite parse verification).
- **Enhanced** `crates/data-store/src/lib.rs` — Registered routing, schema, transaction modules; exported all public types.

**cache-layer** (3 new files):
- **Created** `crates/cache-layer/src/patterns.rs` — `DataLoader` + `DataWriter` traits, `cache_aside()` async fn, `ReadThroughCache`, `WriteThroughCache`. Uses `BoxFuture`. 5 tests.
- **Created** `crates/cache-layer/src/serialization.rs` — `SerializationFormat` enum, `CacheSerializer` trait, `JsonSerializer` (serde_json), `BincodeSerializer` (bincode v1), `MessagePackSerializer` (rmp-serde). 6 tests.
- **Created** `crates/cache-layer/src/keys.rs` — `CacheKeys` factory: session, plugin_state, auth_token, rate_limit, connection, health, billing_quota, clan, message_queue. 3 tests.
- **Enhanced** `crates/cache-layer/src/lib.rs` — Registered keys, patterns, serialization modules; exported all public types.

**workspace** (2 Cargo.toml updates):
- **Enhanced** `Cargo.toml` — Added `bincode = "1"`, `rmp-serde = "1"` to workspace dependencies.
- **Enhanced** `crates/cache-layer/Cargo.toml` — Added bincode.workspace, rmp-serde.workspace.

**Test results**: 47 tests (24 data-store + 23 cache-layer), 0 failures.

### Phase 3+4 Enhancements: Traffic Guard + Connection Manager (126 tests total)

**traffic-guard** (2 new files):
- **Created** `crates/traffic-guard/src/syn_tracker.rs` — `SynTracker`: TCP half-open connection tracker per IP with lazy expiry cleanup. 5 tests.
- **Created** `crates/traffic-guard/src/guard_metrics.rs` — `GuardMetrics`: lock-free `AtomicU64` counters for connections blocked/allowed/throttled, bans, reputation avg; Prometheus text format export. 4 tests.
- **Enhanced** `crates/traffic-guard/src/lib.rs` — Registered `syn_tracker`, `guard_metrics`; exported `SynTracker`, `GuardMetrics`, `GuardMetricsSnapshot`.

**connection-manager** (6 new files):
- **Created** `crates/connection-manager/src/authority.rs` — `SessionAuthority` + `AuthoritativeState`: server-owned key/value state per session, monotonic version counter via `AtomicU64`, snapshot for reconnect. 6 tests.
- **Created** `crates/connection-manager/src/heartbeat_manager.rs` — `HeartbeatManager`: connection-level ping/pong tracking (distinct from session expiry heartbeat.rs), missed-count accumulation, `connections_to_ping()`. 5 tests.
- **Created** `crates/connection-manager/src/failover.rs` — `FailoverManager` + `FailoverPolicy` (PromoteOldest/PromoteByRole/NoFailover): elects replacement primary on disconnect. 4 tests.
- **Created** `crates/connection-manager/src/session_rate_limit.rs` — `SessionRateLimiter`: fixed-window per-session rate limit with auto-reset, `with_window()` constructor. 4 tests.
- **Created** `crates/connection-manager/src/session_auth.rs` — `SessionAuthenticator` + `AuthInfo`: authenticate-once-per-session with role inheritance, `has_role()`. 5 tests.
- **Created** `crates/connection-manager/src/handoff.rs` — `HandoffManager` + `HandoffToken`: two-phase UUID-token handoff protocol with TTL expiry and replay prevention. 4 tests.
- **Enhanced** `crates/connection-manager/src/lib.rs` — Registered all 6 new modules; exported all public types.
- **Enhanced** `crates/connection-manager/Cargo.toml` — Added `uuid.workspace = true`.

**Test results**: 126 tests (75 traffic-guard + 51 connection-manager), 0 failures.

### Batch 4: Admin API Completion (354 tests total)
- **Enhanced** `crates/admin-api/src/routes/connections.rs` — Added DELETE /api/connections/:id (disconnect), GET /api/connections/stats
- **Enhanced** `crates/admin-api/src/routes/sessions.rs` — Added GET /api/sessions/:id, POST /api/sessions/:id/drain, GET /api/sessions/:id/metrics
- **Enhanced** `crates/admin-api/src/routes/plugins.rs` — Added POST /api/plugins/:id/restart, GET /api/plugins/:id/health
- **Enhanced** `crates/admin-api/src/routes/guard.rs` — Added GET /api/guard/bans, POST /api/guard/whitelist, POST /api/guard/blacklist, GET /api/guard/reputation/:ip
- **Enhanced** `crates/admin-api/src/routes/metrics.rs` — Added GET /api/metrics/activity
- **Created** `crates/admin-api/src/routes/config.rs` — GET /api/config, POST /api/config/reload
- **Created** `crates/admin-api/src/routes/billing.rs` — GET /api/billing/plans, /usage/:id, PUT /plan/:id
- **Created** `crates/admin-api/src/routes/audit.rs` — GET /api/audit, GET /api/audit/:id
- **Created** `crates/admin-api/src/routes/ws_streams.rs` — 5 WebSocket streams: /ws/events, /ws/connections, /ws/plugins, /ws/guard, /ws/metrics
- **Enhanced** `crates/admin-api/src/routes/mod.rs` — Registered all ~35 routes including 5 WebSocket streams
- **Enhanced** `crates/admin-api/src/auth.rs` — Added AdminRateLimiter (governor), rate_limit_middleware
- **Enhanced** `crates/admin-api/src/server.rs` — Added trace_middleware layer
- **Enhanced** `crates/admin-api/src/state.rs` — Added audit_log: Arc<AuditLog> field
- **Added** `governor.workspace = true` and `uuid.workspace = true` to admin-api Cargo.toml

### Batch 3: Infrastructure & Reliability (352 tests)
- **Enhanced** `crates/connection-manager/src/session.rs` — Added promote_connection, demote_connection, get_role methods
- **Enhanced** `crates/connection-manager/src/manager.rs` — Added SessionMetrics (AtomicU64), SessionMetricsSnapshot, promote/demote/migrate_connection, record_bytes_in/out/message, get_metrics, drain_session, is_draining. Modified bind_connection to check drain status, create_session/destroy_session for metrics/draining maps
- **Enhanced** `crates/connection-manager/src/lib.rs` — Added SessionMetrics, SessionMetricsSnapshot exports
- **Created** `crates/plugin-host/src/dependency_graph.rs` — DependencyGraph with HashMap-based DAG, add_dependency (with cycle detection via DFS), activation_order (Kahn's topological sort), can_deactivate, DependencyError enum. 9 tests
- **Created** `crates/plugin-host/src/dir_watcher.rs` — DirWatcher using notify crate, PluginFileEvent (Created/Modified/Removed), .dxp file filtering, sync→async bridge via blocking_send. 3 tests
- **Enhanced** `crates/plugin-host/src/registry.rs` — Added restart(), activate_with_timeout(), unregister() methods. 5 tests
- **Created** `crates/traffic-guard/src/adaptive.rs` — AdaptiveThrottle with sysinfo crate, AdaptiveConfig (CPU/memory thresholds), SystemLoad snapshot, ThrottleState (Normal/Throttled), consecutive overload tracking. 6 tests
- **Created** `crates/activity-log/src/audit.rs` — AuditLog with tamper-evident sequence IDs (AtomicU64), AuditEntry struct, AuditAction enum (20 variants), record/query/verify_integrity. 8 tests
- **Created** `crates/admin-api/src/trace_context.rs` — trace_middleware (extract/generate trace_id, log request start/end with duration), X-Trace-Id header propagation. 3 tests
- **Updated** `crates/draox-server/src/main.rs` — Added AuditLog creation and wiring into AppState

### Batch 2: Protocol & Advanced Features (307 tests)
- **Created** `crates/traffic-guard/src/protocol_guards.rs` — ProtocolGuard with per-IP governor rate limiters for HTTP/WS/UDP, SlowlorisDetector
- **Created** `crates/traffic-guard/src/behavioral.rs` — BehavioralAnalyzer with BehaviorFlag, burst detection, payload uniformity analysis
- **Created** `crates/plugin-messaging/src/presence.rs` — PresenceTracker, PresenceStatus (Online/Away/DnD/Offline), PresenceInfo
- **Created** `crates/plugin-messaging/src/typing.rs` — TypingTracker with nested DashMap and timeout-based auto-expiry
- **Created** `crates/plugin-messaging/src/offline_queue.rs` — OfflineQueue with per-user VecDeque, max queue eviction
- **Created** `crates/plugin-messaging/src/moderation.rs` — ContentModerator with word blocklist, MuteEntry with expiry, rate limiting
- **Created** `crates/plugin-clans/src/invites.rs` — InviteManager, ClanInvite with is_expired/is_exhausted/is_valid
- **Created** `crates/plugin-clans/src/alliances.rs` — AllianceManager, Alliance, AllianceStatus (Proposed/Active/Rejected/Dissolved)
- **Created** `crates/socket-server/src/bandwidth.rs` — BandwidthThrottle with per-connection token bucket
- **Created** `crates/socket-server/src/ws_rooms.rs` — RoomManager with bidirectional DashMap mapping
- **Created** `crates/socket-server/src/backpressure.rs` — BackpressureManager with high/low watermarks

### Batch 1: Core Service Enhancements (246 tests)
- **Created** `crates/traffic-guard/src/auth_failure.rs` — AuthFailureTracker: per-IP auth failure tracking with windowed counter, auto-ban
- **Created** `crates/traffic-guard/src/concurrent_connections.rs` — ConcurrentConnectionLimiter: DashMap + AtomicU32 lock-free tracking
- **Created** `crates/traffic-guard/src/subnet_limiter.rs` — SubnetLimiter: /24 IPv4 and /48 IPv6 subnet aggregation with governor
- **Created** `crates/traffic-guard/src/circuit_breaker.rs` — CircuitBreaker: Closed/Open/HalfOpen pattern
- **Enhanced** `crates/traffic-guard/src/guard.rs` — Added auth_failure, concurrent, subnet_limiter, circuit_breaker, connection_ips tracking to 9-step pipeline
- **Enhanced** `crates/plugin-clans/src/clan.rs` — Added description, icon_url, tags, settings fields
- **Enhanced** `crates/plugin-clans/src/manager.rs` — Added transfer_ownership, kick_member, ban_member, search_clans, get_stats, ClanStats struct
- **Enhanced** `crates/plugin-messaging/src/message.rs` — Added ContentType enum, MessageReaction struct, reply_to, edited fields
- **Enhanced** `crates/plugin-messaging/src/store.rs` — Added update_status, search_messages, add/remove_reaction, get_thread, delete_channel
- **Enhanced** `crates/plugin-messaging/src/channel.rs` — Added ChannelType enum, topic, pinned_messages, set_topic/pin/unpin
- **Created** `crates/activity-log/src/time_series.rs` — TimeSeries with BucketSize enum, sliding windows, aggregation
- **Created** `crates/activity-log/src/percentiles.rs` — PercentileTracker with sorted-Vec, PercentileSnapshot (p50/p90/p95/p99)
- **Created** `crates/activity-log/src/sinks.rs` — LogSink trait, MemorySink (ring buffer), CompositeSink (fan-out)

## 2026-04-13

### Phases 12–14 Implementation — Security, Observability, Marketplace
- **Updated** `crates/admin-api/src/auth.rs` — Phase 12: JWT authentication (HS256 via jsonwebtoken), AdminRole enum (Admin/Operator/Viewer) with RBAC, ApiKeyEntry struct, JwtClaims/JwtConfig, create/validate JWT tokens, api_key_auth middleware (Bearer JWT → X-Api-Key fallback), require_write/require_admin guards. 5 tests.
- **Updated** `crates/admin-api/src/routes/app.rs` — Phase 13: Added `/api/health/detailed` endpoint with AggregateHealthResponse, ComponentHealth structs (connections, sessions, traffic_guard, plugins, error_rate)
- **Updated** `crates/admin-api/src/routes/metrics.rs` — Phase 13: Added `/api/metrics/prometheus` endpoint with Prometheus text format (HELP/TYPE annotations for all metrics)
- **Created** `crates/plugin-host/src/package.rs` — Phase 14: DxpPackage struct (manifest + signature + wasm_bytes + assets), from_manifest(), validate(), plugin_id(), is_wasm(), is_signed(), set_signature(). 4 tests.
- **Created** `crates/plugin-host/src/signature.rs` — Phase 14: SignatureVerifier with trusted key management, Ed25519 placeholder verification (structural checks), verify() returns Ok(true/false)/Err. 5 tests.
- **Created** `crates/plugin-host/src/loader.rs` — Phase 14: PluginLoader with DashMap<PluginId, DxpPackage>, install/uninstall/get_package/list_installed, signature requirement enforcement. 6 tests.
- **Updated** `crates/plugin-host/src/lib.rs` — Added modules: package, signature, loader; re-exports: DxpPackage, SignatureVerifier, PluginLoader
- **Fixed** `crates/plugin-host/src/loader.rs` — Removed unused `warn` import
- **Fixed** `crates/admin-api/src/routes/app.rs` — Removed unused `mut` on `overall_healthy`
- **Fixed** `crates/draox-server/src/main.rs` — Changed `_rx` to `rx` in test (referenced in `drop(rx)`)
- **Tests**: 186 total across 15 crates, 0 warnings

### Phase 11 Implementation — Server Binary (`draox-server`)
- **Created** `crates/draox-server/Cargo.toml` — Binary crate depending on all 14 library crates + anyhow + tokio + tracing + tracing-subscriber
- **Created** `crates/draox-server/src/main.rs` — `#[tokio::main]` entry point: tracing init, ShutdownSignal, EventBus, ConnectionTracker, SessionManager, TrafficGuard (wrapping SessionHandler), ActivityLog (event listener), MetricsCollector, UsageTracker, PluginRegistry (registers ClansPlugin + MessagingPlugin), AdminServer (127.0.0.1:9100), Ctrl+C graceful shutdown
- **Updated** `Cargo.toml` (workspace) — Added `crates/draox-server` to members
- **Tests**: 2 tests (server_info, shutdown_signal)

### Phase 10 Implementation — Plugin Messaging (`plugin-messaging`)
- **Updated** `crates/plugin-messaging/Cargo.toml` — Dependencies (server-core, plugin-sdk, dashmap, tokio, tracing, chrono, serde, serde_json, uuid; dev: plugin-host)
- **Created** `crates/plugin-messaging/src/message.rs` — Message struct (id, message_type, from, to, content, timestamp, status), MessageType enum (Direct/Channel/Broadcast/System), MessageStatus enum (Sent/Delivered/Read). 2 tests.
- **Created** `crates/plugin-messaging/src/channel.rs` — Channel struct with HashSet<String> subscribers, subscribe/unsubscribe/is_subscribed. 2 tests.
- **Created** `crates/plugin-messaging/src/store.rs` — MessageStore with DashMap indexes (messages, client_messages, channel_messages, channels), send_message/get_message/get_client_messages/create_channel/subscribe/send_to_channel. 4 tests.
- **Created** `crates/plugin-messaging/src/plugin.rs` — MessagingPlugin implementing Plugin trait with MessageStore on activate. 1 test.
- **Updated** `crates/plugin-messaging/src/lib.rs` — Module declarations and re-exports
- **Tests**: 9 tests passing

### Phase 9 Implementation — Plugin Clans (`plugin-clans`)
- **Updated** `crates/plugin-clans/Cargo.toml` — Dependencies (server-core, plugin-sdk, dashmap, tokio, tracing, chrono, serde, serde_json, uuid; dev: plugin-host)
- **Created** `crates/plugin-clans/src/clan.rs` — Clan struct (id, name, tag, owner, members DashMap, max_members, timestamps), ClanRole enum (Owner/Officer/Member/Recruit) with rank ordering and permissions (can_kick, can_invite, can_manage_roles, can_manage_clan), ClanMember struct. 3 tests.
- **Created** `crates/plugin-clans/src/manager.rs` — ClanManager with DashMap<ClanId, Clan> + client_to_clan index, create_clan/delete_clan/join_clan/leave_clan/set_role/list_clans, max 50 members default, owner cannot leave restriction. 7 tests.
- **Created** `crates/plugin-clans/src/plugin.rs` — ClansPlugin implementing Plugin trait with ClanManager on activate. 1 test.
- **Updated** `crates/plugin-clans/src/lib.rs` — Module declarations and re-exports
- **Tests**: 11 tests passing

### Phase 8 Implementation — Admin API (`admin-api`)
- **Updated** `crates/admin-api/Cargo.toml` — Dependencies (all service crates + axum, tower, tower-http, jsonwebtoken, tokio, tracing, serde, serde_json, chrono)
- **Created** `crates/admin-api/src/error.rs` — ApiError struct with IntoResponse, helpers: not_found, bad_request, unauthorized, forbidden, internal; From<server_core::Error>
- **Created** `crates/admin-api/src/response.rs` — ApiResponse<T> wrapper (success, data, message)
- **Created** `crates/admin-api/src/auth.rs` — AdminRole enum, AuthContext, ApiKeyEntry, api_key_auth middleware
- **Created** `crates/admin-api/src/state.rs` — AppState struct with Arc refs to all services
- **Created** `crates/admin-api/src/server.rs` — AdminServer::start() with CORS + Trace layers, graceful shutdown
- **Created** `crates/admin-api/src/routes/mod.rs` — build_router() with 16 routes
- **Created** `crates/admin-api/src/routes/app.rs` — health, info endpoints
- **Created** `crates/admin-api/src/routes/connections.rs` — list_connections, get_connection
- **Created** `crates/admin-api/src/routes/sessions.rs` — list_sessions, destroy_session
- **Created** `crates/admin-api/src/routes/plugins.rs` — list/get/activate/deactivate/enable/disable
- **Created** `crates/admin-api/src/routes/guard.rs` — guard_stats, ban_ip, unban_ip
- **Created** `crates/admin-api/src/routes/metrics.rs` — get_metrics (JSON)
- **Updated** `crates/admin-api/src/lib.rs` — Module declarations + 9 integration tests with TestHandler
- **Updated** `crates/traffic-guard/src/ban_manager.rs` — Added `active_ban_count()` method
- **Updated** `crates/traffic-guard/src/ip_filter.rs` — Added `blacklist_count()` and `whitelist_count()` methods
- **Tests**: 9 integration tests passing

### Phase 7 Implementation — Plugin Host (`plugin-host`)
- **Updated** `crates/plugin-host/Cargo.toml` — Dependencies (server-core, server-config, plugin-sdk, dashmap, tokio, tracing, chrono, serde, serde_json)
- **Created** `crates/plugin-host/src/handles.rs` — Service handle implementations: NoopConnectionHandle, InMemoryStorageHandle (DashMap namespace-scoped), InMemoryCacheHandle (DashMap), EventBusHandleImpl (Arc<EventBus>), PluginLoggerImpl (tracing with plugin_id), NoopRouterHandle, NoopSchedulerHandle. 7 tests.
- **Created** `crates/plugin-host/src/lifecycle.rs` — validate_transition() function for PluginState machine. 2 tests.
- **Created** `crates/plugin-host/src/context_builder.rs` — ContextBuilder with new(ServerInfo, Arc<EventBus>) and build(plugin_id, config) → PluginContext. 1 test.
- **Created** `crates/plugin-host/src/registry.rs` — PluginRegistry with DashMap<PluginId, PluginEntry>, register_builtin/activate/deactivate/enable/disable/health_check/get_state/list/get_info/deactivate_all. State guards: enable requires ActiveDisabled, disable requires ActiveEnabled. 9 tests.
- **Updated** `crates/plugin-host/src/lib.rs` — Module declarations and re-exports
- **Tests**: 19 tests passing (later expanded to 34 with Phase 14 additions)

### Phase 5b Implementation — Cache Layer (`cache-layer`)
- **Updated** `crates/cache-layer/Cargo.toml` — Dependencies (server-core, server-config, moka, tokio, tracing, serde, serde_json)
- **Created** `crates/cache-layer/src/backend.rs` — CacheBackend trait (get, set with TTL, delete, exists)
- **Created** `crates/cache-layer/src/memory.rs` — MemoryCache wrapping moka::future::Cache with configurable max_capacity and default_ttl
- **Updated** `crates/cache-layer/src/lib.rs` — Module declarations and re-exports
- **Tests**: 7 tests passing (set_get, get_missing, delete, exists, overwrite, ttl_expiry, default_config)

### Phase 6a Implementation — Activity Log (`activity-log`)
- **Updated** `crates/activity-log/Cargo.toml` — Added dependencies (server-core, server-config, dashmap, tokio, tracing, chrono, serde, serde_json)
- **Created** `crates/activity-log/src/query.rs` — `LogFilter` struct with optional category, event_type, from/to DateTime, and limit fields
- **Created** `crates/activity-log/src/logger.rs` — `ActivityLog` struct backed by DashMap with monotonically increasing AtomicU64 IDs, `min_id` tracking for oldest-entry eviction when exceeding `max_entries`, `record()` / `get()` / `query()` / `count()` methods; `LogEntry` struct (id, timestamp, category, event_type, serde_json::Value details); `start_event_listener()` spawns tokio task subscribing to EventBus `subscribe_all()`, converts all 17 ServerEvent variants to LogEntry via `server_event_to_log_parts()` helper; handles broadcast lag and shutdown
- **Created** `crates/activity-log/src/metrics.rs` — `MetricsCollector` with AtomicU64/AtomicI64 fields (connections_total, connections_active, bytes_received_total, bytes_sent_total, requests_total, errors_total), `increment_connections()` / `decrement_connections()` / `record_bytes_received()` / `record_bytes_sent()` / `increment_requests()` / `increment_errors()` / `snapshot()` methods; `MetricsSnapshot` struct with Serialize/Deserialize and timestamp
- **Updated** `crates/activity-log/src/lib.rs` — Module declarations (logger, metrics, query) and re-exports (ActivityLog, LogEntry, MetricsCollector, MetricsSnapshot, LogFilter)
- **Tests**: 8 new tests passing (5 logger: record_and_get, record_over_max_entries, query_by_category, query_by_time_range, query_with_limit; 3 metrics: metrics_increment, metrics_snapshot, metrics_bytes_tracking). Total workspace: 116 tests, 0 warnings

### Phase 6b Implementation — Billing (`billing`)
- **Updated** `crates/billing/Cargo.toml` — Added dependencies (server-core, server-config, dashmap, tokio, tracing, chrono, serde, serde_json)
- **Created** `crates/billing/src/plans.rs` — `PlanTier` enum (Free/Pro/Enterprise) with serde rename_all, `Plan` struct with tier limits (requests/day, connections, bandwidth/day, price), factory methods (free, pro, enterprise, for_tier)
- **Created** `crates/billing/src/usage.rs` — `ClientUsage` struct with AtomicU64 counters (requests, bandwidth_bytes) and NaiveDate for daily reset, `UsageSummary` snapshot struct (Clone + Serialize), `UsageTracker` with DashMap-based storage (usage + plans maps), methods: record_request, record_bandwidth, get_usage, set_plan, get_plan, reset_daily; automatic date-rollover detection resets counters
- **Created** `crates/billing/src/enforcement.rs` — `QuotaStatus` enum (Ok, Warning{usage_percent}, Exceeded{resource}), `QuotaEnforcer` with check_request, check_bandwidth, check_all methods; >80% triggers Warning, >=limit triggers Exceeded; u128 arithmetic avoids overflow on Enterprise u64::MAX limits
- **Updated** `crates/billing/src/lib.rs` — Module declarations (enforcement, plans, usage) and re-exports (QuotaEnforcer, QuotaStatus, Plan, PlanTier, UsageSummary, UsageTracker)
- **Tests**: 16 new tests passing (4 plans, 5 usage, 7 enforcement). Total workspace: 106 tests, 0 warnings

### Phase 5a Implementation — Data Store (`data-store`)
- **Updated** `crates/data-store/Cargo.toml` — Added dependencies (server-core, server-config, sqlx, tokio, tracing, serde, serde_json, chrono)
- **Created** `crates/data-store/src/backend.rs` — `StorageBackend` trait with `BoxFuture` type alias, four async methods: `get`, `set`, `delete`, `list_keys`, all scoped by namespace
- **Created** `crates/data-store/src/error.rs` — `into_storage_error` helper function converting `sqlx::Error` to `server_core::Error::Storage(String)` without modifying server-core
- **Created** `crates/data-store/src/sqlite.rs` — `SqliteStorage` struct backed by `SqlitePool`, `new(url)` and `new_in_memory()` constructors, `run_migrations` creates `kv_store` table (namespace/key composite PK, JSON value as TEXT, updated_at RFC3339), full `StorageBackend` implementation (SELECT/INSERT OR REPLACE/DELETE/LIKE queries)
- **Updated** `crates/data-store/src/lib.rs` — Module declarations (backend, error, sqlite) and re-exports (BoxFuture, StorageBackend, SqliteStorage)
- **Tests**: 10 new tests passing (test_new_in_memory, test_set_and_get, test_get_missing, test_set_overwrite, test_delete, test_delete_missing, test_list_keys, test_list_keys_empty, test_namespace_isolation, test_json_values). Total workspace: 92 tests, 0 warnings

### Phase 4 Implementation — Connection Manager (`connection-manager`)
- **Created** `crates/connection-manager/src/session.rs` — `ClientSession` struct with HashMap<ConnectionId, ConnectionRole>, role validation (max 1 Primary, max 1 Control), add/remove/has/count/touch methods; `SessionInfo` summary struct with Serialize/Deserialize, From<&ClientSession> conversion
- **Created** `crates/connection-manager/src/manager.rs` — `SessionManager` with triple-index DashMaps (sessions, conn_to_session, client_to_session), create_session/bind_connection/unbind_connection/destroy_session/get_session methods, grace period mechanism, event publishing (SessionCreated, SessionDestroyed), expired_empty_sessions() for cleanup task
- **Created** `crates/connection-manager/src/handler.rs` — `SessionHandler` implementing `ConnectionHandler` trait from socket-server, creates session + binds Primary on connect, touches session on data, unbinds on disconnect, logs errors
- **Created** `crates/connection-manager/src/heartbeat.rs` — `session_cleanup_task` async background task, runs every 10s, finds sessions with no connections past grace_period_secs, destroys expired sessions, respects ShutdownReceiver
- **Updated** `crates/connection-manager/src/lib.rs` — Module declarations (handler, heartbeat, manager, session) and re-exports (SessionHandler, session_cleanup_task, SessionManager, ClientSession, SessionInfo)
- **Updated** `crates/connection-manager/Cargo.toml` — Added dependencies (server-config, socket-server, dashmap, tokio, tracing, chrono, serde, serde_json)
- **Tests**: 12 new tests passing (6 session, 5 manager, 1 handler). Total workspace: 75 tests, 0 warnings

### Phase 3 Implementation — Traffic Guard (`traffic-guard`)
- **Created** `crates/traffic-guard/src/verdict.rs` — `GuardVerdict` enum (Allow, Block(reason), Throttle) with Display trait
- **Created** `crates/traffic-guard/src/ip_filter.rs` — `IpFilter` struct with RwLock-based IP/CIDR blacklist and whitelist, dynamic add/remove methods, CIDR matching via ipnet
- **Created** `crates/traffic-guard/src/rate_limiter.rs` — Per-IP `RateLimiter` using governor token bucket (DashMap<IpAddr, Arc<governor::RateLimiter>>), configurable requests-per-sec and burst size
- **Created** `crates/traffic-guard/src/ban_manager.rs` — `BanManager` with DashMap-based ban tracking, auto-ban on violation threshold, escalating ban durations (initial * multiplier^count, capped at max), background cleanup task for expired bans, manual unban
- **Created** `crates/traffic-guard/src/reputation.rs` — `ReputationTracker` with per-IP scoring (DashMap<IpAddr, ReputationEntry>), violation penalty, min-score blocking, background recovery task (per-minute score recovery)
- **Created** `crates/traffic-guard/src/guard.rs` — Main `TrafficGuard` struct orchestrating all checks (whitelist > blacklist > ban > reputation > rate limit), implements `ConnectionHandler` trait, publishes ServerEvent on block/ban, delegates to next_handler on allow
- **Updated** `crates/traffic-guard/src/lib.rs` — Module declarations and re-exports (TrafficGuard, BanManager, IpFilter, RateLimiter, ReputationTracker, GuardVerdict, BanEntry, ReputationEntry)
- **Updated** `crates/traffic-guard/Cargo.toml` — Added dependencies (server-core, server-config, socket-server, governor, ipnet, dashmap, tokio, tracing, chrono, serde)
- **Tests**: 18 new tests passing (1 verdict, 4 ip_filter, 2 rate_limiter, 5 ban_manager, 3 reputation, 3 guard). Total workspace: 63 tests, 0 warnings

### Phase 2 Implementation — Socket Server (`socket-server`)
- **Created** `crates/socket-server/src/handler.rs` — `ConnectionHandler` trait (on_connect, on_data, on_text, on_disconnect, on_error with BoxFuture), `OutgoingMessage` enum (Binary, Text, Ping, Close), `WriteSender` type alias, `NoopHandler` test helper
- **Created** `crates/socket-server/src/tracker.rs` — `ConnectionTracker` with DashMap-based registry, per-IP limits, global limits, write channel (mpsc) per connection, byte counters, state management
- **Created** `crates/socket-server/src/tls.rs` — TLS config loading from PEM files (rustls), mTLS support with CA verification, `TlsAcceptor` creation
- **Created** `crates/socket-server/src/tcp.rs` — `TcpServer` with accept loop, TcpSocket binding (reuseaddr, buffer sizes), per-connection task with concurrent read/write via `into_split()`, idle timeout, nodelay
- **Created** `crates/socket-server/src/udp.rs` — `UdpServer` with socket2 for advanced options (buffer sizes, broadcast), virtual session tracking via DashMap, per-session writer task, session timeout cleanup, platform-specific socket2→std conversion (OwnedFd/OwnedSocket)
- **Created** `crates/socket-server/src/ws.rs` — `WsServer` using axum WebSocket upgrade, ping/pong heartbeat with configurable intervals and pong timeout, message size limits, concurrent send/receive via futures_util split
- **Created** `crates/socket-server/src/http.rs` — `HttpServer` using axum with tower-http middleware (CORS, compression, tracing), body size limits, static file serving, health endpoint
- **Created** `crates/socket-server/src/listener.rs` — `MultiProtocolListener` orchestrator, starts all enabled protocol servers, returns bound addresses
- **Updated** `crates/socket-server/src/lib.rs` — Module declarations and re-exports
- **Updated** `crates/socket-server/Cargo.toml` — All dependencies (server-core, server-config, tokio, socket2, axum, tower-http, rustls, dashmap, futures-util, etc.)
- **Updated** `Cargo.toml` (workspace) — Added `futures-util = "0.3"` and `rustls-pemfile = "2"` to workspace dependencies
- **Tests**: 16 new tests passing (1 handler, 7 tracker, 2 TCP, 2 UDP, 1 WS, 1 HTTP, 2 listener). Total workspace: 45 tests, 0 warnings

### Phase 1 Implementation — Foundation Crates (`server-core`, `server-config`, `plugin-sdk`)
- **Created** Cargo workspace (`Cargo.toml`) with 14 crate members, shared dependencies, edition 2024
- **Created** `crates/server-core/` — core types (`SessionId`, `ClientId`, `ConnectionId`, `PluginId`), `Protocol` enum, `ConnectionRole`/`ConnectionState`/`ConnectionInfo`/`SessionState` structs, `Error` enum (20+ variants with `thiserror`), `EventBus` (broadcast pub/sub with topic filtering), `ShutdownSignal`
- **Created** `crates/server-config/` — `DraoxConfig` model (18 config sections: server, tcp, udp, ws, http, tls, traffic_guard, sessions, storage, cache, billing, admin_api, logging, metrics, marketplace, plugins), TOML loader with env var overrides (`DRAOX_*`), config validation (field checks + port collision detection), file watcher for hot-reload (debounced via notify crate)
- **Created** `crates/plugin-sdk/` — `Plugin` trait (activate/deactivate/on_enable/on_disable/health_check), `PluginManifest` (TOML parser with validation for reverse-domain IDs), `PluginContext` with 7 service handle traits (`ConnectionHandle`, `StorageHandle`, `CacheHandle`, `EventBusHandle`, `PluginLoggerHandle`, `RouterHandle`, `SchedulerHandle`), `PluginState`/`PluginHealth`/`ActivationEvent` enums, `PluginContributions`/`PluginPermissions` structs
- **Created** 11 stub crates for Phases 2–14 (socket-server, traffic-guard, connection-manager, data-store, cache-layer, activity-log, billing, plugin-host, admin-api, plugin-clans, plugin-messaging)
- **Created** `config/default.toml` — complete default configuration file with all sections
- **Created** `.gitignore`
- **Tests**: 29 tests passing (8 server-core, 12 server-config, 9 plugin-sdk)

### Update to v2.1 — Added `traffic-guard` Crate (Anti-Spam/DDoS)
- **Added** `traffic-guard` crate at Layer 1 (Networking) — centralized anti-spam, DDoS protection, rate limiting, IP reputation
- **Updated** crate count: 13 → **14 crates**, API endpoints: ~59 → **~72** (13 new guard endpoints), phases: 13 → **14**
- **Features**: Connection flood protection (per-IP, per-subnet, global), protocol-specific guards (TCP/UDP/WS/HTTP), IP reputation system with auto-ban/auto-expire, behavioral analysis, adaptive throttling based on server load
- **New dependencies**: governor, ipnet, sysinfo
- **Updated** `docs/design_en.html` — v2.1: added Traffic Guard section, updated all counts, architecture diagram, layer model, admin API, config, deps, phases
- **Updated** `docs/design_vi.html` — v2.1: bản tiếng Việt tương ứng với đầy đủ dấu
- **Updated** `CLAUDE.md` — 14 crates, traffic-guard entry, updated Layer 1
- **Updated** `docs/plan.md` — 14 phases, new Phase 3: Traffic Guard with detailed sub-tasks
- **Updated** `docs/history.md` — This entry
- **Updated** `docs/chat.md` — Conversation history

### Major Redesign to Draox Server v2.0 — Plugin-Based Architecture
- **Renamed** project from "Rust MCP Socket Server" to "Draox Server"
- **Removed** all MCP-related features (4 crates: mcp-core, mcp-protocol, mcp-transport, mcp-client)
- **Added** plugin system architecture (VS Code-inspired, hybrid: Built-in Rust + External WASM)
- **Added** server-authoritative multi-connections (multi-connection per client, connection roles, session continuity)
- **Added** marketplace design for plugin distribution (.dxp package format, Ed25519 signing)
- **Converted** group-manager → plugin-clans (now a plugin with clan hierarchy, divisions, alliances)
- **Added** plugin-messaging (instant messaging: direct, channel, broadcast, system messages)
- **Rewritten** `docs/design_en.html` — v2.0: 18 sections, 13 crates, 7 layers, plugin system, marketplace, ~59 API endpoints
- **Rewritten** `docs/design_vi.html` — v2.0: bản tiếng Việt tương ứng với đầy đủ dấu
- **Rewritten** `CLAUDE.md` — Draox Server, 13 crates, 7-layer model, plugin architecture notes
- **Rewritten** `docs/plan.md` — 13 phases with detailed sub-tasks
- **Updated** `docs/history.md` — This entry
- **Updated** `docs/chat.md` — Conversation history

**New crates (4):** server-core, plugin-sdk, plugin-host, plugin-messaging
**Removed crates (4):** mcp-core, mcp-protocol, mcp-transport, mcp-client
**Converted (1):** group-manager → plugin-clans
**Total: 13 crates, 7 layers, 13 phases**

## 2026-04-12

### Update All Documentation to v1.3 — Storage, Cache, Logging, Billing, Groups, Admin API
- **Updated** `docs/design_en.html` — v1.3: added 6 new sections (Data Store, Cache Layer, Activity Log, Billing, Group/Channel Manager, Admin API), 13 crates, 8-layer model, 18 sections, 15 new dependencies, 16-phase timeline, 42 REST API endpoints + 3 WebSocket streams
- **Updated** `docs/design_vi.html` — v1.3: bản tiếng Việt tương ứng với đầy đủ dấu, 13 crate, 8 tầng, 16 giai đoạn
- **Updated** `CLAUDE.md` — 13 crates, 8-layer model (Layer 0–7), admin-api entry, new dependencies
- **Updated** `docs/plan.md` — 16 phases with detailed sub-tasks for phases 7–12
- **Updated** `docs/history.md` — This entry
- **Updated** `docs/chat.md` — Conversation history

### Update Vietnamese Design Report to v1.2
- **Updated** `docs/design_vi.html` — Complete rewrite to v1.2: added Socket Server section (TCP/UDP/WS/HTTP), updated architecture diagram with UDP client, added socket-server crate card, expanded config with per-protocol TOML sections, connection state machine & WebSocket lifecycle flows, 4 new dependencies, 10-phase timeline, 7 crates

### Update English Design Report to v1.2
- **Updated** `docs/design_en.html` — Complete rewrite: added socket-server crate/section, updated architecture diagram, expanded config, 10-phase timeline, 4 new dependencies, version v1.2

### Cập nhật HTML Report tiếng Việt
- **Updated** `docs/design_vi.html` — Cập nhật toàn bộ tiếng Việt có dấu đầy đủ, nâng phiên bản lên v1.1

## 2026-04-11

### Initial Project Setup
- **Created** `docs/design_en.html` — Architecture design report (English version)
- **Created** `docs/design_vi.html` — Architecture design report (Vietnamese version)
- **Created** `CLAUDE.md` — Project conventions, architecture overview, and development guide
- **Created** `docs/chat.md` — Conversation history log
- **Created** `docs/history.md` — This file (change history)
- **Created** `docs/plan.md` — Project execution plan
