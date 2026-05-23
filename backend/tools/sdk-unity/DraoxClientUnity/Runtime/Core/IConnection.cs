using System;
using System.Threading;
using Cysharp.Threading.Tasks;

namespace Draox.Client
{
    // Low-level transport abstraction for WebSocket and TCP connections.
    // gRPC uses GrpcConnection directly (different RPC paradigm, not message-based).
    internal interface IConnection
    {
        UniTask ConnectAsync(DraoxConfig config, CancellationToken ct = default);
        UniTask DisconnectAsync();
        UniTask SendTextAsync(string json, CancellationToken ct = default);

        event Action<string> MessageReceived;
        event Action         Opened;
        event Action<string> Closed;

        bool IsConnected { get; }
    }
}
