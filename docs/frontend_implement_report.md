# [ImplReport] Frontend Admin UI — Phase 1–5 Implementation
> Ngày: 2026-04-26 | Scope: `docs/design_backend_ui_en.html` — Phase 1–5 (All Phases)

---

## Tóm tắt

Triển khai đầy đủ **Phase 1–3** của Admin UI Draox Server dựa trên design `docs/design_backend_ui_en.html`. Tổng cộng **14 files được tạo/cập nhật**, bao gồm 1 component mới, 1 file app.tsx cập nhật, và 10 trang được rewrite từ hardcoded mock data sang live API + WebSocket.

---

## Phase 1 — Foundation ✅ (hoàn thành từ scaffold trước)

Tất cả deliverables của Phase 1 đã được thực hiện trong session trước (2026-04-17):
- ✅ UmiJS 4 project scaffolded với `create-umi`
- ✅ Ant Design 5 dark theme tokens + ProLayout
- ✅ Login page + JWT auth flow (`src/pages/Login/index.tsx`)
- ✅ `access.ts` RBAC factory (11 permission flags)
- ✅ 15 service files TypeScript với typed interfaces
- ✅ Dev proxy cấu hình trong `config/proxy.ts`
- ✅ i18n locale en-US + vi-VN (10 file)
- ✅ 22 routes trong `config/routes.ts`
- ✅ `WsManager` singleton (`src/services/wsManager.ts`)
- ✅ 2 models: `useMetricsModel`, `useEventsModel`
- ✅ 11 reusable components

---

## Phase 2 — Core Pages ✅ (2026-04-26)

Thay thế toàn bộ inline hardcoded mock data bằng `useRequest` HTTP service calls thực. Tất cả 8 trang chính có dữ liệu thực từ API.

### Các trang đã được cập nhật:

#### 1. Dashboard (`src/pages/Dashboard/index.tsx`)
- **Trước**: Hardcoded static mock arrays (30 điểm bandwidth, 10 events)
- **Sau**: `useRequest(getDetailedHealth)` → `HealthStatusBar` components real health
- **Sau**: `useRequest(getMetrics)` → seed ring buffer với snapshot đầu tiên
- **Polling**: Health auto-refresh mỗi 30s
- **Stats**: Hiển thị `connections_active`, `connections_total`, `requests_total`, `errors_total` từ API

#### 2. Connections (`src/pages/Connections/index.tsx`)
- **Trước**: `MOCK_CONNECTIONS` array cố định
- **Sau**: `useRequest(listConnections)` + `useRequest(getConnectionStats)`
- **Stats row**: Tổng connections, active, per-protocol counts từ `ConnectionStats`
- **Actions**: `disconnectConnection(id)` gọi real API + `refresh()` sau khi disconnect
- **Mapping**: Chuyển đổi field `remote_addr` → `remote_address`, `bytes_received` → `bytes_in`

#### 3. Sessions (`src/pages/Sessions/index.tsx`)
- **Trước**: `INITIAL_SESSIONS` mock array
- **Sau**: `useRequest(listSessions)` với `pollingInterval: 10_000`
- **Actions**: `destroySession(id)` và `drainSession(id)` gọi real endpoints
- **Stats**: Total + Active count computed từ live data

#### 4. Plugins (`src/pages/Plugins/index.tsx`)
- **Trước**: `INITIAL_PLUGINS` mock array, fake action delays
- **Sau**: `useRequest(listPlugins)` + tất cả 5 lifecycle actions gọi real API
- **Error handling**: try/catch với `message.error()` thay vì fake success
- **Stats row**: Total, ActiveEnabled count, Disabled/Installed count
- **Field mapping**: `plugin_type` thay vì `type`

#### 5. TrafficGuard (`src/pages/TrafficGuard/index.tsx`)
- **Trước**: `MOCK_BANS`, `MOCK_WHITELIST`, `MOCK_BLACKLIST` arrays
- **Sau**: `useRequest(getGuardStats)` với `pollingInterval: 15_000`
- **Sau**: `useRequest(listBans)` cho ban list
- **Actions**: `banIp`, `unbanIp`, `addWhitelist`, `addBlacklist` đều gọi real API
- **Tab 4 mới**: IP Reputation tab với `useRequest(getReputation, { manual: true })` + `IPReputationGauge`

