# [Plan] Draox Client SDK — Unity (C#) & JavaScript (TypeScript)

> Ngày tạo: 2026-04-29  
> Phiên bản: v1.0  
> Mục tiêu: Cung cấp SDK client cho Unity (C#) và JavaScript/TypeScript để kết nối và tương tác với Draox Server.

---

## 1. Tổng Quan

### 1.1 Bối Cảnh

Draox Server hỗ trợ 4 protocol kết nối:

| Protocol | Port (default) | Đặc điểm |
|----------|---------------|----------|
| TCP      | 9000          | Reliable, binary/text stream |
| UDP      | 9001          | Unreliable, low-latency datagrams |
| WebSocket| 9002          | Full-duplex, text/binary frames, browser-friendly |
| HTTP     | 9003          | REST request/response |

**Kiến trúc session server-authoritative:**
- Mỗi client kết nối → server tự động tạo một **Session** (ID dạng `ses_<uuid>`)
- Session có thể có nhiều Connection đồng thời với các role khác nhau:
  - `primary` — kết nối chính (tối đa 1)
  - `notification` — nhận push events
  - `control` — điều khiển (tối đa 1)
  - `streaming` — data stream liên tục
- Authentication: per-session, một lần auth → tất cả connections trong session thừa kế

### 1.2 Message Format (Wire Protocol)

Server sử dụng JSON envelope cho tất cả messages:

```jsonc
// Client → Server (Request)
{
  "id": "req_<uuid>",          // Request ID (cho correlation)
  "type": "request",
  "action": "auth",            // Tên action (hoặc plugin command)
  "payload": { ... }           // Body tùy action
}

// Server → Client (Response)
{
  "id": "req_<uuid>",          // Echo lại request ID
  "type": "response",
  "success": true,
  "data": { ... },             // Payload kết quả
  "error": null                // Hoặc error message
}

// Server → Client (Push event)
{
  "type": "event",
  "category": "session|connection|plugin|custom",
  "name": "SessionCreated",    // Tên event
  "data": { ... },
  "timestamp": "2026-04-29T..."
}
```

---

## 2. Phạm Vi SDK

### 2.1 Tính Năng Cốt Lõi (cả hai platform)

| # | Tính năng | Mô tả |
|---|-----------|-------|
| 1 | **Connect** | Kết nối tới server (chọn protocol) |
| 2 | **Authenticate** | Gửi credentials, nhận session token |
| 3 | **Send / Request** | Gửi request và chờ response (async/await) |
| 4 | **Subscribe Events** | Đăng ký nhận push events theo category |
| 5 | **Multi-Connection** | Mở thêm connection phụ (notification, streaming) |
| 6 | **Heartbeat** | Tự động ping/pong để duy trì kết nối |
| 7 | **Reconnect** | Tự động kết nối lại khi mất mạng |
| 8 | **Disconnect** | Đóng kết nối sạch |

### 2.2 Plugin API (optional modules)

Sau khi kết nối cơ bản, SDK cung cấp typed wrappers cho built-in plugins:

| Module | Plugin | Tính năng |
|--------|--------|-----------|
| `Clans` | `plugin-clans` | Tạo/join clan, danh sách thành viên |
| `Messaging` | `plugin-messaging` | Gửi/nhận tin nhắn |
| `Presence` | `plugin-presence` | Online status, last seen |
| `Identity` | `plugin-identity` | Đăng ký, login, JWT |

---

## 3. SDK cho JavaScript / TypeScript

### 3.1 Cấu Trúc Package

```
draox-client-js/
├── src/
│   ├── core/
│   │   ├── DraoxClient.ts        # Entry point chính
│   │   ├── Connection.ts         # Quản lý 1 kết nối (WS/HTTP)
│   │   ├── SessionManager.ts     # Multi-connection session
│   │   ├── RequestBroker.ts      # Pending request map + timeout
│   │   ├── EventEmitter.ts       # Typed event emitter
│   │   └── Reconnector.ts        # Auto-reconnect logic
│   ├── protocol/
│   │   ├── types.ts              # DraoxMessage, DraoxEvent, DraoxRequest...
│   │   └── serializer.ts         # JSON encode/decode
│   ├── plugins/
│   │   ├── clans.ts
│   │   ├── messaging.ts
│   │   └── presence.ts
│   └── index.ts                  # Public API exports
├── package.json
├── tsconfig.json
└── README.md
```

### 3.2 TypeScript API Design

```typescript
// ── Types ────────────────────────────────────────────────

export type DraoxProtocol = 'ws' | 'http';
export type ConnectionRole = 'primary' | 'notification' | 'control' | 'streaming';
export type EventCategory = 'session' | 'connection' | 'plugin' | 'server' | 'custom';

export interface DraoxConfig {
  host: string;                   // e.g. "localhost"
  port?: number;                  // default 9002 (WS) or 9003 (HTTP)
  protocol?: DraoxProtocol;       // default 'ws'
  tls?: boolean;                  // wss:// or https://
  timeout?: number;               // request timeout ms, default 10_000
  reconnect?: ReconnectConfig;
}

export interface ReconnectConfig {
  enabled: boolean;               // default true
  maxAttempts?: number;           // default 5
  baseDelay?: number;             // ms, default 1_000
  maxDelay?: number;              // ms, default 30_000
}

export interface DraoxEvent<T = unknown> {
  type: 'event';
  category: EventCategory;
  name: string;
  data: T;
  timestamp: string;
}

// ── DraoxClient ──────────────────────────────────────────

export class DraoxClient {
  constructor(config: DraoxConfig);

  // Lifecycle
  connect(): Promise<void>;
  disconnect(reason?: string): Promise<void>;

  // Auth
  authenticate(credentials: { userId: string; token: string }): Promise<void>;
  get sessionId(): string | null;
  get isAuthenticated(): boolean;

  // Messaging
  send(action: string, payload?: unknown): Promise<unknown>;
  request<T = unknown>(action: string, payload?: unknown): Promise<T>;

  // Events
  on<T = unknown>(eventName: string, handler: (event: DraoxEvent<T>) => void): void;
  off(eventName: string, handler: Function): void;
  onCategory(category: EventCategory, handler: (event: DraoxEvent) => void): void;

  // Multi-connection
  addConnection(role: ConnectionRole): Promise<void>;

  // State
  get state(): 'disconnected' | 'connecting' | 'connected' | 'reconnecting';
}
```

### 3.3 Ví Dụ Sử Dụng (JS/TS)

```typescript
import { DraoxClient } from 'draox-client';

const client = new DraoxClient({
  host: 'game.example.com',
  port: 9002,
  protocol: 'ws',
  tls: true,
  reconnect: { enabled: true, maxAttempts: 5 },
});

await client.connect();

await client.authenticate({
  userId: 'player_001',
  token: 'jwt_token_here',
});

// Gửi request và chờ response
const result = await client.request('clans.list', { page: 1 });
console.log(result.clans);

// Đăng ký nhận events
client.on('SessionDestroyed', (event) => {
  console.log('Session ended:', event.data);
});

client.onCategory('custom', (event) => {
  if (event.name === 'game.match_found') {
    joinMatch(event.data);
  }
});

// Thêm notification connection
await client.addConnection('notification');
```

### 3.4 Plugin Module (ví dụ Messaging)

```typescript
import { MessagingPlugin } from 'draox-client/plugins';

const chat = new MessagingPlugin(client);

await chat.sendMessage({ to: 'player_002', text: 'Hello!' });

chat.onMessage((msg) => {
  console.log(`${msg.from}: ${msg.text}`);
});
```

### 3.5 Phụ Thuộc & Build

- **Runtime**: chạy trên Browser và Node.js (ESM + CJS dual build)
- **WebSocket**: native `WebSocket` (browser) / `ws` package (Node.js)
- **HTTP**: native `fetch`
- **Bundle**: Vite hoặc tsup → `dist/index.js` + `dist/index.d.ts`
- **Không dependency nặng** — chỉ `ws` (Node.js only, optional peer dep)

---

## 4. SDK cho Unity (C#)

### 4.1 Cấu Trúc Package

```
DraoxClientUnity/
├── Runtime/
│   ├── Core/
│   │   ├── DraoxClient.cs          # Entry point
│   │   ├── WebSocketConnection.cs  # NativeWebSocket wrapper
│   │   ├── TcpConnection.cs        # System.Net.Sockets
│   │   ├── SessionManager.cs       # Multi-connection
│   │   ├── RequestBroker.cs        # Pending requests + timeout
│   │   └── Reconnector.cs          # Exponential backoff
│   ├── Protocol/
│   │   ├── DraoxMessage.cs         # JSON envelope types
│   │   └── Serializer.cs           # Newtonsoft.Json / System.Text.Json
│   ├── Plugins/
│   │   ├── ClansPlugin.cs
│   │   ├── MessagingPlugin.cs
│   │   └── PresencePlugin.cs
│   └── DraoxClientUnity.asmdef
├── Editor/
│   └── DraoxSettingsEditor.cs      # Inspector GUI cho DraoxConfig
├── Tests/
│   └── Runtime/
│       └── DraoxClientTests.cs
├── package.json                    # UPM package manifest
└── README.md
```

### 4.2 C# API Design

```csharp
// ── Types ────────────────────────────────────────────────

public enum DraoxProtocol { WebSocket, Tcp }
public enum ConnectionRole { Primary, Notification, Control, Streaming }
public enum ClientState { Disconnected, Connecting, Connected, Reconnecting }

[Serializable]
public class DraoxConfig
{
    public string Host = "localhost";
    public int Port = 9002;
    public DraoxProtocol Protocol = DraoxProtocol.WebSocket;
    public bool UseTls = false;
    public int TimeoutMs = 10_000;
    public ReconnectConfig Reconnect = new();
}

[Serializable]
public class ReconnectConfig
{
    public bool Enabled = true;
    public int MaxAttempts = 5;
    public float BaseDelaySeconds = 1f;
    public float MaxDelaySeconds = 30f;
}

// ── DraoxClient (MonoBehaviour) ──────────────────────────

public class DraoxClient : MonoBehaviour
{
    [SerializeField] private DraoxConfig config;

    // Lifecycle
    public UniTask ConnectAsync(CancellationToken ct = default);
    public UniTask DisconnectAsync(string reason = null);

    // Auth
    public UniTask AuthenticateAsync(string userId, string token, CancellationToken ct = default);
    public string SessionId { get; }
    public bool IsAuthenticated { get; }

    // Messaging
    public UniTask SendAsync(string action, object payload = null);
    public UniTask<T> RequestAsync<T>(string action, object payload = null, CancellationToken ct = default);

    // Events
    public void Subscribe(string eventName, Action<DraoxEvent> handler);
    public void Unsubscribe(string eventName, Action<DraoxEvent> handler);
    public void SubscribeCategory(string category, Action<DraoxEvent> handler);

    // Multi-connection
    public UniTask AddConnectionAsync(ConnectionRole role, CancellationToken ct = default);

    // State
    public ClientState State { get; }

    // Events (C# events)
    public event Action OnConnected;
    public event Action<string> OnDisconnected;
    public event Action<string> OnError;
    public event Action OnAuthenticated;
    public event Action<ClientState> OnStateChanged;
}
```

### 4.3 Ví Dụ Sử Dụng (Unity C#)

```csharp
using Draox.Client;
using UnityEngine;
using Cysharp.Threading.Tasks;

public class GameNetworkManager : MonoBehaviour
{
    [SerializeField] private DraoxClient client;

    private async void Start()
    {
        client.OnConnected += () => Debug.Log("Connected!");
        client.OnDisconnected += reason => Debug.Log($"Disconnected: {reason}");

        await client.ConnectAsync();
        await client.AuthenticateAsync("player_001", playerJwtToken);

        // Gửi request
        var clans = await client.RequestAsync<ClanList>("clans.list", new { page = 1 });
        Debug.Log($"Found {clans.Total} clans");

        // Nhận event
        client.Subscribe("custom.game.match_found", OnMatchFound);

        // Thêm notification connection
        await client.AddConnectionAsync(ConnectionRole.Notification);
    }

    private void OnMatchFound(DraoxEvent evt)
    {
        var matchData = evt.Data<MatchData>();
        // Xử lý match
    }

    private void OnDestroy()
    {
        client.DisconnectAsync().Forget();
    }
}
```

### 4.4 Lưu Ý Unity

- **Thread safety**: tất cả callbacks phải được dispatch về **Unity main thread** (dùng `UniTaskScheduler.MainThread` hoặc `SynchronizationContext`)
- **WebSocket**: dùng `NativeWebSocket` (free, UPM) — hoạt động trên Android, iOS, WebGL
- **TCP**: dùng `System.Net.Sockets.TcpClient` với async await — không hỗ trợ WebGL
- **Serialization**: `Newtonsoft.Json` (Unity phổ biến) hoặc `System.Text.Json` (Unity 2021+)
- **Async**: `UniTask` (Cysharp) — tối ưu cho Unity, không dùng `Task/async` thuần
- **Inspector**: `DraoxConfig` là `[Serializable]` → cấu hình trực tiếp trong Inspector
- **UPM**: phân phối qua Unity Package Manager (git URL hoặc scoped registry)

---

## 5. Thiết Kế Shared (Cross-Platform)

### 5.1 Request/Response Correlation

```
Client gửi:  { "id": "req_abc123", "type": "request", "action": "clans.list" }
Client lưu:  pendingRequests["req_abc123"] = { resolve, reject, timer }
Server trả:  { "id": "req_abc123", "type": "response", "success": true, "data": {...} }
Client nhận: pendingRequests["req_abc123"].resolve(data)
```

Timeout: nếu sau `timeoutMs` không nhận response → `reject(TimeoutError)`.

### 5.2 Reconnect Strategy (Exponential Backoff)

```
Attempt 1: delay = baseDelay * 2^0 + jitter  → ~1s
Attempt 2: delay = baseDelay * 2^1 + jitter  → ~2s
Attempt 3: delay = baseDelay * 2^2 + jitter  → ~4s
...
Attempt N: min(delay, maxDelay)
```

Khi reconnect thành công → tự động re-authenticate bằng token đã lưu (nếu có).

### 5.3 Multi-Connection Protocol

```
Connection 1 (Primary, WS:9002):   request/response + events
Connection 2 (Notification, WS:9002): chỉ nhận server push events
Connection 3 (Streaming, TCP:9000):   raw data stream

Header mỗi kết nối gửi khi connect:
{
  "type": "bind",
  "session_id": "ses_xxx",   // session hiện tại
  "role": "notification"
}
```

### 5.4 Heartbeat

```
Client → Server (mỗi 30s): { "type": "ping", "ts": <unix_ms> }
Server → Client:            { "type": "pong", "ts": <echo> }
Nếu 2 lần ping không có pong → kích hoạt reconnect
```

---

## 6. Thứ Tự Triển Khai

### Phase 1 — JavaScript SDK (4 tuần)

| Tuần | Nội dung |
|------|---------|
| 1 | Core types, JSON serializer, WebSocket connection |
| 2 | RequestBroker (correlation), EventEmitter, DraoxClient API |
| 3 | Reconnector, multi-connection, heartbeat |
| 4 | Plugin modules (Clans, Messaging), tests, README |

### Phase 2 — Unity SDK (4 tuần)

| Tuần | Nội dung |
|------|---------|
| 1 | WebSocket (NativeWebSocket), TCP connection, serialization |
| 2 | DraoxClient MonoBehaviour, main thread dispatch, RequestBroker |
| 3 | Reconnector, multi-connection, UniTask integration |
| 4 | Plugin modules, Unity Editor inspector, package.json, tests |

### Phase 3 — Validation (1 tuần)

- Demo scene (Unity) kết nối tới local Draox server
- Demo Node.js script + browser page
- Integration tests end-to-end

---

## 7. Vị Trí Code

| Platform | Repository | Path |
|----------|-----------|------|
| JavaScript/TypeScript | Repo riêng hoặc `tools/sdk-js/` | `draox-client-js/` |
| Unity (C#) | Repo riêng hoặc `tools/sdk-unity/` | `DraoxClientUnity/` |
| Shared types (tham khảo) | Backend | `backend/crates/server-core/src/types.rs`, `event.rs` |

> **Gợi ý**: Đặt cả hai SDK vào thư mục `tools/` trong workspace hiện tại để dễ đồng bộ khi server thay đổi protocol.

---

## 8. Verification

- [ ] JS SDK: `npm test` — unit tests RequestBroker, EventEmitter, Reconnector
- [ ] JS SDK: Browser demo page kết nối WS thành công, gửi ping/pong
- [ ] JS SDK: Node.js test kết nối → authenticate → request → receive event
- [ ] Unity SDK: `[UnityTest]` — mock WebSocket, kiểm tra request correlation
- [ ] Unity SDK: Play mode demo scene — kết nối local server, hiển thị session ID
- [ ] Cả hai: Kiểm tra reconnect khi server restart
- [ ] Cả hai: Kiểm tra timeout khi request không có response

---

## 9. gRPC + Protobuf Support

> Ngày bổ sung: 2026-04-29  
> Phiên bản: v1.1

### 9.1 So Sánh JSON+WebSocket vs gRPC+Protobuf

| Tiêu chí | JSON + WebSocket | gRPC + Protobuf |
|----------|-----------------|----------------|
| Encoding | Text, verbose | Binary, compact (~3–10× nhỏ hơn) |
| Schema | Không có (implicit) | Strict (.proto — compile-time validation) |
| Streaming | Manual (server push events) | Native server-streaming RPC |
| Browser support | Native | Cần grpc-web proxy (hoặc không hỗ trợ) |
| WebGL (Unity) | Tốt (NativeWebSocket) | Không hỗ trợ — phải dùng WS fallback |
| Standalone/Mobile | Tốt | Tốt (Grpc.Net.Client) |
| Debug | Dễ (human-readable) | Cần tooling (`grpcurl`, Postman gRPC) |
| Latency | Thấp | Rất thấp (HTTP/2 multiplexing) |
| Throughput | Tốt | Cao hơn đáng kể với payload lớn |

**Khuyến nghị sử dụng:**
- **JSON + WebSocket** — browser clients, WebGL Unity, rapid prototyping
- **gRPC + Protobuf** — backend services, high-throughput game servers, mobile/standalone Unity

SDK hỗ trợ cả hai transport; chọn qua `DraoxConfig.protocol`.

---

### 9.2 Server-Side: Crate `grpc-server` Mới

**Tình trạng hiện tại:** Draox Server chưa có gRPC endpoint. `tonic = "0.12"` đã có trong workspace deps nhưng chỉ dùng cho OpenTelemetry OTLP export. Cần thêm crate riêng theo pattern tương tự `graphql-api` (Phase D).

#### 9.2.1 Định Nghĩa Protobuf — `backend/proto/draox.proto`

```protobuf
syntax = "proto3";
package draox;

// ── Unary RPCs ──────────────────────────────────────────────
service DraoxService {
  // Xác thực và nhận session ID
  rpc Authenticate(AuthRequest) returns (AuthResponse);
  // Gửi action request và nhận response
  rpc Send(DraoxRequest) returns (DraoxResponse);
}

// ── Server-Streaming RPC ────────────────────────────────────
service DraoxStreamService {
  // Đăng ký nhận events (stream vô tận cho tới khi disconnect)
  rpc Subscribe(SubscribeRequest) returns (stream DraoxEvent);
}

// ── Messages ────────────────────────────────────────────────
message AuthRequest {
  string user_id = 1;
  string token   = 2;
}

message AuthResponse {
  bool   success    = 1;
  string session_id = 2;
  string error      = 3;
}

message DraoxRequest {
  string id      = 1;   // req_<uuid> — cho correlation
  string action  = 2;   // e.g. "clans.list"
  bytes  payload = 3;   // JSON-encoded body
}

message DraoxResponse {
  string id      = 1;
  bool   success = 2;
  bytes  data    = 3;   // JSON-encoded result
  string error   = 4;
}

message SubscribeRequest {
  string          session_id = 1;
  repeated string categories = 2;  // "session", "plugin", "custom", ...
}

message DraoxEvent {
  string category  = 1;
  string name      = 2;
  bytes  data      = 3;   // JSON-encoded event data
  string timestamp = 4;   // ISO 8601
}
```

#### 9.2.2 Cấu Trúc Crate `backend/crates/grpc-server/`

```
grpc-server/
├── src/
│   ├── lib.rs          # pub fn start(addr, state) -> JoinHandle
│   ├── service.rs      # DraoxServiceImpl (Unary RPCs)
│   └── stream.rs       # DraoxStreamServiceImpl (server-streaming)
├── build.rs            # tonic_build::compile_protos
└── Cargo.toml
```

**`build.rs`:**
```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::compile_protos("../../proto/draox.proto")?;
    Ok(())
}
```

**`Cargo.toml` (grpc-server):**
```toml
[package]
name = "grpc-server"
version.workspace = true
edition.workspace = true

[dependencies]
tonic     = { workspace = true, features = ["transport"] }
prost     = { workspace = true }      # thêm vào workspace
tokio     = { workspace = true }
tokio-stream = { workspace = true }
server-core  = { path = "../server-core" }

[build-dependencies]
tonic-build = "0.12"
```

**Workspace `backend/Cargo.toml` — thêm:**
```toml
[workspace.dependencies]
prost = "0.13"
```

**Port gRPC:** `9004` — thêm vào `ListenerAddresses` struct trong `server-config`:
```rust
pub struct ListenerAddresses {
    pub tcp:  SocketAddr,   // 9000
    pub udp:  SocketAddr,   // 9001
    pub ws:   SocketAddr,   // 9002
    pub http: SocketAddr,   // 9003
    pub grpc: SocketAddr,   // 9004 (MỚI)
}
```

**Tích hợp vào `main.rs`:**
```rust
tokio::spawn(grpc_server::start(config.addresses.grpc, state.clone()));
```

---

### 9.3 JS/TS SDK — Thêm gRPC Transport

> **Giới hạn**: gRPC native không chạy trong browser. `GrpcTransport` chỉ dành cho Node.js. Browser/WebGL clients tiếp tục dùng `WsTransport`.

#### 9.3.1 Cấu Trúc Package Cập Nhật

```
draox-client-js/src/
├── core/
│   ├── DraoxClient.ts        # Chọn transport dựa trên config
│   ├── ...existing files...
├── transport/               # MỚI — tách transport layer
│   ├── Transport.ts          # interface Transport
│   ├── WsTransport.ts        # Tách từ Connection.ts
│   └── GrpcTransport.ts      # MỚI: gRPC cho Node.js
└── index.ts
```

#### 9.3.2 Transport Interface

```typescript
// transport/Transport.ts
export interface Transport {
  connect(): Promise<void>;
  authenticate(userId: string, token: string): Promise<{ sessionId: string }>;
  send(id: string, action: string, payload: unknown): Promise<{ success: boolean; data: unknown; error?: string }>;
  subscribe(categories: string[], onEvent: (e: DraoxEvent) => void): void;
  disconnect(): void;
  readonly state: 'disconnected' | 'connecting' | 'connected';
}
```

#### 9.3.3 Cập Nhật DraoxConfig

```typescript
// Thêm 'grpc' vào union type
export type DraoxProtocol = 'ws' | 'http' | 'grpc';

export interface DraoxConfig {
  host: string;
  port?: number;             // default: ws=9002, http=9003, grpc=9004
  protocol?: DraoxProtocol; // default: 'ws'
  tls?: boolean;
  timeout?: number;
  reconnect?: ReconnectConfig;
  grpc?: {
    protoPath: string;                       // đường dẫn đến draox.proto
    credentials?: 'insecure' | 'ssl';        // default: 'insecure'
  };
}
```

#### 9.3.4 GrpcTransport (Node.js only)

```typescript
// transport/GrpcTransport.ts
import * as grpc from '@grpc/grpc-js';
import * as protoLoader from '@grpc/proto-loader';

export class GrpcTransport implements Transport {
  private draoxStub: any;
  private streamStub: any;
  private activeStream?: grpc.ClientReadableStream<any>;

  constructor(private config: DraoxConfig) {}

  async connect(): Promise<void> {
    const pkgDef = protoLoader.loadSync(this.config.grpc!.protoPath, {
      keepCase: true, longs: String, enums: String, defaults: true, oneofs: true,
    });
    const proto = grpc.loadPackageDefinition(pkgDef).draox as any;
    const creds = this.config.grpc?.credentials === 'ssl'
      ? grpc.credentials.createSsl()
      : grpc.credentials.createInsecure();
    const addr = `${this.config.host}:${this.config.port ?? 9004}`;
    this.draoxStub  = new proto.DraoxService(addr, creds);
    this.streamStub = new proto.DraoxStreamService(addr, creds);
  }

  async authenticate(userId: string, token: string) {
    return new Promise<{ sessionId: string }>((resolve, reject) => {
      this.draoxStub.Authenticate({ user_id: userId, token }, (err: any, res: any) => {
        if (err || !res.success) return reject(err ?? new Error(res.error));
        resolve({ sessionId: res.session_id });
      });
    });
  }

  async send(id: string, action: string, payload: unknown) {
    const payloadBytes = Buffer.from(JSON.stringify(payload));
    return new Promise<{ success: boolean; data: unknown }>((resolve, reject) => {
      this.draoxStub.Send({ id, action, payload: payloadBytes }, (err: any, res: any) => {
        if (err) return reject(err);
        resolve({ success: res.success, data: JSON.parse(res.data.toString()), error: res.error });
      });
    });
  }

  subscribe(categories: string[], onEvent: (e: DraoxEvent) => void): void {
    this.activeStream = this.streamStub.Subscribe({
      session_id: this._sessionId,
      categories,
    });
    this.activeStream!.on('data', (msg: any) => {
      onEvent({
        type: 'event',
        category: msg.category,
        name: msg.name,
        data: JSON.parse(msg.data.toString()),
        timestamp: msg.timestamp,
      });
    });
  }

  disconnect(): void {
    this.activeStream?.cancel();
  }
}
```

**Dependencies mới (`package.json`):**
```json
{
  "dependencies": {
    "@grpc/grpc-js": "^1.10.0",
    "@grpc/proto-loader": "^0.7.0"
  },
  "optionalDependencies": {
    "@grpc/grpc-js": "^1.10.0"
  }
}
```

#### 9.3.5 Cập Nhật DraoxClient (transport selector)

```typescript
// core/DraoxClient.ts
import { WsTransport } from '../transport/WsTransport';
import { GrpcTransport } from '../transport/GrpcTransport';

export class DraoxClient {
  private transport: Transport;

  constructor(config: DraoxConfig) {
    if (config.protocol === 'grpc') {
      this.transport = new GrpcTransport(config);
    } else {
      this.transport = new WsTransport(config);
    }
  }
  // ...rest unchanged
}
```

#### 9.3.6 Ví Dụ Sử Dụng (Node.js + gRPC)

```typescript
import { DraoxClient } from 'draox-client';
import path from 'path';

const client = new DraoxClient({
  host: 'game.example.com',
  port: 9004,
  protocol: 'grpc',
  grpc: {
    protoPath: path.resolve(__dirname, '../proto/draox.proto'),
    credentials: 'ssl',
  },
});

await client.connect();
await client.authenticate({ userId: 'player_001', token: 'jwt...' });

const result = await client.request('clans.list', { page: 1 });

// Nhận events qua server-streaming
client.onCategory('custom', (event) => {
  console.log('Event:', event.name, event.data);
});
```

---

### 9.4 Unity C# SDK — Thêm gRPC Transport

> **Giới hạn**: `Grpc.Net.Client` yêu cầu .NET 5+ → Standalone, Android, iOS được hỗ trợ; **WebGL không hỗ trợ** (tự động fallback về WebSocket).

#### 9.4.1 NuGet Packages Cần Thêm

```xml
<!-- Packages/manifest.json hoặc .csproj -->
<PackageReference Include="Grpc.Net.Client"   Version="2.63.0" />
<PackageReference Include="Google.Protobuf"   Version="3.26.0" />
<PackageReference Include="Grpc.Tools"        Version="2.63.0" PrivateAssets="All" />
```

**Protobuf compile setup** — đặt file `draox.proto` vào `Assets/Draox/Proto/` và cấu hình `Grpc.Tools` để tự generate `Draox.cs` và `DraoxGrpc.cs`.

#### 9.4.2 Cập Nhật DraoxProtocol

```csharp
// Protocol/DraoxMessage.cs
public enum DraoxProtocol
{
    WebSocket,  // port 9002 (browser + WebGL compatible)
    Tcp,        // port 9000
    Grpc,       // port 9004 (standalone, mobile only)
}

[Serializable]
public class DraoxConfig
{
    public string Host = "localhost";
    public int Port = 9002;  // auto-override: WebSocket=9002, Grpc=9004
    public DraoxProtocol Protocol = DraoxProtocol.WebSocket;
    public bool UseTls = false;
    public int TimeoutMs = 10_000;
    public ReconnectConfig Reconnect = new();
}
```

#### 9.4.3 IConnection Interface

```csharp
// Core/IConnection.cs
public interface IConnection
{
    UniTask ConnectAsync(DraoxConfig config, CancellationToken ct = default);
    UniTask<AuthResponse> AuthenticateAsync(string userId, string token, CancellationToken ct = default);
    UniTask<DraoxResponse> SendAsync(DraoxRequest req, CancellationToken ct = default);
    void Subscribe(IEnumerable<string> categories, Action<DraoxEvent> onEvent);
    UniTask DisconnectAsync();
}
```

#### 9.4.4 GrpcConnection.cs

```csharp
// Core/GrpcConnection.cs
using Grpc.Net.Client;
using Cysharp.Threading.Tasks;

public class GrpcConnection : IConnection
{
    private GrpcChannel _channel;
    private Draox.DraoxService.DraoxServiceClient _client;
    private Draox.DraoxStreamService.DraoxStreamServiceClient _streamClient;
    private CancellationTokenSource _subscriptionCts;

    public async UniTask ConnectAsync(DraoxConfig config, CancellationToken ct = default)
    {
        var scheme = config.UseTls ? "https" : "http";
        var port   = config.Port > 0 ? config.Port : 9004;
        var addr   = $"{scheme}://{config.Host}:{port}";
        _channel     = GrpcChannel.ForAddress(addr);
        _client      = new Draox.DraoxService.DraoxServiceClient(_channel);
        _streamClient = new Draox.DraoxStreamService.DraoxStreamServiceClient(_channel);
    }

    public async UniTask<AuthResponse> AuthenticateAsync(string userId, string token, CancellationToken ct = default)
    {
        var req = new Draox.AuthRequest { UserId = userId, Token = token };
        var res = await _client.AuthenticateAsync(req, cancellationToken: ct);
        if (!res.Success) throw new DraoxAuthException(res.Error);
        return new AuthResponse { SessionId = res.SessionId };
    }

    public async UniTask<DraoxResponse> SendAsync(DraoxRequest req, CancellationToken ct = default)
    {
        var grpcReq = new Draox.DraoxRequest
        {
            Id = req.Id,
            Action = req.Action,
            Payload = Google.Protobuf.ByteString.CopyFromUtf8(
                Newtonsoft.Json.JsonConvert.SerializeObject(req.Payload)),
        };
        var res = await _client.SendAsync(grpcReq, cancellationToken: ct);
        return new DraoxResponse
        {
            Id      = res.Id,
            Success = res.Success,
            Data    = Newtonsoft.Json.JsonConvert.DeserializeObject(res.Data.ToStringUtf8()),
            Error   = res.Error,
        };
    }

    public void Subscribe(IEnumerable<string> categories, Action<DraoxEvent> onEvent)
    {
        _subscriptionCts = new CancellationTokenSource();
        SubscribeInternalAsync(categories, onEvent, _subscriptionCts.Token).Forget();
    }

    private async UniTaskVoid SubscribeInternalAsync(
        IEnumerable<string> categories, Action<DraoxEvent> onEvent, CancellationToken ct)
    {
        var req = new Draox.SubscribeRequest();
        req.Categories.AddRange(categories);
        var stream = _streamClient.Subscribe(req, cancellationToken: ct);

        await foreach (var msg in stream.ResponseStream.ReadAllAsync(ct))
        {
            var evt = new DraoxEvent
            {
                Category  = msg.Category,
                Name      = msg.Name,
                Data      = Newtonsoft.Json.JsonConvert.DeserializeObject(msg.Data.ToStringUtf8()),
                Timestamp = msg.Timestamp,
            };
            // Dispatch về Unity main thread
            await UniTask.SwitchToMainThread();
            onEvent(evt);
        }
    }

    public async UniTask DisconnectAsync()
    {
        _subscriptionCts?.Cancel();
        await _channel.ShutdownAsync();
    }
}
```

#### 9.4.5 Cập Nhật DraoxClient (transport selector)

```csharp
// Core/DraoxClient.cs
public class DraoxClient : MonoBehaviour
{
    [SerializeField] private DraoxConfig config;
    private IConnection _connection;

    private void Awake()
    {
#if UNITY_WEBGL && !UNITY_EDITOR
        // WebGL không hỗ trợ gRPC — force WebSocket
        if (config.Protocol == DraoxProtocol.Grpc)
        {
            Debug.LogWarning("[DraoxClient] gRPC not supported on WebGL, falling back to WebSocket");
            config.Protocol = DraoxProtocol.WebSocket;
            config.Port = 9002;
        }
#endif
        _connection = config.Protocol switch
        {
            DraoxProtocol.Grpc      => new GrpcConnection(),
            DraoxProtocol.Tcp       => new TcpConnection(),
            _                       => new WebSocketConnection(),
        };
    }
}
```

#### 9.4.6 Ví Dụ Sử Dụng (Unity + gRPC)

```csharp
using Draox.Client;
using UnityEngine;
using Cysharp.Threading.Tasks;

public class GameNetworkManager : MonoBehaviour
{
    [SerializeField] private DraoxClient client;

    // Trong Inspector: Protocol = Grpc, Host = "game.example.com", Port = 9004

    private async void Start()
    {
        await client.ConnectAsync();
        await client.AuthenticateAsync("player_001", playerJwtToken);

        // Gửi request qua gRPC unary
        var clans = await client.RequestAsync<ClanList>("clans.list", new { page = 1 });
        Debug.Log($"Found {clans.Total} clans");

        // Đăng ký events qua gRPC server-streaming
        client.SubscribeCategory("custom", OnCustomEvent);
    }

    private void OnCustomEvent(DraoxEvent evt)
    {
        if (evt.Name == "game.match_found")
        {
            var matchData = evt.Data<MatchData>();
            // Xử lý match
        }
    }
}
```

---

### 9.5 Thứ Tự Triển Khai — Phase 4 (3 Tuần)

| Tuần | Nội dung |
|------|---------|
| **1** | **Server**: Tạo `backend/proto/draox.proto` → tạo crate `grpc-server` → `build.rs` + `tonic_build` → implement `DraoxServiceImpl` (Unary) + `DraoxStreamServiceImpl` (Streaming) → thêm port 9004 vào config → tích hợp vào `main.rs` |
| **2** | **JS SDK**: Tách `WsTransport.ts`, tạo `Transport` interface → implement `GrpcTransport.ts` → cập nhật `DraoxConfig` + `DraoxClient` transport selector → unit tests Node.js |
| **3** | **Unity SDK**: Copy/generate C# từ `.proto` → implement `GrpcConnection.cs` → cập nhật `DraoxProtocol` enum + `DraoxClient.Awake()` selector → WebGL fallback → integration tests |

---

### 9.6 Verification (gRPC)

- [ ] Server: `cargo build -p grpc-server` — biên dịch thành công, không có lỗi prost/tonic
- [ ] Server: `grpcurl -plaintext localhost:9004 list` — thấy `draox.DraoxService` và `draox.DraoxStreamService`
- [ ] Server: `grpcurl -d '{"user_id":"admin","token":"..."}' localhost:9004 draox.DraoxService/Authenticate` → trả `session_id`
- [ ] JS SDK: Node.js test script kết nối gRPC → authenticate → `clans.list` request → subscribe events
- [ ] Unity SDK: Standalone build kết nối gRPC server, nhận events qua streaming
- [ ] Unity WebGL: Tự động fallback về WebSocket khi `Protocol = Grpc`
- [ ] Cả hai: so sánh payload size JSON vs Protobuf trên cùng một request
