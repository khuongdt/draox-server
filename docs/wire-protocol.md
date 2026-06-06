# Draox Server — Wire Protocol Reference

**Version:** 1.0 · **Updated:** 2026-06-04

This document describes the JSON wire protocol used by all Draox Server socket connections (TCP, UDP, WebSocket). It is intended for developers building client SDKs or integrating with the server.

---

## Overview

All protocols share the same JSON message envelope. The framing mechanism differs per transport:

| Transport | Framing | Notes |
|-----------|---------|-------|
| **TCP** | Newline-delimited (`\n` by default, admin-configurable) | Messages are buffered per-connection and split on the delimiter |
| **UDP** | One datagram = one complete message | No buffering needed |
| **WebSocket** | Built-in WS frame boundaries | Standard WS text frames |

---

## Authentication Flow

Authentication is a two-step process.

### Step 1 — HTTP Login (Admin API)

Obtain a JWT token via the REST endpoint:

```
POST http://<host>:9100/api/auth/login
Content-Type: application/json

{
  "username": "admin",
  "password": "your-password"
}
```

**Response:**

```json
{
  "success": true,
  "data": {
    "token": "<JWT_TOKEN>",
    "username": "admin",
    "role": "admin"
  }
}
```

### Step 2 — Socket Authentication

After connecting via TCP, UDP, or WebSocket, the client **must** send an auth request within `auth_timeout_secs` (default 30 seconds) or the connection will be closed.

**Request:**

```json
{"id":"req_<guid>","type":"request","action":"auth","payload":{"user_id":"admin","token":"<JWT_TOKEN>"}}\n
```

**Success response:**

```json
{"type":"response","id":"req_<guid>","success":true,"data":{"session_id":"ses_..."}}\n
```

**Failure response:**

```json
{"type":"response","id":"req_<guid>","success":false,"error":"auth failed: ExpiredSignature"}\n
```

Store the `session_id` — it identifies the server-authoritative session for this connection.

---

## Message Format

### Request (client → server)

```json
{
  "id":      "req_<guid>",           // Correlation ID, echoed in response
  "type":    "request",              // Frame type
  "action":  "messaging.send_message", // Action to invoke
  "payload": { ... },                // Action-specific payload (optional)
  "token":   "<JWT_TOKEN>"           // JWT (optional, carried in requests after auth)
}
```

### Response (server → client)

```json
{
  "type":    "response",
  "id":      "req_<guid>",           // Echoes the request id
  "success": true,                   // true on success, false on error
  "data":    { ... },                // Response payload (on success)
  "error":   "..."                   // Error message (on failure)
}
```

### Ping / Pong (keep-alive)

**Client sends:**
```json
{"type":"ping","ts":1748930400000}\n
```

**Server replies:**
```json
{"type":"pong"}\n
```

Ping/Pong does **not** require authentication and can be sent at any time.

---

## Role Hierarchy and Access Control

Connections that have not yet authenticated are treated as `anonymous` role.

| Role | Allowed actions |
|------|----------------|
| `anonymous` | `auth`, `ping` only |
| `viewer` | Read-only plugin actions |
| `operator` | Read + write plugin actions |
| `admin` | All actions |

If a connection attempts an action not allowed for its role, the server returns:

```json
{"type":"response","success":false,"error":"role 'anonymous' only allows 'auth' and 'ping'"}
```

---

## Built-in Actions

| Action | Description | Auth required |
|--------|-------------|---------------|
| `auth` | Authenticate connection with JWT | No |
| `ping` | Keep-alive check | No |
| `messaging.*` | Instant messaging actions | Yes |
| `clans.*` | Clans/Groups actions | Yes |

---

## Plugin Actions (Messaging)

### `messaging.send_message`

```json
{
  "type": "request",
  "action": "messaging.send_message",
  "payload": {
    "channel_id": "ch_...",
    "content": "Hello!",
    "content_type": "text"
  }
}
```

### `messaging.subscribe_channel`

```json
{
  "type": "request",
  "action": "messaging.subscribe_channel",
  "payload": { "channel_id": "ch_..." }
}
```

---

## Configuration Reference

Wire protocol settings in `config/default.toml`:

```toml
[wire_protocol]
frame_delimiter = "\n"   # TCP message delimiter (admin-configurable)
auth_timeout_secs = 30   # Seconds to authenticate after connecting
require_auth = true      # Set to false only for development/testing
```

### Admin notes

- **`frame_delimiter`** can be changed to any byte sequence (e.g. `"\r\n"` for Windows-style). All connected clients must use the same delimiter.
- **`auth_timeout_secs`** controls how long an unauthenticated connection is kept open. Lower values improve security; higher values accommodate slow clients.
- **`require_auth = false`** disables the auth gate. All connections are treated as authenticated with `anonymous` role. **Never use in production.**

---

## Error Reference

| Error message | Cause |
|---------------|-------|
| `missing 'token' in auth payload` | `payload.token` not provided in auth request |
| `auth failed: <JWT error>` | JWT invalid, expired, or wrong secret |
| `session not found for connection` | Connection has no associated session |
| `role 'anonymous' only allows 'auth' and 'ping'` | Non-auth action sent before authenticating |
| `no dispatcher configured` | Server misconfiguration |

---

## Code Examples

### C# (WPF / .NET)

```csharp
// Step 1: HTTP login
var loginResp = await httpClient.PostAsJsonAsync("/api/auth/login",
    new { username = "admin", password = "pass" });
var token = loginResp.Data.token;

// Step 2: TCP connect + auth
var writer = new StreamWriter(tcpStream);
var authMsg = JsonSerializer.Serialize(new {
    id = $"req_{Guid.NewGuid()}",
    type = "request",
    action = "auth",
    payload = new { user_id = "admin", token }
});
await writer.WriteLineAsync(authMsg); // \n appended automatically

// Read response
var response = await reader.ReadLineAsync();
var sessionId = JsonDocument.Parse(response).RootElement
    .GetProperty("data").GetProperty("session_id").GetString();
```

### TypeScript (Node.js)

```typescript
import net from 'net';

const socket = net.createConnection(9000, 'localhost');

socket.on('connect', () => {
  const authMsg = JSON.stringify({
    id: `req_${crypto.randomUUID()}`,
    type: 'request',
    action: 'auth',
    payload: { user_id: 'admin', token: jwtToken }
  });
  socket.write(authMsg + '\n');
});

socket.on('data', (data) => {
  const response = JSON.parse(data.toString().trim());
  if (response.success) {
    console.log('Session ID:', response.data.session_id);
  }
});
```

### Unity C# (WebSocket)

```csharp
// WebSocket uses same message format but no newline delimiter
var authMsg = JsonUtility.ToJson(new WireRequest {
    id = $"req_{System.Guid.NewGuid()}",
    type = "request",
    action = "auth",
    payload = new AuthPayload { user_id = username, token = jwtToken }
});
ws.SendText(authMsg);
```

---

## Architecture Notes

- **Plugins never see JWT tokens.** The `AuthHandler` (framework layer) validates the JWT and authenticates the session. Plugins only receive a `WsActionContext` with an `Identity` object containing `user_id` and `role`.
- **Single secret for all protocols.** The JWT secret configured in `[admin_api] jwt_secret` is shared by TCP, UDP, WebSocket, and HTTP authentication.
- **Session-level auth.** Once a connection authenticates, the session is marked as authenticated. All connections within the same session inherit the auth state.