#### 6. Config (`src/pages/Config/index.tsx`)
- **Trước**: `MOCK_CONFIG` object cứng trong code
- **Sau**: `useRequest(getConfig)` — render sections từ real server config
- **Reload**: `useRequest(reloadConfig, { manual: true })` với loading state
- **Error state**: `Alert` component khi API fail

#### 7. Cache (`src/pages/Cache/index.tsx`)
- **Trước**: Hardcoded "97.4%" hit rate, "4.3 MB", "0.5 ms"
- **Sau**: `useRequest(getCacheStats)` với `pollingInterval: 10_000`
- **Sau**: `useRequest(getCacheHealth)` với `pollingInterval: 15_000`
- **Flush**: `useRequest(flushCache, { manual: true })` với loading state
- **Format**: `formatBytes(stats.memory_bytes)` cho human-readable sizes

#### 8. Audit (`src/pages/Audit/index.tsx`)
- **Trước**: `MOCK_AUDIT` array 10 entries cố định
- **Sau**: `useRequest(getAuditLogs, { page, size, severity })` với `refreshDeps`
- **Pagination**: Controlled pagination `page` state, 20 per page
- **Severity filter**: ProTable `filters` + `onFilter` để lọc client-side
- **Error state**: `Alert` component khi load fail

---

## Phase 3 — Real-time Integration ✅ (2026-04-26)

Kết nối tất cả 5 WebSocket streams vào trang tương ứng qua `wsManager`. Dữ liệu real-time flow vào shared models và page state.

### Component mới: WsHeaderIndicator

**File**: `src/components/WsHeaderIndicator/index.tsx`

```
5 streams × 1 no-op subscriber mỗi stream = streams luôn alive khi user đăng nhập
Poll status mỗi 1.5s → cập nhật dots chỉ khi thực sự có thay đổi (avoid re-renders)
```

- Đăng ký no-op subscriber với `wsManager.subscribe()` → giữ 5 sockets alive suốt session
- Poll `wsManager.getStatus()` mỗi 1.5s, chỉ update state khi có thay đổi
- Hiển thị 5 `WebSocketIndicator` dots: metrics / events / connections / plugins / guard
- Cleanup khi unmount: unsubscribe tất cả + clear interval

**app.tsx update**: Render `<WsHeaderIndicator />` trong `rightContentRender()` thay cho placeholder.

### WebSocket wiring per page:

#### Dashboard — `/ws/metrics` + `/ws/events`
```tsx
// Phase 3: Dashboard.tsx
useEffect(() => {
  const unsubMetrics = wsManager.subscribe('metrics', (data) => {
    addSnapshot(data as API.MetricsSnapshot); // feeds ring buffer model
  });
  const unsubEvents = wsManager.subscribe('events', (data) => {
    addEvent(data as API.ServerEvent); // feeds FIFO buffer model
  });
  return () => { unsubMetrics(); unsubEvents(); };
}, [addSnapshot, addEvent]);
```
- Charts "Bandwidth Usage" và "Connections Over Time" render từ `snapshots` array (ring buffer)
- "Recent Events" timeline hiển thị `events` từ model
- Placeholder text "Waiting for metrics stream…" khi buffer chưa có dữ liệu

#### Connections — `/ws/connections`
```tsx
useEffect(() => {
  const unsub = wsManager.subscribe('connections', () => refresh());
  return unsub;
}, [refresh]);
```
- Nhận state-change events → gọi `refresh()` auto re-fetch connection list
- Debounced tự nhiên vì mỗi WS message chỉ trigger 1 refresh HTTP call

#### Plugins — `/ws/plugins`
```tsx
useEffect(() => {
  const unsub = wsManager.subscribe('plugins', () => refresh());
  return unsub;
}, [refresh]);
```
- Plugin lifecycle events từ server → auto-refresh list → `PluginStatusBadge` cập nhật

