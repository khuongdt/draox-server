# Draox Demo — Unity Test Application

A self-contained Unity scene that exercises every feature of the **DraoxClientUnity SDK**:  
WebSocket / TCP / gRPC connections, authentication, request/response, event subscription, and the three built-in plugins (Clans, Messaging, Presence).

---

## Prerequisites

| Requirement | Version |
|-------------|---------|
| Unity | 2022.3 LTS or newer |
| DraoxClientUnity package | included at `../DraoxClientUnity/` |
| UniTask | 2.5.0+ (via UPM) |
| NativeWebSocket | 1.1.6+ (via UPM) |
| TextMeshPro | built-in Unity package |
| Draox Server | running locally (see below) |

---

## Quick Start

### 1. Start the Draox Server

```bash
# From the repo root
cargo run -- --config config/default.toml
```

Default ports used by this demo:

| Protocol | Port |
|----------|------|
| WebSocket | 9002 |
| TCP       | 9000 |
| gRPC      | 9004 |

### 2. Import into Unity

1. Open Unity Hub → **Add project from disk** → select this folder (`DraoxDemo/`).
2. Open the project (Unity will import all packages automatically).
3. From the menu bar: **Draox → Build Demo Scene**.  
   This runs the editor script and populates `Assets/Scenes/DemoScene.unity` with the full UI hierarchy, wires all component references, and saves the scene.
4. Open **Assets/Scenes/DemoScene.unity** and hit **Play**.

### 3. Open Package Manager (first time only)

Go to **Window → Package Manager → + → Add package by name** and add:

```
com.cysharp.unitask
io.github.endel.nativewebsocket
com.unity.textmeshpro
```

---

## Scene Layout

```
Canvas
├── StatusBar          ← connection state indicator (color dot + label)
├── TabBar             ← 6 tab buttons
├── Panels/
│   ├── ConnectionPanel
│   ├── AuthPanel
│   ├── RequestPanel
│   ├── ClansPanel
│   ├── MessagingPanel
│   └── PresencePanel
├── EventLog           ← scrollable colored log (right side)
└── DemoRoot           ← DemoManager + DraoxClient components
```

---

## Panel Reference

### Connection

| Field | Default | Description |
|-------|---------|-------------|
| Host | `127.0.0.1` | Server hostname or IP |
| Port | `9002` | Server port |
| Protocol | WebSocket | WebSocket / TCP / gRPC |
| Use TLS | off | Enable wss:// or grpcs:// |
| Timeout (ms) | `10000` | Request timeout |
| Reconnect | on | Enable automatic reconnect |
| Max Attempts | `5` | `0` = unlimited |
| Base Delay (s) | `1` | First retry delay (doubles each attempt) |

**Buttons**
- **Connect** — applies the fields to the client config and calls `ConnectAsync()`.
- **Disconnect** — calls `DisconnectAsync("user_request")`.

---

### Auth

| Field | Default |
|-------|---------|
| User ID | `user_001` |
| Token | `test_token` |

**Buttons**
- **Authenticate** — calls `AuthenticateAsync(userId, token)`. The Session ID is displayed on success.
- **Add Connection** — calls `AddConnectionAsync(role)` to bind an extra connection (Notification / Control / Streaming).

---

### Request

Test any action directly against the server.

| Field | Description |
|-------|-------------|
| Action | `echo`, `ping`, any custom action |
| Payload (JSON) | Arbitrary JSON object, e.g. `{"message":"hello"}` |

**Buttons**
- **Send** — fire-and-forget (`SendAsync`), no response expected.
- **Request** — awaited call (`RequestAsync<object>`), prints the JSON response.
- **Subscribe / Unsubscribe** — register/deregister a handler for an event name.
- **Ping** — sends an `echo`/`ping` request and reports round-trip time.

---

### Clans

Tests the `ClansPlugin`.

| Operation | Fields |
|-----------|--------|
| List clans | (none) |
| Create clan | Name, Tag, Description |
| Join clan | Clan ID |
| Leave clan | (none) |
| Kick member | Target User ID |
| Promote member | Target User ID, New Role (`member`/`officer`/`owner`) |

**Events shown in log:**
- `clan.joined` — local player joined a clan
- `clan.left` — local player left a clan
- `clan.member_joined` / `clan.member_left` — member roster change

---

### Messaging

Tests the `MessagingPlugin`.

| Operation | Fields |
|-----------|--------|
| Send message | Channel, Text |
| Typing indicator | Channel |
| History | Channel, Limit |
| Delete message | Message ID |
| Edit message | Message ID, New Text |
| React | Message ID, Emoji |

**Events shown in log:**
- `msg.received` — incoming message
- `msg.deleted` — a message was deleted
- `msg.typing` — a user is typing

---

### Presence

