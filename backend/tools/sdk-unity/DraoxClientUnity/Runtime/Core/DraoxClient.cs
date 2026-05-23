using System;
using System.Collections.Generic;
using System.Threading;
using Cysharp.Threading.Tasks;
using UnityEngine;

namespace Draox.Client
{
    /// <summary>
    /// Main entry point for the Draox client SDK.
    /// Attach to a GameObject and configure via the Inspector.
    /// </summary>
    public class DraoxClient : MonoBehaviour
    {
        [SerializeField] private DraoxConfig config = new DraoxConfig();

        // ── Internal state ────────────────────────────────────────────────────
        private IConnection    _connection;
        private RequestBroker  _broker;
        private Reconnector    _reconnector;
        private SessionManager _sessionManager;

        private CancellationTokenSource _lifetimeCts;
        private CancellationTokenSource _heartbeatCts;

        private string _savedUserId;
        private string _savedToken;
        private int    _missedPings;

        private readonly Dictionary<string, List<Action<DraoxEvent>>> _eventHandlers    = new Dictionary<string, List<Action<DraoxEvent>>>();
        private readonly Dictionary<string, List<Action<DraoxEvent>>> _categoryHandlers = new Dictionary<string, List<Action<DraoxEvent>>>();

#if DRAOX_GRPC && (!UNITY_WEBGL || UNITY_EDITOR)
        private GrpcConnection _grpc;
#endif

        // ── Public state ──────────────────────────────────────────────────────
        public ClientState State           { get; private set; } = ClientState.Disconnected;
        public string      SessionId       => _sessionManager?.SessionId;
        public bool        IsAuthenticated => !string.IsNullOrEmpty(SessionId);

        // Exposes config so callers (e.g. demo UI) can mutate it before ConnectAsync().
        public DraoxConfig Config => config;

        // ── C# events ─────────────────────────────────────────────────────────
        public event Action              OnConnected;
        public event Action<string>      OnDisconnected;
        public event Action<string>      OnError;
        public event Action              OnAuthenticated;
        public event Action<ClientState> OnStateChanged;

        // ── Unity lifecycle ───────────────────────────────────────────────────
        private void Awake()
        {
#if UNITY_WEBGL && !UNITY_EDITOR
            if (config.Protocol == DraoxProtocol.Grpc)
            {
                Debug.LogWarning("[Draox] gRPC is not supported on WebGL — falling back to WebSocket");
                config.Protocol = DraoxProtocol.WebSocket;
                config.Port     = 9002;
            }
            if (config.Protocol == DraoxProtocol.Tcp)
            {
                Debug.LogWarning("[Draox] TCP is not supported on WebGL — falling back to WebSocket");
                config.Protocol = DraoxProtocol.WebSocket;
                config.Port     = 9002;
            }
#endif
        }

        private void Update()
        {
            // NativeWebSocket requires manual queue dispatch on WebGL.
#if UNITY_WEBGL && !UNITY_EDITOR
            (_connection as WebSocketConnection)?.DispatchMessageQueue();
#endif
        }

        private void OnDestroy()
        {
            _lifetimeCts?.Cancel();
            DisconnectAsync("object_destroyed").Forget();
        }

        // ── Public API ────────────────────────────────────────────────────────

        public async UniTask ConnectAsync(CancellationToken ct = default)
        {
            if (State == ClientState.Connected || State == ClientState.Connecting)
                return;

            _lifetimeCts = new CancellationTokenSource();
            SetState(ClientState.Connecting);

#if DRAOX_GRPC && (!UNITY_WEBGL || UNITY_EDITOR)
            if (config.Protocol == DraoxProtocol.Grpc)
            {
                _grpc           = new GrpcConnection();
                _sessionManager = new SessionManager(config);
                await _grpc.ConnectAsync(config, ct);
                SetState(ClientState.Connected);
                OnConnected?.Invoke();
                return;
            }
#endif
            _connection = config.Protocol switch
            {
#if !UNITY_WEBGL || UNITY_EDITOR
                DraoxProtocol.Tcp => new TcpConnection(),
#endif
                _                 => new WebSocketConnection(),
            };

            _broker         = new RequestBroker();
            _reconnector    = new Reconnector(config.Reconnect);
            _sessionManager = new SessionManager(config);

            _connection.MessageReceived += OnMessageReceived;
            _connection.Closed          += OnConnectionClosed;

            await _connection.ConnectAsync(config, ct);

            SetState(ClientState.Connected);
            OnConnected?.Invoke();
            StartHeartbeat();
        }