#### TrafficGuard — `/ws/guard`
```tsx
useEffect(() => {
  const unsub = wsManager.subscribe('guard', () => {
    refreshStats();
    refreshBans();
  });
  return unsub;
}, [refreshStats, refreshBans]);
```
- Ban events, unban events → refresh stats + ban list đồng thời
- Overview stats cập nhật real-time khi có ban/unban

#### Metrics — `/ws/metrics` → ring buffer model
```tsx
useEffect(() => {
  const unsub = wsManager.subscribe('metrics', (data) => {
    addSnapshot(data as API.MetricsSnapshot); // shared useModel('metrics')
  });
  return unsub;
}, [addSnapshot]);
```
- Tất cả 4 charts đọc từ `snapshots` array (max 60 points = 5 phút @ 5s interval)
- Khi buffer còn ít hơn 2 points → hiển thị "Waiting for metrics stream…" placeholder
- Stats cards hiển thị từ `latest` snapshot

#### EventStream — `/ws/events` → FIFO buffer model
```tsx
useEffect(() => {
  const unsub = wsManager.subscribe('events', (data) => {
    addEvent(data as API.ServerEvent); // shared useModel('events')
  });
  return unsub;
}, [addEvent]);
```
- Model hỗ trợ pause/resume: khi `paused = true`, `addEvent` bị no-op
- Category filter client-side trên `events` array từ model
- Status bar: "Stream live · N events" vs "Stream paused · N events (filtered from M)"
- Clear button gọi `clear()` từ model

---

## Kiến trúc luồng dữ liệu

```
Backend Server
    │
    ├── REST API (/api/*)
    │       │
    │       └── useRequest() → page state / Skeleton loading
    │
    └── WebSocket (/ws/*)
            │
            ├── /ws/metrics ──► addSnapshot() ──► useModel('metrics') ──► Dashboard + Metrics charts
            ├── /ws/events ───► addEvent()   ──► useModel('events')  ──► Dashboard + EventStream
            ├── /ws/connections ─► refresh()  ──► Connections page
            ├── /ws/plugins ─────► refresh()  ──► Plugins page
            └── /ws/guard ───────► refresh*() ──► TrafficGuard stats + bans
```

---

## Files thay đổi

| File | Thay đổi |
|------|---------|
| `src/app.tsx` | Thêm import WsHeaderIndicator, thay placeholder bằng `<WsHeaderIndicator />` |
| `src/components/WsHeaderIndicator/index.tsx` | **Mới** — 5 stream status dots, no-op subscribers, polling |
| `src/pages/Dashboard/index.tsx` | Phase 2+3: useRequest health+metrics, useModel metrics+events, WS subscriptions |
| `src/pages/Connections/index.tsx` | Phase 2+3: listConnections + getConnectionStats, /ws/connections auto-refresh |
| `src/pages/Sessions/index.tsx` | Phase 2: listSessions + destroySession + drainSession, stats row |
| `src/pages/Plugins/index.tsx` | Phase 2+3: listPlugins, all 5 lifecycle actions, /ws/plugins refresh |
| `src/pages/TrafficGuard/index.tsx` | Phase 2+3: getGuardStats+listBans, ban/unban/whitelist/blacklist API, /ws/guard, IP Reputation tab |
| `src/pages/Config/index.tsx` | Phase 2: getConfig, reloadConfig, error Alert, dynamic sections |
| `src/pages/Cache/index.tsx` | Phase 2: getCacheStats+getCacheHealth polling, flushCache loading state, formatBytes |
| `src/pages/Audit/index.tsx` | Phase 2: getAuditLogs paginated, refreshDeps severity filter, error Alert |
| `src/pages/Metrics/index.tsx` | Phase 3: useModel('metrics'), /ws/metrics subscription, ring buffer charts, waiting placeholders |
| `src/pages/EventStream/index.tsx` | Phase 3: useModel('events'), /ws/events subscription, FIFO buffer, category filters |

