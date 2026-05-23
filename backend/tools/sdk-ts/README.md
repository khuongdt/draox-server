# Draox TypeScript SDK

A TypeScript/Node.js client SDK for Draox server — WebSocket transport, full MessagingPlugin support.

## Structure

```
sdk-ts/
├── draox-client/            # SDK library
│   └── src/
│       ├── DraoxClient.ts           Main client (EventEmitter-based)
│       ├── types.ts                 Config, enums, event types
│       ├── Serializer.ts            JSON wire format parser
│       ├── RequestBroker.ts         Request/response correlation with timeout
│       ├── Reconnector.ts           Exponential backoff reconnect
│       ├── transports/
│       │   ├── ITransport.ts        Transport interface
│       │   └── WebSocketTransport.ts  ws-based WebSocket transport
│       ├── plugins/
│       │   └── MessagingPlugin.ts   msg.send / history / events
│       └── index.ts                 Public exports
└── draox-ts-demo/           # CLI messaging demo
    └── src/index.ts
```

## Usage

```typescript
import { DraoxClient, MessagingPlugin } from 'draox-client';

const client = new DraoxClient({ host: 'localhost', port: 9002 });
await client.connect();
await client.authenticate('user_001', 'test_token');

const messaging = new MessagingPlugin(client);
messaging.onMessage = (e) => console.log(`${e.message.sender_id}: ${e.message.text}`);
messaging.registerListeners();

await messaging.sendMessage('general', 'Hello from TypeScript!');
```

## Requirements

- Node.js 18+
- Draox server running on localhost:9002
