# Draox Client SDK — Implementation Report

**Version**: 1.0  
**Date**: 2026-05-03  
**Scope**: All client SDKs implemented in `backend/tools/`

---

## Table of Contents

1. [Overview](#1-overview)
2. [Wire Protocol Specification](#2-wire-protocol-specification)
3. [SDK Comparison Table](#3-sdk-comparison-table)
4. [SDK 1 — Unity C# (`sdk-unity/`)](#4-sdk-1--unity-c-sdk-unity)
5. [SDK 2 — .NET WPF (`sdk-wpf/`)](#5-sdk-2--net-wpf-sdk-wpf)
6. [SDK 3 — TypeScript/Node.js (`sdk-ts/`)](#6-sdk-3--typescriptnodejs-sdk-ts)
7. [Tool — SDK Code Generator (`sdk-gen/`)](#7-tool--sdk-code-generator-sdk-gen)
8. [Plugin API Reference](#8-plugin-api-reference)
9. [Advanced Topics](#9-advanced-topics)

---

## 1. Overview

The Draox server exposes a real-time socket protocol over **WebSocket (port 9002)** and **TCP (port 9000)**. This report documents all client SDKs that implement the protocol, each targeting a different runtime:

```
backend/tools/
├── sdk-unity/      Unity C# — game clients (WebGL, mobile, desktop, console)
├── sdk-wpf/        .NET C# — desktop/WPF applications
├── sdk-ts/         TypeScript — Node.js server-to-server or CLI clients
└── sdk-gen/        Rust CLI — generates REST API clients from OpenAPI spec
```

All three SDK packages share the same:
- JSON wire protocol format
- Request/response correlation pattern (per-request UUID, timeout)
- Exponential backoff reconnect logic
- Plugin architecture (MessagingPlugin, and more)
- Heartbeat mechanism (30 s ping/pong, 2 missed → reconnect)

---

## 2. Wire Protocol Specification

All messages are JSON-encoded strings sent over WebSocket frames or TCP newline-delimited lines.

### 2.1 Client → Server: Request

```json
{
  "id":      "req_a1b2c3d4e5f6",
  "type":    "request",
  "action":  "msg.send",
  "payload": {
    "channel_id": "general",
    "text": "Hello, world!"
  }
}
```

| Field     | Type   | Description                              |
|-----------|--------|------------------------------------------|
| `id`      | string | Unique request ID (UUID without dashes)  |
| `type`    | string | Always `"request"`                       |
| `action`  | string | Action name (`"auth"`, `"msg.send"`, …)  |
| `payload` | object | Action-specific parameters               |

### 2.2 Server → Client: Response

```json
{
  "id":      "req_a1b2c3d4e5f6",
  "type":    "response",
  "success": true,
  "data":    { "message": { "id": "msg_xyz", "text": "Hello, world!" } },
  "error":   null
}
```

| Field     | Type    | Description                                  |
|-----------|---------|----------------------------------------------|
| `id`      | string  | Mirrors the request ID for correlation       |
| `type`    | string  | Always `"response"`                          |
| `success` | boolean | `true` on success, `false` on error          |
| `data`    | object  | Response payload (present when success=true) |
| `error`   | string  | Error message (present when success=false)   |

### 2.3 Server → Client: Event (Push)

```json
{
  "type":      "event",
  "category":  "msg",
  "name":      "received",
  "data":      { "message": { "id": "msg_abc", "sender_id": "user_002", "text": "Hi!" } },
  "timestamp": "2026-05-03T10:15:30Z"
}
```

| Field       | Type   | Description                                   |
|-------------|--------|-----------------------------------------------|
| `type`      | string | Always `"event"`                              |
| `category`  | string | Plugin namespace: `"msg"`, `"clan"`, `"presence"` |
| `name`      | string | Event name: `"received"`, `"deleted"`, …      |
| `data`      | object | Event-specific payload                        |
| `timestamp` | string | ISO 8601 UTC timestamp                        |

### 2.4 Authentication

```json
// Request
{ "id": "req_xxx", "type": "request", "action": "auth",
  "payload": { "user_id": "user_001", "token": "secret_token" } }

// Response
{ "id": "req_xxx", "type": "response", "success": true,
  "data": { "session_id": "sess_abcdef..." } }
```

### 2.5 Heartbeat

```json
// Client → Server
{ "type": "ping", "ts": 1746262530000 }

// Server → Client
{ "type": "pong" }
```

Clients send a ping every 30 seconds. If 2 consecutive pings receive no pong, the client reconnects.

### 2.6 Multi-Connection Bind (Unity only)

```json
{ "type": "bind", "session_id": "sess_abcdef...", "role": "notification" }
```

Roles: `"primary"`, `"notification"`, `"control"`, `"streaming"`.

---

## 3. SDK Comparison Table

| Feature                    | Unity C#         | .NET WPF C#      | TypeScript        |
|----------------------------|------------------|------------------|-------------------|
| Runtime                    | Unity Engine     | .NET 8 desktop   | Node.js ≥18       |
| Async model                | UniTask          | Task/async-await | Promise/async     |
| WebSocket library          | NativeWebSocket  | System.Net.WebSockets | ws 8.x       |
| TCP support                | Yes              | Yes              | No (WS only)      |
| gRPC support               | Optional (define)| No               | No                |
| Auto-reconnect             | Yes              | Yes              | Yes               |
| Heartbeat (30 s)           | Yes              | Yes              | Yes               |
| Request timeout            | Yes (10 s)       | Yes (10 s)       | Yes (10 s)        |
| MessagingPlugin            | Yes              | Yes              | Yes               |
| ClansPlugin                | Yes              | No               | No                |
| PresencePlugin             | Yes              | No               | No                |
| Multi-connection sessions  | Yes              | No               | No                |
| UI thread dispatch         | Unity MainThread | SynchronizationContext | N/A         |
| WebGL support              | Yes (WS only)    | No               | No                |
| Test suite included        | Yes              | No               | No                |
| NuGet / npm package        | UPM (package.json) | Manual reference | npm workspace   |

---

## 4. SDK 1 — Unity C# (`sdk-unity/`)

### 4.1 Directory Structure

```
sdk-unity/
├── DraoxClientUnity/                   # Unity Package (UPM)
│   ├── package.json                    # UPM manifest
│   ├── Runtime/
│   │   ├── DraoxClientUnity.asmdef
│   │   ├── Core/
│   │   │   ├── DraoxClient.cs          # Main MonoBehaviour client
│   │   │   ├── IConnection.cs          # Transport abstraction (UniTask)
│   │   │   ├── WebSocketConnection.cs  # NativeWebSocket transport
│   │   │   ├── TcpConnection.cs        # TcpClient transport
│   │   │   ├── GrpcConnection.cs       # gRPC transport (#if DRAOX_GRPC)
│   │   │   ├── RequestBroker.cs        # Request/response correlation
│   │   │   ├── Reconnector.cs          # Exponential backoff
│   │   │   └── SessionManager.cs       # Multi-connection pool
│   │   ├── Protocol/
│   │   │   ├── DraoxMessage.cs         # Config, enums, DTOs, wire types
│   │   │   └── Serializer.cs           # Newtonsoft.Json parser
│   │   └── Plugins/
│   │       ├── MessagingPlugin.cs      # msg.* actions + events
│   │       ├── ClansPlugin.cs          # clan.* actions + events
│   │       └── PresencePlugin.cs       # presence.* actions + events
│   ├── Editor/
│   │   ├── DraoxSettingsEditor.cs      # Inspector settings editor
│   │   └── DraoxClientUnity.Editor.asmdef
│   └── Tests/
│       └── Runtime/
│           ├── DraoxClientTests.cs
│           └── DraoxClientUnity.Tests.asmdef
└── DraoxDemo/                          # Unity demo project
    └── Assets/Scripts/
        ├── DemoManager.cs              # Bootstrap, tab switching
        ├── ConnectionPanel.cs          # Connect/disconnect UI
        ├── AuthPanel.cs                # Login UI
        ├── MessagingPanel.cs           # Chat UI
        ├── ClansPanel.cs               # Clans UI
        ├── PresencePanel.cs            # Presence UI
        └── RequestPanel.cs             # Raw request inspector
```

### 4.2 Prerequisites

- Unity 2022.3 LTS or later
- `NativeWebSocket` package (GitHub: endel/NativeWebSocket)
- `UniTask` package (GitHub: Cysharp/UniTask)
- **Optional (gRPC)**: `Grpc.Net.Client`, `Google.Protobuf`, `Grpc.Tools` + define `DRAOX_GRPC`

### 4.3 Installation

**Via UPM (Git URL)**:
1. Open *Window → Package Manager*
2. Click `+` → *Add package from git URL*
3. Enter: `https://github.com/your-org/draox-sdk-unity.git?path=DraoxClientUnity`

**Manual**:
1. Copy `DraoxClientUnity/` into `Assets/Packages/DraoxClientUnity/`
2. Install NativeWebSocket and UniTask dependencies

### 4.4 Quick Start

```csharp
using Draox.Unity;
using Draox.Unity.Plugins;
using Cysharp.Threading.Tasks;
using UnityEngine;

public class ChatController : MonoBehaviour
{
    [SerializeField] private DraoxClient client;

    private MessagingPlugin _messaging;

    private async void Start()
    {
        await client.ConnectAsync();
        await client.AuthenticateAsync("user_001", "test_token");

        _messaging = new MessagingPlugin(client);
        _messaging.OnMessage += OnMessageReceived;
        _messaging.RegisterListeners();

        var history = await _messaging.GetHistoryAsync("general", 20);
        foreach (var msg in history.Messages)
            Debug.Log($"{msg.SenderId}: {msg.Text}");
    }

    private void OnMessageReceived(MessageReceivedEvent evt)
    {
        Debug.Log($"[{evt.Message.ChannelId}] {evt.Message.SenderId}: {evt.Message.Text}");
    }

    public async UniTaskVoid SendMessage(string text)
    {
        var resp = await _messaging.SendMessageAsync("general", text);
        Debug.Log($"Sent: {resp.Message.Id}");
    }
}
```

### 4.5 DraoxClient API

**Configuration** (`DraoxConfig`):

| Property                  | Type             | Default           | Description              |
|---------------------------|------------------|-------------------|--------------------------|
| `Host`                    | string           | `"localhost"`     | Server hostname          |
| `Port`                    | int              | `9002`            | Server port              |
| `Protocol`                | DraoxProtocol    | `WebSocket`       | `WebSocket`, `Tcp`, `Grpc` |
| `UseTls`                  | bool             | `false`           | TLS/SSL                  |
| `TimeoutMs`               | int              | `10000`           | Request timeout (ms)     |
| `HeartbeatIntervalSeconds`| int              | `30`              | Ping interval            |
| `Reconnect.Enabled`       | bool             | `true`            | Auto-reconnect           |
| `Reconnect.MaxAttempts`   | int              | `5`               | Max reconnect tries      |
| `Reconnect.BaseDelaySeconds` | float         | `1.0`             | Initial retry delay      |
| `Reconnect.MaxDelaySeconds`  | float         | `30.0`            | Max retry delay          |

**Properties**:

| Property          | Type        | Description                       |
|-------------------|-------------|-----------------------------------|
| `State`           | ClientState | Current connection state          |
| `SessionId`       | string?     | Session ID (null if not authed)   |
| `IsAuthenticated` | bool        | Whether authenticated             |

**Methods**:

| Method                                                      | Returns         | Description                          |
|-------------------------------------------------------------|-----------------|--------------------------------------|
| `ConnectAsync(ct?)`                                         | `UniTask`       | Establish connection                 |
| `DisconnectAsync(reason?)`                                  | `UniTask`       | Close connection                     |
| `AuthenticateAsync(userId, token, ct?)`                     | `UniTask`       | Authenticate, stores session ID      |
| `SendAsync(action, payload?, ct?)`                          | `UniTask`       | Fire-and-forget request              |
| `RequestAsync<T>(action, payload?, ct?)`                    | `UniTask<T?>`   | Request with typed response          |
| `Subscribe(eventName, handler)`                             | `void`          | Subscribe to `"Category.Name"` key   |
| `Unsubscribe(eventName, handler)`                           | `void`          | Unsubscribe by exact key             |
| `SubscribeCategory(category, handler)`                      | `void`          | Subscribe to all events in category  |
| `UnsubscribeCategory(category, handler)`                    | `void`          | Unsubscribe from category            |
| `AddConnectionAsync(role, ct?)`                             | `UniTask`       | Add secondary connection with role   |

**Events**:

| Event           | Type                    | When fired                     |
|-----------------|-------------------------|--------------------------------|
| `OnConnected`   | `Action`                | WebSocket/TCP connected        |
| `OnDisconnected`| `Action<string>`        | Connection lost (reason)       |
| `OnError`       | `Action<string>`        | Protocol error                 |
| `OnAuthenticated`| `Action`               | Auth success                   |
| `OnStateChanged`| `Action<ClientState>`   | State transitions              |

**WebGL note**: Call `client.DispatchMessageQueue()` from `Update()` when targeting WebGL.

---

## 5. SDK 2 — .NET WPF (`sdk-wpf/`)

### 5.1 Directory Structure

```
sdk-wpf/
├── DraoxClientWpf/                     # Class library (no Windows dependency)
│   ├── DraoxClientWpf.csproj           # net8.0, System.Text.Json
│   └── Core/
│       ├── DraoxConfig.cs              # Config, enums, wire DTOs
│       ├── IConnection.cs              # Transport interface (Task-based)
│       ├── WebSocketConnection.cs      # System.Net.WebSockets
│       ├── TcpConnection.cs            # TcpClient, line-delimited JSON
│       ├── Serializer.cs               # System.Text.Json parser
│       ├── RequestBroker.cs            # ConcurrentDictionary correlation
│       ├── Reconnector.cs              # Exponential backoff (Task)
│       └── DraoxClient.cs              # Main client class
│   └── Plugins/
│       └── MessagingPlugin.cs          # msg.* actions + events
└── DraoxWpfDemo/                       # WPF demo application
    ├── DraoxWpfDemo.csproj             # net8.0-windows, UseWPF
    ├── App.xaml / App.xaml.cs
    └── MainWindow.xaml / .xaml.cs      # Dark-themed chat UI
```

### 5.2 Prerequisites

- .NET 8.0 SDK or later
- Visual Studio 2022 or VS Code with C# extension

### 5.3 Project Setup

Add project reference in your `.csproj`:

```xml
<ItemGroup>
  <ProjectReference Include="..\DraoxClientWpf\DraoxClientWpf.csproj" />
</ItemGroup>
```

### 5.4 Quick Start

```csharp
using Draox.Client;
using Draox.Client.Plugins;

// Create and configure client
var client = new DraoxClient(new DraoxConfig
{
    Host = "localhost",
    Port = 9002,
    Protocol = DraoxProtocol.WebSocket,
    TimeoutMs = 10_000,
    Reconnect = new ReconnectConfig { Enabled = true, MaxAttempts = 5 }
});

// Event handlers
client.OnConnected    += () => Console.WriteLine("Connected");
client.OnDisconnected += reason => Console.WriteLine($"Disconnected: {reason}");
client.OnStateChanged += state => Console.WriteLine($"State → {state}");

// Connect and authenticate
await client.ConnectAsync();
await client.AuthenticateAsync("user_001", "test_token");

// Messaging plugin
var messaging = new MessagingPlugin(client);
messaging.OnMessage += evt =>
{
    var msg = evt.Message;
    Console.WriteLine($"[{msg.ChannelId}] {msg.SenderId}: {msg.Text}");
};
messaging.RegisterListeners();

// Send a message
var resp = await messaging.SendMessageAsync("general", "Hello from WPF!");
Console.WriteLine($"Sent: {resp.Message.Id}");

// Load history
var history = await messaging.GetHistoryAsync("general", 20);
foreach (var m in history.Messages.Reverse())
    Console.WriteLine($"  {m.SenderId}: {m.Text}");

// Cleanup
await client.DisconnectAsync();
client.Dispose();
```

### 5.5 DraoxClient API

**Configuration** (`DraoxConfig`):

| Property                   | Type          | Default     | Description            |
|----------------------------|---------------|-------------|------------------------|
| `Host`                     | string        | `"localhost"` | Server hostname      |
| `Port`                     | int           | `9002`      | Server port            |
| `Protocol`                 | DraoxProtocol | `WebSocket` | `WebSocket` or `Tcp`   |
| `UseTls`                   | bool          | `false`     | TLS/SSL                |
| `TimeoutMs`                | int           | `10000`     | Request timeout (ms)   |
| `HeartbeatIntervalSeconds` | int           | `30`        | Ping interval          |
| `Reconnect`                | ReconnectConfig | (see below)| Reconnect options     |

**Methods**:

| Method                                             | Returns    | Description                        |
|----------------------------------------------------|------------|------------------------------------|
| `ConnectAsync(ct?)`                                | `Task`     | Establish connection               |
| `DisconnectAsync(reason?)`                         | `Task`     | Close connection                   |
| `AuthenticateAsync(userId, token, ct?)`            | `Task`     | Authenticate with server           |
| `SendAsync(action, payload?, ct?)`                 | `Task`     | Fire-and-forget                    |
| `RequestAsync<T>(action, payload?, ct?)`           | `Task<T?>` | Request with typed response        |
| `Subscribe(eventName, handler)`                    | `void`     | Subscribe to `"Category.Name"` key |
| `Unsubscribe(eventName, handler)`                  | `void`     | Unsubscribe                        |
| `SubscribeCategory(category, handler)`             | `void`     | Subscribe to category events       |
| `UnsubscribeCategory(category, handler)`           | `void`     | Unsubscribe from category          |
| `Dispose()`                                        | `void`     | Disconnect and release resources   |

**Events**:

| Event            | Signature               | When fired               |
|------------------|-------------------------|--------------------------|
| `OnConnected`    | `Action`                | TCP/WS connected         |
| `OnDisconnected` | `Action<string>`        | Connection closed         |
| `OnError`        | `Action<string>`        | Protocol error           |
| `OnAuthenticated`| `Action`                | Auth success             |
| `OnStateChanged` | `Action<ClientState>`   | State transition         |

**Thread safety**: All events are dispatched on the `SynchronizationContext` captured at construction time (typically the UI thread). This means WPF UI updates in event handlers are safe without `Dispatcher.Invoke`.

### 5.6 WPF Demo

Run the demo application (`DraoxWpfDemo`):

1. Start the Draox server: `cargo run -- --config config/default.toml`
2. Open `sdk-wpf/DraoxWpfDemo.sln` in Visual Studio
3. Press **F5** to run
4. Enter host/port in the sidebar, click **Connect**
5. Enter user ID and token, click **Auth**
6. Type messages in the input field and press Enter or **Send**

---

## 6. SDK 3 — TypeScript/Node.js (`sdk-ts/`)

### 6.1 Directory Structure

```
sdk-ts/
├── draox-client/               # SDK library package
│   ├── package.json            # ws@8, @types/ws, typescript@5
│   ├── tsconfig.json
│   └── src/
│       ├── index.ts            # Public exports
│       ├── types.ts            # DraoxConfig, enums, DraoxEvent, WireResponse
│       ├── Serializer.ts       # JSON parse/serialize
│       ├── RequestBroker.ts    # Map<id, {resolve, reject, timer}>
│       ├── Reconnector.ts      # Exponential backoff with AbortSignal
│       ├── DraoxClient.ts      # EventEmitter-based main client
│       ├── transports/
│       │   ├── ITransport.ts   # Transport interface
│       │   └── WebSocketTransport.ts  # ws package transport
│       └── plugins/
│           └── MessagingPlugin.ts     # msg.* actions + events
└── draox-ts-demo/              # CLI chat demo
    ├── package.json
    ├── tsconfig.json
    └── src/index.ts            # readline + ANSI colors
```

### 6.2 Prerequisites

- Node.js 18 or later (requires `crypto.randomUUID()`)
- npm or yarn

### 6.3 Installation

```bash
# In sdk-ts/draox-client
npm install

# In sdk-ts/draox-ts-demo
npm install
```

Or copy `draox-client/` into your project as a local package:

```json
{
  "dependencies": {
    "draox-client": "file:./draox-client"
  }
}
```

### 6.4 Quick Start

```typescript
import { DraoxClient, MessagingPlugin } from 'draox-client';

// Create client
const client = new DraoxClient({
  host: 'localhost',
  port: 9002,
  protocol: 'ws',
  timeoutMs: 10_000,
  reconnect: { enabled: true, maxAttempts: 5 }
});

// Listen to lifecycle events
client.on('connected',     ()      => console.log('Connected'));
client.on('disconnected',  (r)     => console.log('Disconnected:', r));
client.on('stateChanged',  (state) => console.log('State:', state));
client.on('authenticated', ()      => console.log('Authenticated'));

// Connect and authenticate
await client.connect();
await client.authenticate('user_001', 'test_token');
console.log('Session:', client.sessionId);

// Use MessagingPlugin
const messaging = new MessagingPlugin(client);
messaging.onMessage = (evt) => {
  const m = evt.message;
  console.log(`[${m.channel_id}] ${m.sender_id}: ${m.text}`);
};
messaging.registerListeners();

// Load history
const history = await messaging.getHistory('general', 20);
for (const m of [...(history.messages ?? [])].reverse())
  console.log(`  ${m.sender_id}: ${m.text}`);

// Send a message
const resp = await messaging.sendMessage('general', 'Hello from TypeScript!');
console.log('Sent:', resp.message.id);

// Cleanup
messaging.unregisterListeners();
await client.disconnect();
```

### 6.5 DraoxClient API

**Configuration** (`DraoxConfig`):

| Property              | Type             | Default       | Description                 |
|-----------------------|------------------|---------------|-----------------------------|
| `host`                | string           | `"localhost"` | Server hostname             |
| `port`                | number           | `9002`        | Server port                 |
| `protocol`            | `'ws' \| 'tcp'`  | `'ws'`        | Transport protocol          |
| `useTls`              | boolean          | `false`       | TLS/WSS                     |
| `timeoutMs`           | number           | `10000`       | Request timeout (ms)        |
| `heartbeatIntervalMs` | number           | `30000`       | Ping interval (ms)          |
| `reconnect`           | ReconnectConfig  | (see below)   | Reconnect options           |

**Methods**:

| Method                                          | Returns           | Description                        |
|-------------------------------------------------|-------------------|------------------------------------|
| `connect()`                                     | `Promise<void>`   | Establish connection               |
| `disconnect(reason?)`                           | `Promise<void>`   | Close connection                   |
| `authenticate(userId, token)`                   | `Promise<void>`   | Authenticate with server           |
| `send(action, payload?)`                        | `Promise<void>`   | Fire-and-forget                    |
| `request<T>(action, payload?)`                  | `Promise<T>`      | Request with typed response        |
| `subscribe(eventName, handler)`                 | `void`            | Subscribe to `"Category.Name"` key |
| `unsubscribe(eventName, handler)`               | `void`            | Unsubscribe                        |
| `subscribeCategory(category, handler)`          | `void`            | Subscribe to all category events   |
| `unsubscribeCategory(category, handler)`        | `void`            | Unsubscribe from category          |

**Events** (EventEmitter):

| Event            | Args                  | When fired                  |
|------------------|-----------------------|-----------------------------|
| `'connected'`    | none                  | WebSocket opened            |
| `'disconnected'` | `(reason: string)`    | Connection closed           |
| `'error'`        | `(err: Error)`        | Protocol/network error      |
| `'authenticated'`| none                  | Auth success                |
| `'stateChanged'` | `(state: ClientState)`| State transition            |
| `'event'`        | `(evt: DraoxEvent)`   | Any server push event       |

**Properties**:

| Property          | Type     | Description                     |
|-------------------|----------|---------------------------------|
| `state`           | string   | Current `ClientState`           |
| `sessionId`       | string?  | Session ID after authentication |
| `isAuthenticated` | boolean  | Whether authenticated           |

### 6.6 CLI Demo

```bash
cd sdk-ts/draox-ts-demo

# Default: user_001 on localhost:9002, channel general
npm start

# Custom channel
npm start -- --channel team-alpha

# Custom server credentials
HOST=192.168.1.10 PORT=9002 USER_ID=alice TOKEN=mytoken npm start
```

**Available commands in the CLI**:

| Input              | Description                        |
|--------------------|------------------------------------|
| `<text>`           | Send message to current channel    |
| `/history`         | Reload last 20 messages            |
| `/delete <id>`     | Delete message by ID               |
| `/edit <id> <text>`| Edit message text                  |
| `/react <id> <emoji>` | Add emoji reaction              |
| `/help`            | Show command list                  |
| `/quit`            | Exit                               |

---

## 7. Tool — SDK Code Generator (`sdk-gen/`)

### 7.1 Overview

`sdk-gen` is a Rust CLI tool that reads an **OpenAPI 3.0** specification and generates typed REST API client code for multiple target languages.

```
sdk-gen/
├── Cargo.toml
└── src/
    ├── main.rs              # CLI entry point (clap)
    ├── model.rs             # ApiEndpoint IR extraction from OpenAPI
    ├── spec.rs              # OpenAPI spec loader (JSON/YAML)
    └── emitters/
        ├── mod.rs           # Emitter registry
        ├── typescript.rs    # TypeScript fetch-based client
        └── dart.rs          # Dart http-based client
```

### 7.2 Build

```bash
cd backend/tools/sdk-gen
cargo build --release
# Binary: target/release/sdk-gen
```

Or from the workspace root:

```bash
cargo build -p sdk-gen --release
```

### 7.3 Usage

```
sdk-gen --help

USAGE:
    sdk-gen [OPTIONS] --spec <SPEC>

OPTIONS:
    -s, --spec <SPEC>         Path to OpenAPI spec (JSON or YAML)
    -o, --output <OUTPUT>     Output directory [default: sdk-out]
    -b, --base-url <BASE_URL> Base URL written into the generated client
                              [default: https://api.draox-server.io]
    -t, --targets <TARGETS>   Target languages: typescript,dart
                              [default: typescript]
    -h, --help                Print help
    -V, --version             Print version
```

**Examples**:

```bash
# Generate TypeScript client from the Admin API spec
sdk-gen --spec ../../admin-api/openapi.json --output ./out/ts --targets typescript

# Generate both TypeScript and Dart
sdk-gen --spec ./my-api.yaml \
        --output ./generated \
        --base-url https://my.server.io \
        --targets typescript,dart

# TypeScript output: ./generated/client.ts
# Dart output:       ./generated/client.dart
```

### 7.4 Generated Output Example

**TypeScript** (`client.ts`):

```typescript
// Auto-generated by sdk-gen — DO NOT EDIT

const BASE_URL = 'https://api.draox-server.io';

async function request<T>(method: string, path: string, body?: unknown): Promise<T> {
  const res = await fetch(`${BASE_URL}${path}`, {
    method,
    headers: { 'Content-Type': 'application/json' },
    body: body ? JSON.stringify(body) : undefined,
  });
  if (!res.ok) throw new Error(`${method} ${path} → ${res.status}`);
  return res.json() as Promise<T>;
}

/** List all plugins */
export async function listPlugins(): Promise<unknown> {
  return request('GET', `/plugins`);
}

/** Install a plugin */
export async function installPlugin(body: Record<string, unknown>): Promise<unknown> {
  return request('POST', `/plugins/install`, body);
}
```

### 7.5 Extending sdk-gen

To add a new language target (e.g. Kotlin):

1. Create `src/emitters/kotlin.rs`
2. Implement `pub fn emit(endpoints: &[ApiEndpoint], base_url: &str) -> String`
3. Add `pub mod kotlin;` to `src/emitters/mod.rs`
4. Add a `"kotlin"` match arm in `src/main.rs`

The `ApiEndpoint` intermediate representation contains all necessary data:

```rust
pub struct ApiEndpoint {
    pub method: String,          // "GET", "POST", etc.
    pub path: String,            // "/plugins/{id}"
    pub operation_id: String,    // "GetPlugin" (PascalCase)
    pub summary: Option<String>, // doc comment text
    pub params: Vec<ApiParam>,   // path, query, header params
    pub request_body: Option<String>,
    pub response_type: Option<String>,
    pub tags: Vec<String>,
}
```

---

## 8. Plugin API Reference

### 8.1 MessagingPlugin (all SDKs)

#### Actions (outbound requests)

| Method                                                          | Action       | Description                    |
|-----------------------------------------------------------------|--------------|--------------------------------|
| `SendMessageAsync(channelId, text, replyToId?)`                 | `msg.send`   | Send a text message            |
| `GetHistoryAsync(channelId, limit?, beforeId?)`                 | `msg.history`| Fetch message history          |
| `DeleteMessageAsync(messageId)`                                 | `msg.delete` | Delete a message               |
| `EditMessageAsync(messageId, newText)`                          | `msg.edit`   | Edit message text              |
| `SendTypingAsync(channelId)`                                    | `msg.typing` | Send typing indicator (fire-forget) |
| `ReactAsync(messageId, emoji)`                                  | `msg.react`  | Add emoji reaction             |

#### Callbacks / Events (inbound push)

| Callback / Handler   | Server event    | Payload type              |
|----------------------|-----------------|---------------------------|
| `OnMessage`          | `msg.received`  | `MessageReceivedEvent`    |
| `OnMessageDeleted`   | `msg.deleted`   | `MessageDeletedEvent`     |
| `OnTyping`           | `msg.typing`    | `TypingEvent`             |

#### Message DTO Fields

```
MessageDto {
  id:          string   — Unique message ID
  channel_id:  string   — Target channel
  sender_id:   string   — Sender user ID
  text:        string   — Message text
  sent_at:     string   — ISO 8601 timestamp
  reply_to_id: string?  — Parent message ID (threads)
}
```

#### Setup / Teardown

```csharp
// C#
var plugin = new MessagingPlugin(client);
plugin.OnMessage += evt => { /* ... */ };
plugin.RegisterListeners();   // subscribe to "msg" category
// ...
plugin.UnregisterListeners(); // unsubscribe
```

```typescript
// TypeScript
const plugin = new MessagingPlugin(client);
plugin.onMessage = (evt) => { /* ... */ };
plugin.registerListeners();
// ...
plugin.unregisterListeners();
```

---

### 8.2 ClansPlugin (Unity only)

#### Actions

| Method                                          | Action           | Description              |
|-------------------------------------------------|------------------|--------------------------|
| `ListClansAsync()`                              | `clan.list`      | Get list of all clans    |
| `GetClanAsync(clanId)`                          | `clan.get`       | Get clan details         |
| `CreateClanAsync(name, description?)`           | `clan.create`    | Create a new clan        |
| `JoinClanAsync(clanId)`                         | `clan.join`      | Join a clan              |
| `LeaveClanAsync(clanId)`                        | `clan.leave`     | Leave a clan             |
| `KickMemberAsync(clanId, userId)`               | `clan.kick`      | Kick member (owner)      |
| `PromoteMemberAsync(clanId, userId)`            | `clan.promote`   | Promote to officer       |
| `DisbandClanAsync(clanId)`                      | `clan.disband`   | Disband clan (owner)     |

#### Events

| Callback               | Server event        | Description              |
|------------------------|---------------------|--------------------------|
| `OnClanJoined`         | `clan.joined`       | Current user joined clan |
| `OnClanLeft`           | `clan.left`         | Current user left clan   |
| `OnMemberJoined`       | `clan.member_joined`| Another user joined      |
| `OnMemberLeft`         | `clan.member_left`  | Another user left        |

---

### 8.3 PresencePlugin (Unity only)

#### Actions

| Method                                                      | Action             | Description                      |
|-------------------------------------------------------------|--------------------|----------------------------------|
| `SetStatusAsync(status, customText?)`                       | `presence.set`     | Update own status                |
| `GetPresenceAsync(userIds[])`                               | `presence.get`     | Get status of multiple users     |
| `WatchUsersAsync(userIds[])`                                | `presence.watch`   | Subscribe to presence updates    |
| `UnwatchUsersAsync(userIds[])`                              | `presence.unwatch` | Unsubscribe from updates         |

**Status values**: `"online"`, `"away"`, `"busy"`, `"invisible"`

#### Events

| Callback           | Server event        | Description                    |
|--------------------|---------------------|--------------------------------|
| `OnPresenceChanged`| `presence.changed`  | User's status changed          |

---

## 9. Advanced Topics

### 9.1 Custom Raw Requests

All SDKs expose a generic `RequestAsync<T>` / `request<T>()` method for sending custom plugin actions not covered by built-in plugins:

```csharp
// C# — custom leaderboard action
var result = await client.RequestAsync<LeaderboardResponse>(
    "leaderboard.top",
    new { game_mode = "ranked", limit = 10 });
```

```typescript
// TypeScript
const result = await client.request<LeaderboardResponse>(
    'leaderboard.top',
    { game_mode: 'ranked', limit: 10 });
```

### 9.2 Subscribing to Raw Events

```csharp
// C# — listen to a specific event
client.Subscribe("msg.received", evt => {
    var msg = evt.Data<MessageReceivedEvent>();
});

// Subscribe to all events in a category
client.SubscribeCategory("msg", evt => {
    Console.WriteLine($"msg event: {evt.Name}");
});
```

```typescript
// TypeScript
client.subscribe('msg.received', (evt) => {
    console.log('message event:', evt);
});

client.subscribeCategory('msg', (evt) => {
    console.log('msg category event:', evt.name);
});
```

### 9.3 Reconnect Configuration

All SDKs use exponential backoff: `delay = min(base * 2^attempt, max)`.

```csharp
// C# — custom reconnect
var config = new DraoxConfig {
    Reconnect = new ReconnectConfig {
        Enabled = true,
        MaxAttempts = 10,
        BaseDelaySeconds = 0.5,
        MaxDelaySeconds = 60.0,
    }
};
```

```typescript
// TypeScript
const client = new DraoxClient({
  reconnect: {
    enabled: true,
    maxAttempts: 10,
    baseDelayMs: 500,
    maxDelayMs: 60_000,
  }
});
```

To disable reconnect: `Reconnect.Enabled = false` / `reconnect: { enabled: false }`.

### 9.4 gRPC Transport (Unity only)

Enable with the `DRAOX_GRPC` scripting define symbol. Server must be running gRPC on port **9004**.

```csharp
// In DraoxConfig:
Protocol = DraoxProtocol.Grpc,
Port = 9004,
```

**Requirements**: Add NuGet packages `Grpc.Net.Client`, `Google.Protobuf`, `Grpc.Tools` to your Unity project via NuGetForUnity.

### 9.5 TCP vs WebSocket

| Criterion    | TCP (port 9000)         | WebSocket (port 9002)          |
|--------------|-------------------------|--------------------------------|
| Framing      | Newline-delimited JSON  | WebSocket frames               |
| Browser      | Not supported           | Supported (native)             |
| WebGL        | Not supported           | Supported (NativeWebSocket)    |
| Overhead     | Minimal                 | HTTP upgrade header overhead   |
| Recommended  | Server-to-server, native apps | All other clients         |

### 9.6 Error Handling

**C#**:
```csharp
try
{
    await client.ConnectAsync();
    await client.AuthenticateAsync("user", "token");
}
catch (DraoxTimeoutException ex)
{
    Console.WriteLine($"Timeout: {ex.Message}");
}
catch (DraoxException ex)
{
    Console.WriteLine($"Protocol error: {ex.Message}");
}
catch (Exception ex)
{
    Console.WriteLine($"Network error: {ex.Message}");
}
```

**TypeScript**:
```typescript
try {
    await client.connect();
    await client.authenticate('user', 'token');
} catch (err) {
    if (err instanceof Error) console.error('Error:', err.message);
}

client.on('error', (err) => console.error('Runtime error:', err.message));
```

---

*Report generated from `backend/tools/` — Draox SDK v1.0, 2026-05-03*