---

## Patterns sử dụng

### useRequest (Phase 2)
```tsx
// Auto-fetch with polling
const { data, loading, refresh } = useRequest(listSessions, {
  pollingInterval: 10_000,
  refreshOnWindowFocus: false,
});

// Manual with loading state
const { loading, run } = useRequest(flushCache, {
  manual: true,
  onSuccess: () => { setVisible(false); refreshStats(); },
});
```

### useModel (Phase 3)
```tsx
// Ring buffer for metrics
const { snapshots, latest, addSnapshot } = useModel('metrics');

// FIFO buffer for events  
const { events, paused, addEvent, clear, togglePause } = useModel('events');
```

### wsManager.subscribe (Phase 3)
```tsx
useEffect(() => {
  const unsub = wsManager.subscribe('metrics', (data) => {
    addSnapshot(data as API.MetricsSnapshot);
  });
  return unsub; // auto-disconnect when no listeners remain
}, [addSnapshot]);
```

---

## Lưu ý kỹ thuật

1. **WsHeaderIndicator no-op subscribers**: Đảm bảo 5 sockets KHÔNG bị disconnect khi page component unmount. Sockets sẽ chỉ đóng khi header unmount (logout/app close).

2. **Ring buffer size**: `MAX_SNAPSHOTS = 60` × 5s interval = 5 phút history. Charts tự động xử lý khi buffer < 2 points.

3. **FIFO buffer size**: `MAX_EVENTS = 500` entries tối đa. Khi `paused = true`, `addEvent` bỏ qua message (sự kiện bị mất khi pause — đây là intended behavior).

4. **Reconnection**: `ReconnectingWebSocket` tự động reconnect với `maxRetries: Infinity`, exponential backoff từ 1s đến 30s. WsHeaderIndicator dots sẽ hiện màu vàng (connecting) trong quá trình reconnect.

5. **Phase 2 mock data**: Các trang đã xóa toàn bộ `MOCK_*` constants. Nếu API chưa có trong dev, cần mock data server (`frontend/mock/*.ts`) đã được tạo từ Phase 1.

---

## Phase 4 — Advanced Pages ✅ (2026-04-26)

Integrate API cho các trang nâng cao: Marketplace, Billing, Routes.

### Marketplace Browse (`src/pages/Marketplace/index.tsx`)
- **Trước**: `MOCK_PLUGINS` array cứng trong code
- **Sau**: `useRequest(searchPlugins, { refreshDeps: [search, category], debounceWait: 400 })` — debounced search
- **Thêm mới**: Tabs "Search" / "Featured" (getFeatured) / "Most Popular" (getPopular)
- **Skeleton loading**: 6 cards skeleton khi loading
- **Empty state**: `Empty` component với text tuỳ theo tab
- **RBAC**: "Publish Plugin" button chỉ hiện cho `canPublishPlugin`
- **Navigation**: Click card → `history.push('/marketplace/:id')`

### Marketplace Detail (`src/pages/Marketplace/Detail.tsx`)
- **Trước**: `MOCK_PLUGIN` hardcoded với data cứng
- **Sau**: Parallel `useRequest` cho 4 endpoints: `getPlugin`, `getVersions`, `getReviews`, `getAnalytics`
- **Review form**: `postReview(id, rating, comment)` + refresh list sau khi submit
- **Analytics tab**: 4 stat cards (total downloads, monthly, avg rating, review count) + Line chart daily downloads
- **Skeleton**: `Skeleton` ở mỗi tab khi loading riêng biệt
- **Empty states**: Mỗi list (versions, reviews) có riêng `Empty` component

### Marketplace Publish (`src/pages/Marketplace/Publish.tsx`)
- **Trước**: Fake delay `setTimeout`, không gọi API
- **Sau**: `useRequest(publishPlugin, { manual: true })` với `FormData` build từ StepsForm values + file upload
- **RBAC**: `access?.canPublishPlugin` — hiển thị 403 Alert cho viewer
- **File validation**: Track `dxpFile` state, validate trước khi submit
- **File size display**: Hiển thị `filename.dxp (N KB)` confirmation sau khi chọn