        public async UniTask DisconnectAsync(string reason = "client_disconnect")
        {
            _heartbeatCts?.Cancel();
            _lifetimeCts?.Cancel();

            if (_sessionManager != null)
                await _sessionManager.DisconnectAllAsync();

#if DRAOX_GRPC && (!UNITY_WEBGL || UNITY_EDITOR)
            if (_grpc != null)
            {
                await _grpc.DisconnectAsync();
                _grpc = null;
            }
#endif
            if (_connection != null)
            {
                _broker?.FailAll(new DraoxException($"Disconnected: {reason}"));
                await _connection.DisconnectAsync();
                _connection = null;
            }

            SetState(ClientState.Disconnected);
            OnDisconnected?.Invoke(reason);
        }

        public async UniTask AuthenticateAsync(string userId, string token, CancellationToken ct = default)
        {
            _savedUserId = userId;
            _savedToken  = token;

#if DRAOX_GRPC && (!UNITY_WEBGL || UNITY_EDITOR)
            if (_grpc != null)
            {
                var sessionId = await _grpc.AuthenticateAsync(userId, token, ct);
                _sessionManager.SessionId = sessionId;
                OnAuthenticated?.Invoke();
                return;
            }
#endif
            var res = await RequestInternalAsync<AuthResponseData>("auth", new { user_id = userId, token }, ct);
            _sessionManager.SessionId = res.SessionId;
            OnAuthenticated?.Invoke();
        }

        // Fire-and-forget — no response expected.
        public UniTask SendAsync(string action, object payload = null, CancellationToken ct = default)
        {
            EnsureConnected();
            var json = Serializer.Serialize(new WireRequest { id = NewId(), action = action, payload = payload });
            return _connection.SendTextAsync(json, ct);
        }

        // Awaitable request — waits for the matching response from the server.
        public async UniTask<T> RequestAsync<T>(string action, object payload = null, CancellationToken ct = default)
        {
            EnsureConnected();

#if DRAOX_GRPC && (!UNITY_WEBGL || UNITY_EDITOR)
            if (_grpc != null)
            {
                var grpcRes = await _grpc.SendAsync(new DraoxRequest { Id = NewId(), Action = action, Payload = payload }, ct);
                if (!grpcRes.Success) throw new DraoxException(grpcRes.Error ?? "request failed");
                return Serializer.Deserialize<T>(grpcRes.RawData);
            }
#endif
            var res = await RequestInternalAsync<T>(action, payload, ct);
            return res;
        }

        public void Subscribe(string eventName, Action<DraoxEvent> handler)
        {
            AddHandler(_eventHandlers, eventName, handler);
        }

        public void Unsubscribe(string eventName, Action<DraoxEvent> handler)
        {
            RemoveHandler(_eventHandlers, eventName, handler);
        }

        public void SubscribeCategory(string category, Action<DraoxEvent> handler)
        {
            AddHandler(_categoryHandlers, category, handler);

#if DRAOX_GRPC && (!UNITY_WEBGL || UNITY_EDITOR)
            if (_grpc != null && IsAuthenticated)
                _grpc.Subscribe(SessionId, new[] { category }, DispatchEvent);
#endif
        }

        public void UnsubscribeCategory(string category, Action<DraoxEvent> handler)
        {
            RemoveHandler(_categoryHandlers, category, handler);
        }

        public UniTask AddConnectionAsync(ConnectionRole role, CancellationToken ct = default) =>
            _sessionManager.AddConnectionAsync(role, ct);

        // ── Internal helpers ──────────────────────────────────────────────────

        private async UniTask<T> RequestInternalAsync<T>(string action, object payload, CancellationToken ct)
        {
            var id   = NewId();
            var json = Serializer.Serialize(new WireRequest { id = id, action = action, payload = payload });
            var res  = await _broker.SendAsync(_connection, json, id, config.TimeoutMs, ct);
            if (!res.Success) throw new DraoxException(res.Error ?? "request failed");
            return Serializer.Deserialize<T>(res.RawData);
        }

