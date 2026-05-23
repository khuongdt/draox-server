// gRPC is not supported on WebGL.
// To enable gRPC support: install Grpc.Net.Client, Google.Protobuf, and Grpc.Tools via NuGet,
// then add DRAOX_GRPC to Player Settings > Scripting Define Symbols.
// Grpc.Tools will generate Draox.cs and DraoxGrpc.cs from backend/proto/draox.proto.
#if DRAOX_GRPC && (!UNITY_WEBGL || UNITY_EDITOR)

using System;
using System.Threading;
using Cysharp.Threading.Tasks;
using Grpc.Net.Client;
using UnityEngine;

namespace Draox.Client
{
    // gRPC transport using Grpc.Net.Client.
    // Uses unary RPCs for request/response and server-streaming for event subscription.
    // Generated types (Draox.DraoxService, Draox.AuthRequest, etc.) come from proto/draox.proto.
    internal class GrpcConnection
    {
        private GrpcChannel                                    _channel;
        private Draox.DraoxService.DraoxServiceClient          _client;
        private Draox.DraoxStreamService.DraoxStreamServiceClient _streamClient;
        private CancellationTokenSource                        _subscriptionCts;

        public bool IsConnected => _channel != null;

        public UniTask ConnectAsync(DraoxConfig config, CancellationToken ct = default)
        {
            var scheme = config.UseTls ? "https" : "http";
            var port   = config.Port > 0 ? config.Port : 9004;
            var addr   = $"{scheme}://{config.Host}:{port}";

            _channel      = GrpcChannel.ForAddress(addr);
            _client       = new Draox.DraoxService.DraoxServiceClient(_channel);
            _streamClient = new Draox.DraoxStreamService.DraoxStreamServiceClient(_channel);
            return UniTask.CompletedTask;
        }

        public async UniTask<string> AuthenticateAsync(string userId, string token, CancellationToken ct = default)
        {
            var req = new Draox.AuthRequest { UserId = userId, Token = token };
            var res = await _client.AuthenticateAsync(req, cancellationToken: ct);
            if (!res.Success)
                throw new DraoxAuthException(res.Error);
            return res.SessionId;
        }

        public async UniTask<DraoxResponse> SendAsync(DraoxRequest req, CancellationToken ct = default)
        {
            var payloadBytes = Google.Protobuf.ByteString.CopyFromUtf8(
                req.Payload != null ? Serializer.Serialize(req.Payload) : "{}");

            var grpcReq = new Draox.DraoxRequest
            {
                Id      = req.Id,
                Action  = req.Action,
                Payload = payloadBytes,
            };

            var res = await _client.SendAsync(grpcReq, cancellationToken: ct);

            return new DraoxResponse
            {
                Id      = res.Id,
                Success = res.Success,
                RawData = res.Data?.ToStringUtf8(),
                Error   = res.Error,
            };
        }

        // Starts a server-streaming subscription for the given event categories.
        // onEvent is always dispatched on the Unity main thread.
        public void Subscribe(string sessionId, string[] categories, Action<DraoxEvent> onEvent)
        {
            _subscriptionCts?.Cancel();
            _subscriptionCts = new CancellationTokenSource();
            SubscribeAsync(sessionId, categories, onEvent, _subscriptionCts.Token).Forget();
        }

        private async UniTaskVoid SubscribeAsync(
            string sessionId, string[] categories, Action<DraoxEvent> onEvent, CancellationToken ct)
        {
            var req = new Draox.SubscribeRequest { SessionId = sessionId };
            req.Categories.AddRange(categories);

            var stream = _streamClient.Subscribe(req, cancellationToken: ct);
            try
            {
                await foreach (var msg in stream.ResponseStream.ReadAllAsync(ct))
                {
                    await UniTask.SwitchToMainThread();
                    onEvent(new DraoxEvent
                    {
                        Category  = msg.Category,
                        Name      = msg.Name,
                        RawData   = msg.Data?.ToStringUtf8(),
                        Timestamp = msg.Timestamp,
                    });
                }
            }
            catch (OperationCanceledException) { }
            catch (Exception ex)
            {
                Debug.LogError($"[Draox] gRPC stream error: {ex.Message}");
            }
        }

        public async UniTask DisconnectAsync()
        {
            _subscriptionCts?.Cancel();
            if (_channel != null)
            {
                await _channel.ShutdownAsync().AsUniTask();
                _channel = null;
            }
        }
    }
}

#endif