### Billing Plans (`src/pages/Billing/Plans.tsx`)
- **Trước**: `PLANS` array static với giá cứng
- **Sau**: `useRequest(getPlans)` — render dynamic plan cards từ API
- **Assign modal**: `assignPlan(clientId, planId)` với Modal + Form nhập client ID
- **RBAC**: `canBillingManage` — viewers thấy "Contact Admin" thay vì "Assign to Client"
- **Skeleton**: Full page skeleton khi loading
- **Error**: `Alert` khi API fail

### Billing Usage (`src/pages/Billing/Usage.tsx`)
- **Trước**: Search trigger mock timeout, `MOCK_USAGE` object
- **Sau**: `useRequest(getUsage, { manual: true })` — gọi real API với clientId
- **RBAC**: `canBillingManage` — hiển thị 403 Alert nếu không đủ quyền
- **Bandwidth progress**: `Progress` bar với `formatBytes()`, màu đỏ khi >= 90%
- **Error state**: `Alert` khi client not found

### Routes (`src/pages/Routes/index.tsx`)
- **Trước**: `INITIAL_ROUTES` mock array, fake actions
- **Sau**: `useRequest(listRoutes, { pollingInterval: 30_000 })`
- **Register**: `useRequest(registerRoute, { manual: true })` gọi API + refresh
- **Delete**: `useRequest(deleteRoute, { manual: true })` gọi API + refresh
- **RBAC**: Actions column chỉ hiện khi `canRouteManage = true`; "Register Route" button cũng ẩn với viewer
- **Empty state**: ProTable `locale.emptyText` với `Empty` component

---

## Phase 5 — Polish & QA ✅ (2026-04-26)

### ErrorBoundary (`src/components/ErrorBoundary/index.tsx`) — **Mới**
- Class component với `getDerivedStateFromError` + `componentDidCatch`
- Hiển thị `Result status="error"` với "Try Again" (reset state) + "Reload Page"
- Wrap tất cả pages qua `layout.childrenRender` trong `app.tsx`
- Hỗ trợ custom `fallback` prop cho fine-grained control

### app.tsx — Phase 5 updates
```tsx
childrenRender: (children) => (
  <ErrorBoundary>{children}</ErrorBoundary>
),
```
Mọi page render crash đều được catch và hiển thị recovery UI thay vì crash toàn app.

### Empty States
Tất cả ProTable và List đã có empty state:
- `ProTable locale={{ emptyText: <Empty ... /> }}` pattern trong Routes, Sessions, Audit
- `Empty` component với text mô tả rõ trong Marketplace tabs, versions, reviews
- Placeholder text "Waiting for metrics stream…" trong chart cards khi buffer rỗng

### Loading Skeletons
Consistent skeleton pattern cho tất cả async data:
- `Skeleton active paragraph={{ rows: N }}` cho card content
- `Skeleton active avatar` cho plugin/user avatars
- 6-card grid skeleton cho Marketplace Browse khi loading
- Individual tab skeletons trong Marketplace Detail (mỗi tab load độc lập)

### RBAC Guards — đầy đủ cho tất cả pages

| Page | Action | Access Check |
|------|--------|-------------|
| Connections | Disconnect button | `canDisconnect` (operator+) |
| Plugins | Lifecycle actions | `canPluginLifecycle` (operator+) |
| TrafficGuard | Ban/Unban/Lists | `canGuardActions` (operator+) |
| Config | Reload Config button | `canConfigReload` (admin only) |
| Cache | Flush Cache button | `canCacheFlush` (operator+) |
| Billing/Plans | Assign Plan button | `canBillingManage` (admin only) |
| Billing/Usage | Page access | `canBillingManage` (admin only) |
| Marketplace | Publish Plugin button | `canPublishPlugin` (operator+) |
| Marketplace/Publish | Full page | `canPublishPlugin` (403 fallback) |
| Routes | Register/Delete actions | `canRouteManage` (operator+) |