        private void OnMessageReceived(string json)
        {
            var msg = Serializer.Parse(json);
            if (msg == null) return;

            switch (msg.Type)
            {
                case "response":
                    _broker?.Complete(msg.Id, new DraoxResponse
                    {
                        Id      = msg.Id,
                        Success = msg.Success,
                        RawData = msg.RawData,
                        Error   = msg.Error,
                    });
                    break;

                case "event":
                    DispatchEvent(new DraoxEvent
                    {
                        Category  = msg.Category,
                        Name      = msg.Name,
                        RawData   = msg.RawData,
                        Timestamp = msg.Timestamp,
                    });
                    break;

                case "pong":
                    _missedPings = 0;
                    break;
            }
        }

        private void DispatchEvent(DraoxEvent evt)
        {
            // Dispatch by "Category.Name" (full), "Name" (short), and "Category".
            TryDispatch(_eventHandlers, $"{evt.Category}.{evt.Name}", evt);
            TryDispatch(_eventHandlers, evt.Name, evt);
            TryDispatch(_categoryHandlers, evt.Category, evt);
        }

        private static void TryDispatch(
            Dictionary<string, List<Action<DraoxEvent>>> dict, string key, DraoxEvent evt)
        {
            if (dict.TryGetValue(key, out var handlers))
                foreach (var h in handlers) h(evt);
        }

        private void OnConnectionClosed(string reason)
        {
            _heartbeatCts?.Cancel();
            SetState(ClientState.Disconnected);
            OnDisconnected?.Invoke(reason);

            if (config.Reconnect.Enabled && _reconnector != null)
                TryReconnectAsync().Forget();
        }

        private async UniTaskVoid TryReconnectAsync()
        {
            SetState(ClientState.Reconnecting);

            var success = await _reconnector.AttemptAsync(async () =>
            {
                try
                {
                    await _connection.ConnectAsync(config, _lifetimeCts.Token);
                    if (!string.IsNullOrEmpty(_savedUserId))
                        await AuthenticateAsync(_savedUserId, _savedToken, _lifetimeCts.Token);
                    return true;
                }
                catch { return false; }
            }, _lifetimeCts.Token);

            if (success)
            {
                SetState(ClientState.Connected);
                OnConnected?.Invoke();
                StartHeartbeat();
            }
        }

        private void StartHeartbeat()
        {
            _heartbeatCts?.Cancel();
            _heartbeatCts = new CancellationTokenSource();
            HeartbeatLoopAsync(_heartbeatCts.Token).Forget();
        }

        private async UniTaskVoid HeartbeatLoopAsync(CancellationToken ct)
        {
            var interval = TimeSpan.FromSeconds(config.HeartbeatIntervalSeconds);
            while (!ct.IsCancellationRequested)
            {
                await UniTask.Delay(interval, cancellationToken: ct);
                if (_connection == null || !_connection.IsConnected) break;

                _missedPings++;
                if (_missedPings >= 2)
                {
                    Debug.LogWarning("[Draox] Heartbeat timeout — triggering reconnect");
                    OnConnectionClosed("heartbeat_timeout");
                    break;
                }

                var ping = Serializer.Serialize(new PingMessage { ts = DateTimeOffset.UtcNow.ToUnixTimeMilliseconds() });
                try { await _connection.SendTextAsync(ping, ct); }
                catch { break; }
            }
        }

        private void EnsureConnected()
        {
            if (State != ClientState.Connected)
                throw new DraoxException($"Not connected (state: {State})");
        }

        private static void AddHandler(
            Dictionary<string, List<Action<DraoxEvent>>> dict, string key, Action<DraoxEvent> handler)
        {
            if (!dict.TryGetValue(key, out var list))
                dict[key] = list = new List<Action<DraoxEvent>>();
            list.Add(handler);
        }

        private static void RemoveHandler(
            Dictionary<string, List<Action<DraoxEvent>>> dict, string key, Action<DraoxEvent> handler)
        {
            if (dict.TryGetValue(key, out var list))
                list.Remove(handler);
        }

        private static string NewId() => $"req_{Guid.NewGuid():N}";

        private void SetState(ClientState state)
        {
            if (State == state) return;
            State = state;
            OnStateChanged?.Invoke(state);
        }
    }
}
