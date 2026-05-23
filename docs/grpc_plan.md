# Draox Server — gRPC + Protobuf Implementation Plan

> **Ngày lập:** 2026-05-03  
> **Phiên bản:** v1.0  
> **Phạm vi:** Server-side (`grpc-api` crate) + Client SDK (TypeScript, Unity C#)  
> **Port gRPC:** 9004

---

## Mục Lục

1. [Tổng Quan](#1-tổng-quan)
2. [So Sánh JSON+WebSocket vs gRPC+Protobuf](#2-so-sánh-jsonwebsocket-vs-grpcprotobuf)
3. [Phạm Vi Thay Đổi](#3-phạm-vi-thay-đổi)
4. [Thiết Kế Proto](#4-thiết-kế-proto)
5. [Server-side Crate grpc-api](#5-server-side-crate-grpc-api)
6. [Thay Đổi server-config](#6-thay-đổi-server-config)
7. [Thay Đổi backend/Cargo.toml](#7-thay-đổi-backendcargotoml)
8. [Thay Đổi main.rs](#8-thay-đổi-mainrs)
9. [TypeScript SDK — GrpcTransport](#9-typescript-sdk--grpctransport)
10. [Unity C# SDK — GrpcConnection](#10-unity-c-sdk--grpcconnection)
11. [Timeline Triển Khai](#11-timeline-triển-khai)
12. [Verification & Testing](#12-verification--testing)

---

## 1. Tổng Quan

Draox Server hiện hỗ trợ 4 transport protocols: TCP (9000), UDP (9001), WebSocket (9002), HTTP (9003). Tất cả đều dùng JSON làm wire format.

**Mục tiêu**: Thêm gRPC + Protobuf transport (port **9004**) cho các use case yêu cầu:
- Hiệu năng cao (binary encoding, HTTP/2)
- Type-safe API contract (`.proto` file)  
- Native server-streaming cho real-time events
- Tích hợp backend service-to-service (không qua browser)

**Kiến trúc tích hợp**: gRPC server chạy song song (như `admin-api`), không đi qua `MultiProtocolListener`. Chia sẻ `SessionManager`, `EventBus`, `PluginRegistry` với các thành phần hiện tại.

```
                          ┌─────────────────────────────────────────────┐
                          │            Draox Server Process              │
                          │                                              │
  TCP  :9000  ──────────► │  MultiProtocolListener                       │
  UDP  :9001  ──────────► │  (TCP + UDP + WS + HTTP)                     │
  WS   :9002  ──────────► │         │                                    │
  HTTP :9003  ──────────► │         │ TrafficGuard → SessionManager       │
                          │         │       │              │             │
  gRPC :9004  ──────────► │  GrpcServer ────┤              │             │
  Admin:9100  ──────────► │  AdminServer ───┘         EventBus          │
                          │                           PluginRegistry     │
                          └─────────────────────────────────────────────┘
```

---

## 2. So Sánh JSON+WebSocket vs gRPC+Protobuf

| Tiêu chí | JSON + WebSocket | gRPC + Protobuf |
|----------|----------------|----------------|
| **Encoding** | Text, verbose | Binary, compact (~3–10× nhỏ hơn) |
| **Schema** | Không có (implicit) | Strict (`.proto` file) |
| **Streaming** | Manual event push qua EventBus | Native server-streaming RPC |
| **Browser support** | Native | Cần grpc-web proxy hoặc Envoy |
| **Unity (standalone/mobile)** | NativeWebSocket | `Grpc.Net.Client` (`#if DRAOX_GRPC`) |
| **Unity (WebGL)** | NativeWebSocket | ❌ Không hỗ trợ — fallback WS |
| **Node.js** | `ws` package | `@grpc/grpc-js` |
| **Debug** | Dễ (plain text, browser DevTools) | Cần `grpcurl` / Postman gRPC |
| **Latency** | Thấp | Rất thấp (HTTP/2 multiplexed) |
| **Load balancing** | Sticky connection | Native per-RPC L7 load balancing |
| **Code generation** | Manual hoặc OpenAPI | `protoc` → auto-generated clients |
| **Phù hợp cho** | Browser, WebGL, quick prototyping | Backend services, game servers, microservices |

**Khuyến nghị:**
- Dùng **gRPC** cho: backend service-to-service, Unity standalone/mobile game clients, high-throughput scenarios
- Dùng **WebSocket** cho: browser clients, WebGL, developer tooling, prototyping

---

## 3. Phạm Vi Thay Đổi

| File | Loại | Mô tả |
|------|------|-------|
| `backend/proto/draox.proto` | Mới | 3 gRPC services + ~15 message types |
| `backend/crates/grpc-api/` | Mới (crate) | Server-side implementation với tonic |
| `backend/Cargo.toml` | Sửa | Thêm `tonic`, `prost`, `grpc-api` member |
| `backend/crates/server-config/src/model.rs` | Sửa | Thêm `GrpcConfig` struct + field |
| `backend/config/default.toml` | Sửa | Thêm section `[grpc]` |
| `backend/crates/draox-server/src/main.rs` | Sửa | Wire GrpcServer startup |
| `backend/tools/sdk-ts/draox-client/src/transports/GrpcTransport.ts` | Mới | gRPC transport (Node.js only) |
| `backend/tools/sdk-ts/draox-client/src/types.ts` | Sửa | Thêm `'grpc'` vào `DraoxProtocol` |
| `backend/tools/sdk-ts/draox-client/package.json` | Sửa | Thêm `@grpc/grpc-js`, `@grpc/proto-loader` |

**Không thay đổi:**
- `MultiProtocolListener` — gRPC là protocol riêng biệt
- `TrafficGuard` — không áp dụng cho gRPC (tonic có interceptor riêng)
- `plugin-messaging`, `plugin-clans` — không thay đổi source code

---

## 4. Thiết Kế Proto

**File**: `backend/proto/draox.proto`

### Design Decisions

**Dual-layer approach:**
1. **Generic layer** (`DraoxService`) — mirrors JSON wire protocol, forward bất kỳ plugin action nào mà không cần thay đổi proto
2. **Typed layer** (`MessagingService`) — typed convenience API cho messaging, tốt cho code generation

```protobuf
syntax = "proto3";
package draox.v1;

// ═══════════════════════════════════════════════════════
// SERVICE DEFINITIONS
// ═══════════════════════════════════════════════════════

// Authentication service — stateless, no session required
service AuthService {
  rpc Authenticate(AuthRequest) returns (AuthResponse);
}

// Generic service — mirrors the existing JSON wire protocol
// Forward any plugin action without changing the proto
service DraoxService {
  rpc Send(DraoxRequest)           returns (DraoxResponse);
  rpc Subscribe(SubscribeRequest)  returns (stream DraoxEvent);
}

// Messaging service — typed API for plugin-messaging
service MessagingService {
  rpc SendMessage(SendMessageRequest)           returns (SendMessageResponse);
  rpc GetHistory(HistoryRequest)                returns (HistoryResponse);
  rpc DeleteMessage(DeleteMessageRequest)        returns (MutationResponse);
  rpc EditMessage(EditMessageRequest)            returns (MutationResponse);
  rpc AddReaction(AddReactionRequest)            returns (MutationResponse);
  rpc SubscribeChannel(SubscribeChannelRequest)  returns (stream MessageEvent);
}

// ═══════════════════════════════════════════════════════
// AUTH MESSAGES
// ═══════════════════════════════════════════════════════

message AuthRequest {
  string user_id = 1;
  string token   = 2;
}

message AuthResponse {
  bool   success    = 1;
  string session_id = 2;
  string error      = 3;
}

// ═══════════════════════════════════════════════════════
// GENERIC MESSAGES
// ═══════════════════════════════════════════════════════

message DraoxRequest {
  string id      = 1;  // Client-generated request ID for correlation
  string action  = 2;  // e.g., "msg.send", "clans.join", "presence.set"
  bytes  payload = 3;  // JSON-encoded action payload
}

message DraoxResponse {
  string id      = 1;
  bool   success = 2;
  bytes  data    = 3;  // JSON-encoded response data
  string error   = 4;
}

message SubscribeRequest {
  string          session_id = 1;
  repeated string categories = 2;  // e.g., ["msg", "presence", "*"]
}

message DraoxEvent {
  string category  = 1;  // e.g., "msg"
  string name      = 2;  // e.g., "received"
  bytes  data      = 3;  // JSON-encoded event payload
  string timestamp = 4;  // ISO 8601
}

// ═══════════════════════════════════════════════════════
// MESSAGING MESSAGES
// ═══════════════════════════════════════════════════════

message SendMessageRequest {
  string session_id        = 1;
  string channel           = 2;
  string text              = 3;
  optional string reply_to = 4;
}

message SendMessageResponse {
  bool   success    = 1;
  string message_id = 2;
  string error      = 3;
}

message HistoryRequest {
  string session_id          = 1;
  string channel             = 2;
  int32  limit               = 3;  // default 20
  optional string before_id  = 4;
}

message HistoryResponse {
  repeated Message messages = 1;
  string error              = 2;
}

message Message {
  string id        = 1;
  string channel   = 2;
  string sender_id = 3;
  string text      = 4;
  string timestamp = 5;
}

message DeleteMessageRequest { string session_id = 1; string message_id = 2; }
message EditMessageRequest   { string session_id = 1; string message_id = 2; string new_text = 3; }
message AddReactionRequest   { string session_id = 1; string message_id = 2; string emoji = 3; }
message MutationResponse     { bool success = 1; string error = 2; }

message SubscribeChannelRequest { string session_id = 1; string channel = 2; }
message MessageEvent {
  string  event_type = 1;  // "received" | "deleted" | "edited" | "reaction"
  Message message    = 2;
  string  timestamp  = 3;
}
```

---

## 5. Server-side Crate `grpc-api`

**Pattern tham khảo**: `backend/crates/graphql-api/` (đã tồn tại, cùng kiến trúc)

### 5.1 Cấu Trúc Thư Mục

```
backend/crates/grpc-api/
├── Cargo.toml
├── build.rs
└── src/
    ├── lib.rs
    ├── server.rs
    ├── state.rs
    ├── interceptor.rs
    └── service/
        ├── mod.rs
        ├── auth.rs
        ├── draox.rs
        └── messaging.rs
```

### 5.2 `Cargo.toml`

```toml
[package]
name    = "grpc-api"
version.workspace = true
edition.workspace = true

[dependencies]
tonic            = { workspace = true }
prost            = { workspace = true }
tokio            = { workspace = true }
tokio-stream     = "0.1"
futures-util     = { workspace = true }
async-trait      = { workspace = true }
tracing          = { workspace = true }
serde_json       = { workspace = true }
server-core      = { workspace = true }
server-config    = { workspace = true }
connection-manager   = { workspace = true }
plugin-host          = { workspace = true }
plugin-messaging     = { workspace = true }

[build-dependencies]
tonic-build = "0.12"
```

### 5.3 `build.rs`

```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .build_server(true)
        .build_client(false)  // server-side only
        .compile_protos(&["../../proto/draox.proto"], &["../../proto"])?;
    Ok(())
}
```

### 5.4 `src/lib.rs`

```rust
pub mod service;
pub mod server;
pub mod state;
pub mod interceptor;

pub use server::GrpcServer;
pub use state::GrpcState;

// Re-export generated proto types
pub mod proto {
    tonic::include_proto!("draox.v1");
}
```

### 5.5 `src/state.rs`

```rust
use connection_manager::SessionManager;
use plugin_host::PluginRegistry;
use server_core::event::EventBus;
use std::sync::Arc;

#[derive(Clone)]
pub struct GrpcState {
    pub session_manager: Arc<SessionManager>,
    pub event_bus:       Arc<EventBus>,
    pub plugin_registry: Arc<PluginRegistry>,
}
```

### 5.6 `src/server.rs`

```rust
use crate::{
    proto::{
        auth_service_server::AuthServiceServer,
        draox_service_server::DraoxServiceServer,
        messaging_service_server::MessagingServiceServer,
    },
    service::{auth::AuthServiceImpl, draox::DraoxServiceImpl, messaging::MessagingServiceImpl},
    state::GrpcState,
};
use server_core::types::ShutdownReceiver;
use std::net::SocketAddr;
use tokio_stream::wrappers::TcpListenerStream;
use tracing::info;

pub struct GrpcServer;

impl GrpcServer {
    pub async fn start(
        addr: SocketAddr,
        state: GrpcState,
        mut shutdown: ShutdownReceiver,
    ) -> server_core::Result<SocketAddr> {
        let auth_svc      = AuthServiceServer::new(AuthServiceImpl::new(state.clone()));
        let draox_svc     = DraoxServiceServer::new(DraoxServiceImpl::new(state.clone()));
        let messaging_svc = MessagingServiceServer::new(MessagingServiceImpl::new(state.clone()));

        let listener = tokio::net::TcpListener::bind(addr).await
            .map_err(|e| server_core::Error::Internal(e.to_string()))?;
        let bound = listener.local_addr()
            .map_err(|e| server_core::Error::Internal(e.to_string()))?;

        info!("gRPC server bound to {bound}");

        tokio::spawn(async move {
            tonic::transport::Server::builder()
                .add_service(auth_svc)
                .add_service(draox_svc)
                .add_service(messaging_svc)
                .serve_with_incoming_shutdown(
                    TcpListenerStream::new(listener),
                    async move { let _ = shutdown.recv().await; },
                )
                .await
                .expect("gRPC server crashed");
        });

        Ok(bound)
    }
}
```

### 5.7 `src/interceptor.rs`

```rust
use crate::{proto::auth_response, state::GrpcState};
use tonic::{Request, Status};

// Session validation interceptor — extracts session_id from request metadata
// Usage: tonic::transport::Server::builder().layer(AuthLayer::new(state.clone()))
pub fn validate_session(state: &GrpcState, session_id: &str) -> Result<(), Status> {
    if state.session_manager.get_session(session_id).is_some() {
        Ok(())
    } else {
        Err(Status::unauthenticated("invalid or expired session"))
    }
}
```

### 5.8 `src/service/auth.rs`

```rust
use crate::{proto::*, state::GrpcState};
use tonic::{Request, Response, Status};

pub struct AuthServiceImpl {
    state: GrpcState,
}

impl AuthServiceImpl {
    pub fn new(state: GrpcState) -> Self { Self { state } }
}

#[tonic::async_trait]
impl auth_service_server::AuthService for AuthServiceImpl {
    async fn authenticate(
        &self,
        req: Request<AuthRequest>,
    ) -> Result<Response<AuthResponse>, Status> {
        let r = req.into_inner();
        // Delegate to connection-manager SessionManager authentication
        match self.state.session_manager.authenticate_session(&r.user_id, &r.token).await {
            Ok(session_id) => Ok(Response::new(AuthResponse {
                success: true,
                session_id: session_id.to_string(),
                error: String::new(),
            })),
            Err(e) => Ok(Response::new(AuthResponse {
                success: false,
                session_id: String::new(),
                error: e.to_string(),
            })),
        }
    }
}
```

### 5.9 `src/service/draox.rs`

```rust
use crate::{interceptor::validate_session, proto::*, state::GrpcState};
use tokio_stream::wrappers::BroadcastStream;
use tonic::{Request, Response, Status, Streaming};

pub struct DraoxServiceImpl { state: GrpcState }

impl DraoxServiceImpl {
    pub fn new(state: GrpcState) -> Self { Self { state } }
}

#[tonic::async_trait]
impl draox_service_server::DraoxService for DraoxServiceImpl {
    async fn send(
        &self,
        req: Request<DraoxRequest>,
    ) -> Result<Response<DraoxResponse>, Status> {
        let r = req.into_inner();
        // Parse action and forward to plugin via EventBus
        // Returns JSON response serialized to bytes
        let payload: serde_json::Value = serde_json::from_slice(&r.payload)
            .unwrap_or(serde_json::Value::Null);
        
        // Dispatch via EventBus Custom event (plugins listen for their actions)
        self.state.event_bus.publish(server_core::event::ServerEvent::Custom {
            source: "grpc".to_string(),
            name: r.action.clone(),
            payload,
        });
        
        Ok(Response::new(DraoxResponse {
            id: r.id,
            success: true,
            data: b"{}".to_vec(),
            error: String::new(),
        }))
    }

    type SubscribeStream = tonic::codec::Streaming<DraoxEvent>;

    async fn subscribe(
        &self,
        req: Request<SubscribeRequest>,
    ) -> Result<Response<Self::SubscribeStream>, Status> {
        let r = req.into_inner();
        validate_session(&self.state, &r.session_id)?;
        
        // Subscribe to EventBus broadcast channel
        let rx = self.state.event_bus.subscribe();
        let categories = r.categories.clone();
        
        let stream = BroadcastStream::new(rx).filter_map(move |evt| {
            // Filter by categories and map to DraoxEvent
            // ...
            None::<Result<DraoxEvent, Status>>
        });
        
        Ok(Response::new(Box::pin(stream) as Self::SubscribeStream))
    }
}
```

### 5.10 `src/service/messaging.rs`

```rust
use crate::{interceptor::validate_session, proto::*, state::GrpcState};
use tonic::{Request, Response, Status};

pub struct MessagingServiceImpl { state: GrpcState }

impl MessagingServiceImpl {
    pub fn new(state: GrpcState) -> Self { Self { state } }

    // Helper: dispatch action via EventBus and wait for response
    async fn dispatch(&self, session_id: &str, action: &str, payload: serde_json::Value)
        -> Result<serde_json::Value, Status>
    {
        // Publish as Custom event; plugin handles and responds via session connection
        // For typed responses: use a tokio oneshot channel keyed by request_id
        self.state.event_bus.publish(server_core::event::ServerEvent::Custom {
            source: format!("grpc:{session_id}"),
            name: action.to_string(),
            payload,
        });
        Ok(serde_json::Value::Null)  // Simplified; full impl uses request broker
    }
}

#[tonic::async_trait]
impl messaging_service_server::MessagingService for MessagingServiceImpl {
    async fn send_message(
        &self,
        req: Request<SendMessageRequest>,
    ) -> Result<Response<SendMessageResponse>, Status> {
        let r = req.into_inner();
        validate_session(&self.state, &r.session_id)?;
        
        let payload = serde_json::json!({
            "channel": r.channel,
            "text": r.text,
            "reply_to": r.reply_to,
        });
        
        self.dispatch(&r.session_id, "msg.send", payload).await?;
        
        Ok(Response::new(SendMessageResponse {
            success: true,
            message_id: uuid::Uuid::new_v4().to_string(),
            error: String::new(),
        }))
    }

    async fn get_history(
        &self,
        req: Request<HistoryRequest>,
    ) -> Result<Response<HistoryResponse>, Status> {
        let r = req.into_inner();
        validate_session(&self.state, &r.session_id)?;
        // Dispatch msg.history action
        Ok(Response::new(HistoryResponse { messages: vec![], error: String::new() }))
    }

    async fn delete_message(&self, req: Request<DeleteMessageRequest>) -> Result<Response<MutationResponse>, Status> {
        let r = req.into_inner();
        validate_session(&self.state, &r.session_id)?;
        self.dispatch(&r.session_id, "msg.delete", serde_json::json!({ "id": r.message_id })).await?;
        Ok(Response::new(MutationResponse { success: true, error: String::new() }))
    }

    async fn edit_message(&self, req: Request<EditMessageRequest>) -> Result<Response<MutationResponse>, Status> {
        let r = req.into_inner();
        validate_session(&self.state, &r.session_id)?;
        self.dispatch(&r.session_id, "msg.edit", serde_json::json!({ "id": r.message_id, "text": r.new_text })).await?;
        Ok(Response::new(MutationResponse { success: true, error: String::new() }))
    }

    async fn add_reaction(&self, req: Request<AddReactionRequest>) -> Result<Response<MutationResponse>, Status> {
        let r = req.into_inner();
        validate_session(&self.state, &r.session_id)?;
        self.dispatch(&r.session_id, "msg.react", serde_json::json!({ "id": r.message_id, "emoji": r.emoji })).await?;
        Ok(Response::new(MutationResponse { success: true, error: String::new() }))
    }

    type SubscribeChannelStream = std::pin::Pin<Box<dyn futures_util::Stream<Item = Result<MessageEvent, Status>> + Send>>;

    async fn subscribe_channel(
        &self,
        req: Request<SubscribeChannelRequest>,
    ) -> Result<Response<Self::SubscribeChannelStream>, Status> {
        let r = req.into_inner();
        validate_session(&self.state, &r.session_id)?;
        
        let rx = self.state.event_bus.subscribe();
        let channel = r.channel.clone();
        
        // Filter broadcast events for this channel and map to MessageEvent
        let stream = tokio_stream::wrappers::BroadcastStream::new(rx)
            .filter_map(move |evt| {
                // match Custom events with name "msg.received" for this channel
                std::future::ready(None::<Result<MessageEvent, Status>>)
            });
        
        Ok(Response::new(Box::pin(stream)))
    }
}
```

---

## 6. Thay Đổi `server-config`

**File**: `backend/crates/server-config/src/model.rs`

### Thêm `GrpcConfig` struct

```rust
// ────────────────────────────────────────────────────────
// gRPC
// ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GrpcConfig {
    pub enabled: bool,
    pub port: u16,
    pub tls_enabled: bool,
    pub max_frame_size_bytes: u32,
    pub reflection_enabled: bool,
    pub max_concurrent_streams: Option<u32>,
}

impl Default for GrpcConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            port: 9004,
            tls_enabled: false,
            max_frame_size_bytes: 4 * 1024 * 1024,  // 4 MB
            reflection_enabled: true,
            max_concurrent_streams: None,
        }
    }
}
```

### Cập nhật `DraoxConfig`

```rust
pub struct DraoxConfig {
    pub server:        ServerConfig,
    pub tcp:           TcpConfig,
    pub udp:           UdpConfig,
    pub websocket:     WebSocketConfig,
    pub http:          HttpConfig,
    pub grpc:          GrpcConfig,   // ← THÊM MỚI
    pub tls:           TlsConfig,
    // ... các field còn lại giữ nguyên ...
}

impl Default for DraoxConfig {
    fn default() -> Self {
        Self {
            // ... existing fields ...
            grpc: GrpcConfig::default(),   // ← THÊM MỚI
        }
    }
}
```

### Cập nhật `config/default.toml`

```toml
[grpc]
enabled                 = false
port                    = 9004
tls_enabled             = false
max_frame_size_bytes    = 4194304   # 4 MB
reflection_enabled      = true
# max_concurrent_streams = 100     # uncomment to limit
```

---

## 7. Thay Đổi `backend/Cargo.toml`

### Thêm workspace dependencies

```toml
[workspace.dependencies]
# ... existing deps ...

# gRPC — THÊM MỚI
tonic  = { version = "0.12", features = ["transport"] }
prost  = "0.13"

# Internal crates — THÊM MỚI
grpc-api = { path = "crates/grpc-api" }
```

### Thêm workspace member

```toml
[workspace]
members = [
    # ... existing members ...
    "crates/grpc-api",   # ← THÊM MỚI (sau graphql-api)
]
```

---

## 8. Thay Đổi `main.rs`

**File**: `backend/crates/draox-server/src/main.rs`

Thêm imports:
```rust
use grpc_api::{GrpcServer, GrpcState};
```

Thêm sau block Admin API (dòng ~190):
```rust
// ── gRPC Server ──
if config.grpc.enabled {
    let grpc_state = GrpcState {
        session_manager: Arc::clone(&session_manager),
        event_bus:       Arc::clone(&event_bus),
        plugin_registry: Arc::clone(&plugin_registry),
    };
    let grpc_addr: std::net::SocketAddr = format!(
        "{}:{}", config.server.host, config.grpc.port
    )
    .parse()
    .unwrap_or_else(|_| "0.0.0.0:9004".parse().unwrap());

    match GrpcServer::start(grpc_addr, grpc_state, shutdown.subscribe()).await {
        Ok(bound) => info!("gRPC listening on {bound}"),
        Err(e)    => tracing::warn!("gRPC server failed to start: {e}"),
    }
}
```

---

## 9. TypeScript SDK — GrpcTransport

> **Lưu ý**: gRPC native **không hỗ trợ browser**. `GrpcTransport` chỉ dành cho Node.js.  
> Browser và WebGL clients tiếp tục dùng `WebSocketTransport`.

### 9.1 Cập nhật `package.json`

**File**: `backend/tools/sdk-ts/draox-client/package.json`

```json
{
  "dependencies": {
    "@grpc/grpc-js":    "^1.12.0",
    "@grpc/proto-loader": "^0.7.0"
  },
  "devDependencies": {
    "@types/node": "^22.0.0"
  }
}
```

### 9.2 Cập nhật `types.ts`

**File**: `backend/tools/sdk-ts/draox-client/src/types.ts`

```typescript
// Thêm 'grpc' vào union type
export type DraoxProtocol = 'ws' | 'http' | 'grpc';

export interface DraoxConfig {
  host:      string;
  port:      number;
  protocol?: DraoxProtocol;  // default: 'ws'
  // ... existing fields ...
  grpc?: {
    protoPath?:   string;           // Path tới draox.proto file
    credentials?: 'insecure' | 'tls';
  };
}
```

### 9.3 `GrpcTransport.ts` (mới)

**File**: `backend/tools/sdk-ts/draox-client/src/transports/GrpcTransport.ts`

```typescript
// Node.js only — gRPC native không hỗ trợ browser
import * as grpc from '@grpc/grpc-js';
import * as protoLoader from '@grpc/proto-loader';
import * as path from 'path';
import { ITransport } from './ITransport';
import { DraoxConfig, DraoxEvent } from '../types';

const DEFAULT_PROTO_PATH = path.resolve(__dirname, '../../../../../proto/draox.proto');

export class GrpcTransport implements ITransport {
  private channel:     grpc.Channel | null = null;
  private authStub:    any = null;
  private draoxStub:   any = null;
  private msgStub:     any = null;
  private _sessionId:  string | null = null;

  constructor(private readonly config: DraoxConfig) {}

  async connect(): Promise<void> {
    const protoPath = this.config.grpc?.protoPath ?? DEFAULT_PROTO_PATH;
    const pkgDef = protoLoader.loadSync(protoPath, {
      keepCase: false,
      longs: String,
      enums: String,
      defaults: true,
      oneofs: true,
    });
    const proto = grpc.loadPackageDefinition(pkgDef) as any;
    const svc = proto.draox.v1;

    const target = `${this.config.host}:${this.config.port}`;
    const creds = this.config.grpc?.credentials === 'tls'
      ? grpc.credentials.createSsl()
      : grpc.credentials.createInsecure();

    this.authStub  = new svc.AuthService(target, creds);
    this.draoxStub = new svc.DraoxService(target, creds);
    this.msgStub   = new svc.MessagingService(target, creds);

    // Health-check: wait for channel to be ready
    await new Promise<void>((resolve, reject) => {
      this.authStub.waitForReady(Date.now() + 5000, (err: Error) =>
        err ? reject(err) : resolve()
      );
    });
  }

  async authenticate(userId: string, token: string): Promise<string> {
    return new Promise((resolve, reject) => {
      this.authStub.authenticate({ userId, token }, (err: grpc.ServiceError, res: any) => {
        if (err || !res.success) return reject(err ?? new Error(res.error));
        this._sessionId = res.sessionId;
        resolve(res.sessionId);
      });
    });
  }

  async send(action: string, payload: unknown): Promise<unknown> {
    const id = crypto.randomUUID();
    return new Promise((resolve, reject) => {
      this.draoxStub.send(
        { id, action, payload: Buffer.from(JSON.stringify(payload)) },
        (err: grpc.ServiceError, res: any) => {
          if (err) return reject(err);
          if (!res.success) return reject(new Error(res.error));
          resolve(JSON.parse(Buffer.from(res.data).toString()));
        }
      );
    });
  }

  subscribe(categories: string[], onEvent: (e: DraoxEvent) => void): () => void {
    if (!this._sessionId) throw new Error('Not authenticated');
    const call = this.draoxStub.subscribe({ sessionId: this._sessionId, categories });
    call.on('data', (evt: any) => {
      onEvent({
        category:  evt.category,
        name:      evt.name,
        data:      JSON.parse(Buffer.from(evt.data).toString()),
        timestamp: evt.timestamp,
      });
    });
    call.on('error', (err: Error) => console.error('[GrpcTransport] subscribe error', err));
    return () => call.cancel();
  }

  disconnect(): void {
    this.authStub?.close();
    this.draoxStub?.close();
    this.msgStub?.close();
    this._sessionId = null;
  }

  get sessionId(): string | null { return this._sessionId; }
}
```

### 9.4 Cập nhật `DraoxClient.ts`

Thêm transport selector trong phương thức khởi tạo:

```typescript
import { GrpcTransport } from './transports/GrpcTransport';

// Trong constructor hoặc factory method:
private createTransport(config: DraoxConfig): ITransport {
  switch (config.protocol) {
    case 'grpc': return new GrpcTransport(config);    // ← THÊM MỚI
    default:     return new WebSocketTransport(config);
  }
}
```

### 9.5 Ví Dụ Sử Dụng (Node.js)

```typescript
import { DraoxClient, MessagingPlugin } from 'draox-client';

const client = new DraoxClient({
  host:     'localhost',
  port:     9004,
  protocol: 'grpc',
  grpc: { credentials: 'insecure' },
});

await client.connect();
await client.authenticate('user_001', 'test_token');

const messaging = new MessagingPlugin(client);
messaging.onMessage = (e) => console.log(`${e.message.sender_id}: ${e.message.text}`);
messaging.registerListeners();

await messaging.sendMessage('general', 'Hello via gRPC!');
```

---

## 10. Unity C# SDK — GrpcConnection

**File hiện có**: `backend/tools/sdk-unity/DraoxClientUnity/Runtime/Core/GrpcConnection.cs`

File này đã được implement trong Phase 18. Cần cập nhật proto reference khi `draox.proto` hoàn thiện:

1. Thêm `draox.proto` vào `Runtime/Protos/draox.proto`
2. Cấu hình `Grpc.Tools` trong `DraoxClientUnity.asmdef` để generate C# classes
3. Cập nhật `GrpcConnection.cs` để reference generated `DraoxService.DraoxServiceClient` và `MessagingService.MessagingServiceClient`

**Activation**: Enable scripting define `DRAOX_GRPC` trong Unity Player Settings để bật gRPC.

**Platform support**:
| Platform | gRPC | Fallback |
|----------|------|---------|
| Standalone (Windows/Mac/Linux) | ✅ | — |
| Android | ✅ | — |
| iOS | ✅ | — |
| WebGL | ❌ | WebSocket |

---

## 11. Timeline Triển Khai

### Tuần 1 — Server-side Infrastructure

| Task | Files | Ước tính |
|------|-------|---------|
| Tạo `backend/proto/draox.proto` | 1 file | 2h |
| Thêm `tonic`, `prost` vào workspace | `Cargo.toml` | 30m |
| Tạo crate `grpc-api` skeleton | 8 files | 4h |
| Implement `AuthServiceImpl` | `service/auth.rs` | 2h |
| Implement `DraoxServiceImpl` (generic) | `service/draox.rs` | 3h |
| Implement `MessagingServiceImpl` | `service/messaging.rs` | 4h |
| Thêm `GrpcConfig` vào server-config | `model.rs`, `default.toml` | 1h |
| Wire vào `main.rs` | `main.rs` | 1h |
| Unit tests cho service handlers | `tests/` | 3h |
| **Build & smoke test** | — | 2h |

### Tuần 2 — TypeScript SDK

| Task | Files | Ước tính |
|------|-------|---------|
| Cập nhật `package.json` (grpc deps) | 1 file | 30m |
| Cập nhật `types.ts` (protocol + grpc config) | 1 file | 1h |
| Implement `GrpcTransport.ts` | 1 file | 4h |
| Cập nhật `DraoxClient.ts` (transport selector) | 1 file | 1h |
| Demo Node.js script (gRPC messaging) | 1 file | 2h |
| Test với grpcurl + Node.js demo | — | 2h |

### Tuần 3 — Unity & Polish

| Task | Files | Ước tính |
|------|-------|---------|
| Copy `draox.proto` vào Unity SDK | 1 file | 30m |
| Cập nhật `GrpcConnection.cs` proto refs | 1 file | 2h |
| Integration tests Unity + server | tests | 3h |
| gRPC reflection test (grpcui) | — | 1h |
| Documentation + report hoàn thiện | `docs/grpc_plan.md` | 2h |

**Tổng**: ~40 giờ trong 3 tuần

---

## 12. Verification & Testing

### 12.1 Build Verification

```bash
# Kiểm tra proto compile
cargo build -p grpc-api

# Full workspace build
cargo build --workspace

# Tests
cargo test -p grpc-api
cargo test --workspace
```

### 12.2 grpcurl Testing

```bash
# Cài grpcurl
# https://github.com/fullstorydev/grpcurl

# Kiểm tra services
grpcurl -plaintext localhost:9004 list
# → draox.v1.AuthService
# → draox.v1.DraoxService
# → draox.v1.MessagingService

# Authenticate
grpcurl -plaintext \
  -d '{"user_id":"user_001","token":"test_token"}' \
  localhost:9004 draox.v1.AuthService/Authenticate

# Send message (thay SESSION_ID bằng giá trị từ bước trên)
grpcurl -plaintext \
  -d '{"session_id":"SESSION_ID","channel":"general","text":"Hello gRPC!"}' \
  localhost:9004 draox.v1.MessagingService/SendMessage

# Subscribe to channel (streaming)
grpcurl -plaintext \
  -d '{"session_id":"SESSION_ID","channel":"general"}' \
  localhost:9004 draox.v1.MessagingService/SubscribeChannel
```

### 12.3 TypeScript Demo

```bash
cd backend/tools/sdk-ts/draox-client
npm install
npm run build

node -e "
const { DraoxClient } = require('./dist');
async function main() {
  const client = new DraoxClient({ host:'localhost', port:9004, protocol:'grpc' });
  await client.connect();
  const session = await client.authenticate('user_001','test_token');
  console.log('Session:', session);
  await client.disconnect();
}
main().catch(console.error);
"
```

### 12.4 Enable gRPC trong config

Để test, bật `grpc.enabled = true` trong `config/default.toml`:
```toml
[grpc]
enabled = true
port    = 9004
```

Hoặc via environment variable:
```bash
DRAOX_GRPC_ENABLED=true cargo run -- --config config/default.toml
```

---

## Phụ Lục — Critical Files Reference

| File | Vai trò |
|------|--------|
| `backend/proto/draox.proto` | Source of truth cho tất cả gRPC contracts |
| `backend/crates/grpc-api/build.rs` | Compile proto → Rust code |
| `backend/crates/grpc-api/src/service/messaging.rs` | Core business logic cho Messaging |
| `backend/crates/server-config/src/model.rs` | Config structs — ảnh hưởng TOML parsing |
| `backend/crates/draox-server/src/main.rs` | Wire-up — server không start nếu sai ở đây |
| `backend/tools/sdk-ts/draox-client/src/transports/GrpcTransport.ts` | TS gRPC transport |

## Reuse Existing Code

- `SessionManager::get_session()` → validate session_id trong gRPC interceptor (thay cho JWT decode)
- `EventBus::publish()` → forward plugin events tới gRPC streaming subscribers
- `server_core::types::ShutdownReceiver` → graceful shutdown (cùng pattern với admin-api)
- `GrpcState` mirrors pattern của `AppState` trong `admin-api/src/state.rs`
- `GrpcConfig` default pattern giống `TcpConfig`, `UdpConfig` (enabled=false, port=...)