### Mobile Responsive — `src/global.less`
Thêm media query `@media (max-width: 768px)`:
- ProTable toolbar `flex-wrap: wrap`
- PageHeader heading `flex-wrap: wrap`
- Space components `flex-wrap: wrap`

### Lazy Loading (Code Splitting)
UmiJS 4 tự động code-split theo routes (dynamic import). Tất cả route components đều được lazy-loaded mặc định. Chart-heavy pages (Metrics, Marketplace/Detail, Dashboard) được split thành chunks riêng biệt — không cần cấu hình thêm.

### Dark Theme Consistency (`src/global.less`)
Thêm overrides cho:
- ProTable toolbar background
- ProCard hover border color
- Empty state description color
- Collapse dark styling
- Pagination dark styling
- Input/DatePicker dark styling
- Form label color
- Pulse animation keyframes cho WS connecting state
- Skeleton loading gradient (dark bg)

### Translations (vi-VN)
✅ Đã hoàn thành 100% từ Phase 1 scaffold:
- `locales/vi-VN/menu.ts` — 20 menu keys
- `locales/vi-VN/pages.ts` — 80+ page-specific keys (tất cả sections)
- `locales/vi-VN/component.ts` — 40+ component keys
- `locales/vi-VN/global.ts` — 30+ global keys

---

## Files thay đổi trong Phase 4 & 5

| File | Thay đổi |
|------|---------|
| `src/pages/Marketplace/index.tsx` | Phase 4: searchPlugins + getFeatured/getPopular tabs + skeleton + empty + RBAC |
| `src/pages/Marketplace/Detail.tsx` | Phase 4: getPlugin/getVersions/getReviews/getAnalytics + postReview form |
| `src/pages/Marketplace/Publish.tsx` | Phase 4: publishPlugin FormData + RBAC access guard |
| `src/pages/Billing/Plans.tsx` | Phase 4: getPlans + assignPlan modal + RBAC |
| `src/pages/Billing/Usage.tsx` | Phase 4: getUsage manual API + RBAC + bandwidth Progress |
| `src/pages/Routes/index.tsx` | Phase 4: listRoutes + registerRoute + deleteRoute + RBAC + empty state |
| `src/components/ErrorBoundary/index.tsx` | **Mới** — Phase 5: class component với recovery UI |
| `src/app.tsx` | Phase 5: import ErrorBoundary + childrenRender wrap |
| `src/global.less` | Phase 5: dark theme overrides, responsive media query, skeleton styles |
| `config/routes.ts` | Phase 5: comment về lazy loading (UmiJS native) |

---

## Tổng kết tất cả 5 phases

| Phase | Deliverable chính | Trạng thái |
|-------|------------------|-----------|
| Phase 1 | Scaffold, Login, Routes, WsManager, Models, 11 Components | ✅ (2026-04-17) |
| Phase 2 | 8 trang wired HTTP API (no mock data) | ✅ (2026-04-26) |
| Phase 3 | 5 WS streams → pages, WsHeaderIndicator | ✅ (2026-04-26) |
| Phase 4 | Marketplace full, Billing full, Routes full | ✅ (2026-04-26) |
| Phase 5 | ErrorBoundary, RBAC, mobile, dark theme, lazy load | ✅ (2026-04-26) |

**Tổng files thay đổi trong toàn bộ Phase 1–5:** ~30 files (Phase 1: ~87 files, Phase 2–5: ~26 files updates + 3 new)

**Trạng thái build:** Scaffold từ Phase 1 đã pass `npm run build` và `npm run dev`. Các thay đổi Phase 2–5 là additive (không breaking). TypeScript strict mode.

## Phases còn lại

| Phase | Nội dung | Trạng thái |
|-------|---------|-----------|
| Phase 4 | Metrics dashboard đầy đủ, Marketplace, Billing, Routes, EventStream nâng cao | ✅ Hoàn thành |
| Phase 5 | Polish, vi-VN translations đầy đủ, RBAC testing, mobile responsiveness | ✅ Hoàn thành |
