# Draox TypeScript Demo — CLI Messaging Chat

A terminal-based chat client demonstrating the **MessagingPlugin** of the Draox TypeScript SDK.

## Quick Start

```bash
# 1. Install dependencies for the SDK
cd ../draox-client && npm install

# 2. Install demo dependencies
cd ../draox-ts-demo && npm install

# 3. Start the Draox server (from repo root)
cargo run -- --config config/default.toml

# 4. Run the demo
npm start

# Custom channel
npm start -- --channel team-alpha

# Custom server / credentials
HOST=192.168.1.10 PORT=9002 USER_ID=alice TOKEN=secret npm start
```

## Screenshot

```
  Draox Messaging Demo (TypeScript)

  Server:  localhost:9002
  User:    user_001
  Channel: #general

  Connecting… OK
  Authenticating… OK  (session: a1b2c3d4…)

  #general
  ──────────────────────────────────────────────────
  user_002  14:20
    Hey there!
  user_001  14:21
    Hello from TypeScript SDK!
  ──────────────────────────────────────────────────
  Type a message or /help for commands.

  [user_001] _
```

## Commands

| Command | Description |
|---------|-------------|
| `<text>` | Send a message to the current channel |
| `/history` | Reload the last 20 messages |
| `/delete <id>` | Delete a message by ID |
| `/edit <id> <text>` | Edit a message |
| `/react <id> <emoji>` | Add emoji reaction |
| `/help` | Show this list |
| `/quit` | Exit |

## Real-time Events

- Incoming messages appear automatically as they arrive
- Typing indicators show `<user> is typing…`
- Deleted messages are announced inline

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `HOST` | `localhost` | Server hostname |
| `PORT` | `9002` | WebSocket port |
| `USER_ID` | `user_001` | User identifier |
| `TOKEN` | `test_token` | Auth token |