Tests the `PresencePlugin`.

| Operation | Fields |
|-----------|--------|
| Set status | Status (`online`/`away`/`busy`/`invisible`), Custom text |
| Get presence | Comma-separated user IDs |
| Watch | Comma-separated user IDs |
| Unwatch | Comma-separated user IDs |

**Events shown in log:**
- `presence.changed` — a watched user's status changed

---

## Event Log

All SDK events and responses appear in the right-side scrollable log:

| Color | Meaning |
|-------|---------|
| Gray  | `[LOG]` info messages |
| Green | `[OK]`  successful operations |
| Yellow| `[WARN]` warnings / disconnects |
| Red   | `[ERR]` errors / exceptions |
| Blue  | `[EVT]` server-pushed events |

---

## Platform Notes

| Platform | WebSocket | TCP | gRPC |
|----------|-----------|-----|------|
| Standalone (Win/Mac/Linux) | ✅ | ✅ | ✅ (opt-in) |
| Android / iOS | ✅ | ✅ | ✅ (opt-in) |
| WebGL | ✅ | ❌ auto-fallback | ❌ auto-fallback |
| Unity Editor | ✅ | ✅ | ✅ (opt-in) |

**gRPC opt-in**: install `Grpc.Net.Client`, `Google.Protobuf`, and `Grpc.Tools` via NuGet (NuGetForUnity),  
then add `DRAOX_GRPC` to **Player Settings → Other Settings → Scripting Define Symbols**.

---

## Building the Scene

The recommended way is to use the included editor script:

**Draox → Build Demo Scene**

The script (`Assets/Scripts/Editor/DemoSceneBuilder.cs`) does the following automatically:

1. Opens `Assets/Scenes/DemoScene.unity`.
2. Creates the full GameObject hierarchy — Camera, EventSystem, Canvas, StatusBar, TabBar, all six Panels, EventLog.
3. Adds and configures all components (`Image`, `ScrollRect`, `CanvasScaler`, `VerticalLayoutGroup`, TMP text, etc.).
4. Wires every `[SerializeField]` reference via `SerializedObject` API.
5. Wires all button `OnClick` listeners via `UnityEventTools`.
6. Saves the scene.

> Run it again at any time to regenerate the scene from scratch (safe to re-run).

---

## Scene Setup Guide (manual)

If you prefer to build manually instead of using the editor script:

1. **Create a Canvas** — Screen Space Overlay, `CanvasScaler` set to Scale With Screen Size, reference 1920×1080, match 0.5.
2. **Create DemoRoot** — empty GameObject, attach `DraoxClient` and `DemoManager`.
   - Wire `DemoManager.draoxClient` to the `DraoxClient` component.
3. **Create EventLog** — Panel on the right half, attach `EventLog.cs`, wire `ScrollRect` and `TextMeshProUGUI`.
4. **Create StatusBar** — top strip with an `Image` (status dot) and `TextMeshProUGUI` (state label).
   - Wire both to `DemoManager`.
5. **Create TabBar** — 6 `Button` components, each calls the matching `DemoManager.ShowXxx()` method via `OnClick`.
6. **Create each Panel** — add the corresponding script, wire all `[SerializeField]` fields.
7. **Wire DemoManager tab panel references** to the GameObjects created in step 6.

---

## File Structure

```
DraoxDemo/
├── Assets/
│   ├── Scenes/
│   │   └── DemoScene.unity          # Pre-built scene (open this)
│   └── Scripts/
│       ├── DraoxDemo.asmdef         # Assembly: references DraoxClientUnity + TMP
│       ├── EventLog.cs              # Singleton colored scrollable log
│       ├── DemoManager.cs           # Root controller, plugin helpers, tab navigation
│       ├── ConnectionPanel.cs       # Connect / Disconnect
│       ├── AuthPanel.cs             # Authenticate / AddConnection
│       ├── RequestPanel.cs          # Send / Request / Subscribe / Ping
│       ├── ClansPanel.cs            # Clans CRUD + events
│       ├── MessagingPanel.cs        # Send, history, delete, edit, react + events
│       └── PresencePanel.cs         # Set status, get/watch/unwatch + events
└── README.md                        # This file
```

---

## Troubleshooting

| Problem | Fix |
|---------|-----|
| `WebSocket is not connected` | Check server is running on the correct host/port |
| `Not connected (state: Disconnected)` | Call **Connect** before any other operation |
| `Auth failed: session not found` | Ensure the user ID / token match a valid account on the server |
| TCP not available on WebGL | Expected — client auto-falls back to WebSocket |
| gRPC buttons do nothing | Add `DRAOX_GRPC` scripting define and install Grpc.Net.Client |
| Log empty | Make sure `EventLog` component is in the scene and `Instance` is set |
