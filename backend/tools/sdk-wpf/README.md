# Draox SDK for .NET (WPF/Desktop)

A .NET 8 client SDK for Draox server — no Unity dependency. Supports WebSocket and TCP transports.

## Structure

```
sdk-wpf/
├── DraoxClientWpf/       # SDK library (.NET 8 class library)
│   ├── Core/
│   │   ├── DraoxClient.cs          Main client class
│   │   ├── DraoxConfig.cs          Config, enums, wire types
│   │   ├── IConnection.cs          Transport interface
│   │   ├── WebSocketConnection.cs  System.Net.WebSockets transport
│   │   ├── TcpConnection.cs        TCP line-delimited transport
│   │   ├── Serializer.cs           System.Text.Json helpers
│   │   ├── RequestBroker.cs        Request/response correlation
│   │   └── Reconnector.cs          Auto-reconnect with exponential backoff
│   └── Plugins/
│       └── MessagingPlugin.cs      msg.send / msg.history / events
└── DraoxWpfDemo/         # WPF demo app (net8.0-windows)
    ├── MainWindow.xaml    Dark-themed chat UI
    └── MainWindow.xaml.cs Connection + auth + messaging logic
```

## Quick Start

```bash
cd DraoxWpfDemo
dotnet run
```

## Usage

```csharp
var client = new DraoxClient(new DraoxConfig
{
    Host     = "localhost",
    Port     = 9002,
    Protocol = DraoxProtocol.WebSocket,
});

await client.ConnectAsync();
await client.AuthenticateAsync("user_001", "test_token");

var messaging = new MessagingPlugin(client);
messaging.OnMessage += e => Console.WriteLine($"{e.Message.SenderId}: {e.Message.Text}");
messaging.RegisterListeners();

await messaging.SendMessageAsync("general", "Hello from .NET!");
```

## Demo Features

| Feature | Description |
|---------|-------------|
| Connect / Disconnect | WebSocket or TCP to any host:port |
| Authenticate | User ID + token, session displayed |
| Load History | Fetches last 30 messages from a channel |
| Send Message | Enter key or Send button |
| Receive Messages | Real-time via `msg.received` events |
| Typing indicator | Shows `<user> is typing…` notifications |
| Message deleted | Shows deletion notice in chat |

## Requirements

- .NET 8 SDK
- Draox server running on `localhost:9002` (WS) or `localhost:9000` (TCP)
