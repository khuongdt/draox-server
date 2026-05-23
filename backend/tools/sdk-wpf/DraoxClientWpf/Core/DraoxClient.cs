using System.Net.Http;
using System.Text;
using System.Text.Json;

namespace Draox.Client;

/// <summary>
/// Draox client SDK for .NET desktop applications.
/// Create an instance, call ConnectAsync(), then AuthenticateAsync() before using the messaging APIs.
/// </summary>
public class DraoxClient : IDisposable
{
    private readonly DraoxConfig _config;
    private IConnection? _connection;
    private RequestBroker? _broker;
    private Reconnector? _reconnector;
    private CancellationTokenSource? _lifetimeCts;
    private CancellationTokenSource? _heartbeatCts;
    private readonly SynchronizationContext? _syncCtx;

    private string? _savedUserId;
    private string? _savedToken;
    private string? _savedUsername;
    private string? _savedPassword;
    private int _missedPings;

    private static readonly HttpClient _http = new();

    private readonly Dictionary<string, List<Action<DraoxEvent>>> _eventHandlers    = new();
    private readonly Dictionary<string, List<Action<DraoxEvent>>> _categoryHandlers = new();

    public ClientState State     { get; private set; } = ClientState.Disconnected;
    public string?     SessionId { get; private set; }
    public bool        IsAuthenticated => !string.IsNullOrEmpty(SessionId);
    public DraoxConfig Config    => _config;

    public event Action?              OnConnected;
    public event Action<string>?      OnDisconnected;
    public event Action<string>?      OnError;
    public event Action?              OnAuthenticated;
    public event Action<ClientState>? OnStateChanged;

    public DraoxClient(DraoxConfig? config = null)
    {
        _config  = config ?? new DraoxConfig();
        _syncCtx = SynchronizationContext.Current;
    }

    // ── Public API ────────────────────────────────────────────────────────────

    public async Task ConnectAsync(CancellationToken ct = default)
    {
        if (State is ClientState.Connected or ClientState.Connecting) return;

        _lifetimeCts = new CancellationTokenSource();
        SetState(ClientState.Connecting);

        _connection = _config.Protocol switch
        {
            DraoxProtocol.Tcp => new TcpConnection(),
            _                 => new WebSocketConnection(),
        };

        _broker      = new RequestBroker();
        _reconnector = new Reconnector(_config.Reconnect);

        _connection.MessageReceived += OnMessageReceived;
        _connection.Closed          += OnConnectionClosed;

        await _connection.ConnectAsync(_config, ct);

        SetState(ClientState.Connected);
        RaiseEvent(() => OnConnected?.Invoke());
        StartHeartbeat();
    }

    public async Task DisconnectAsync(string reason = "client_disconnect")
    {
        _heartbeatCts?.Cancel();
        _lifetimeCts?.Cancel();

        if (_connection is not null)
        {
            _broker?.FailAll(new DraoxException($"Disconnected: {reason}"));
            await _connection.DisconnectAsync();
            _connection = null;
        }

        SessionId = null;
        SetState(ClientState.Disconnected);
        RaiseEvent(() => OnDisconnected?.Invoke(reason));
    }

    public async Task AuthenticateAsync(string userId, string token, CancellationToken ct = default)
    {
        _savedUserId = userId;
        _savedToken  = token;

        var data = await RequestInternalAsync<AuthResponseData>(
            "auth", new { user_id = userId, token }, ct);
        SessionId = data?.SessionId;
        RaiseEvent(() => OnAuthenticated?.Invoke());
    }

    public async Task LoginAsync(string username, string password, CancellationToken ct = default)
    {
        _savedUsername = username;
        _savedPassword = password;

        var scheme = _config.UseTls ? "https" : "http";
        var url    = $"{scheme}://{_config.Host}:{_config.AdminPort}/api/auth/login";
        var body   = JsonSerializer.Serialize(new { username, password });
        using var req = new HttpRequestMessage(HttpMethod.Post, url)
        {
            Content = new StringContent(body, Encoding.UTF8, "application/json"),
        };
        using var resp = await _http.SendAsync(req, ct);
        var json = await resp.Content.ReadAsStringAsync(ct);

        if (!resp.IsSuccessStatusCode)
            throw new DraoxException($"Login failed ({(int)resp.StatusCode}): {json}");

        using var doc  = JsonDocument.Parse(json);
        var root = doc.RootElement;
        if (!root.TryGetProperty("success", out var ok) || !ok.GetBoolean())
            throw new DraoxException("Login failed: server returned success=false");

        var data     = root.GetProperty("data");
        var token    = data.GetProperty("token").GetString()
                       ?? throw new DraoxException("Login failed: missing token");
        var user     = data.TryGetProperty("username", out var u) ? u.GetString() ?? username : username;

        await AuthenticateAsync(user, token, ct);
    }

    // Fire-and-forget.
    public Task SendAsync(string action, object? payload = null, CancellationToken ct = default)
    {
        EnsureConnected();
        var json = Serializer.Serialize(
            new WireRequest { Id = NewId(), Action = action, Payload = Serializer.ToNode(payload) });
        return _connection!.SendTextAsync(json, ct);
    }

    // Awaited request/response.
    public async Task<T?> RequestAsync<T>(string action, object? payload = null, CancellationToken ct = default)
    {
        EnsureConnected();
        return await RequestInternalAsync<T>(action, payload, ct);
    }

    public void Subscribe(string eventName, Action<DraoxEvent> handler)
        => AddHandler(_eventHandlers, eventName, handler);

    public void Unsubscribe(string eventName, Action<DraoxEvent> handler)
        => RemoveHandler(_eventHandlers, eventName, handler);

    public void SubscribeCategory(string category, Action<DraoxEvent> handler)
        => AddHandler(_categoryHandlers, category, handler);

    public void UnsubscribeCategory(string category, Action<DraoxEvent> handler)
        => RemoveHandler(_categoryHandlers, category, handler);

    public void Dispose() => DisconnectAsync().GetAwaiter().GetResult();

    // ── Internal ──────────────────────────────────────────────────────────────

    private async Task<T?> RequestInternalAsync<T>(string action, object? payload, CancellationToken ct)
    {
        var id   = NewId();
        var json = Serializer.Serialize(
            new WireRequest { Id = id, Action = action, Payload = Serializer.ToNode(payload) });
        var res  = await _broker!.SendAsync(_connection!, json, id, _config.TimeoutMs, ct);
        if (!res.Success) throw new DraoxException(res.Error ?? "request failed");
        return Serializer.Deserialize<T>(res.RawData);
    }

    private void OnMessageReceived(string json)
    {
        var msg = Serializer.Parse(json);
        if (msg is null) return;

        switch (msg.Type)
        {
            case "response":
                _broker?.Complete(msg.Id!, new DraoxResponse
                {
                    Id      = msg.Id!,
                    Success = msg.Success,
                    RawData = msg.RawData,
                    Error   = msg.Error,
                });
                break;

            case "event":
                DispatchEvent(new DraoxEvent
                {
                    Category  = msg.Category ?? "",
                    Name      = msg.Name ?? "",
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
        TryDispatch(_eventHandlers,    $"{evt.Category}.{evt.Name}", evt);
        TryDispatch(_eventHandlers,    evt.Name,                     evt);
        TryDispatch(_categoryHandlers, evt.Category,                 evt);
    }

    private void TryDispatch(Dictionary<string, List<Action<DraoxEvent>>> dict, string key, DraoxEvent evt)
    {
        if (dict.TryGetValue(key, out var handlers))
            foreach (var h in handlers)
                RaiseEvent(() => h(evt));
    }

    private void OnConnectionClosed(string reason)
    {
        _heartbeatCts?.Cancel();
        SetState(ClientState.Disconnected);
        RaiseEvent(() => OnDisconnected?.Invoke(reason));

        if (_config.Reconnect.Enabled && _reconnector is not null
            && _lifetimeCts is { IsCancellationRequested: false })
            _ = TryReconnectAsync();
    }

    private async Task TryReconnectAsync()
    {
        SetState(ClientState.Reconnecting);

        var success = await _reconnector!.AttemptAsync(async () =>
        {
            try
            {
                await _connection!.ConnectAsync(_config, _lifetimeCts!.Token);
                if (_savedUsername is not null && _savedPassword is not null)
                    await LoginAsync(_savedUsername, _savedPassword, _lifetimeCts.Token);
                else if (!string.IsNullOrEmpty(_savedUserId))
                    await AuthenticateAsync(_savedUserId, _savedToken!, _lifetimeCts.Token);
                return true;
            }
            catch { return false; }
        }, _lifetimeCts!.Token);

        if (success)
        {
            SetState(ClientState.Connected);
            RaiseEvent(() => OnConnected?.Invoke());
            StartHeartbeat();
        }
    }

    private void StartHeartbeat()
    {
        _heartbeatCts?.Cancel();
        _heartbeatCts = new CancellationTokenSource();
        _ = HeartbeatLoopAsync(_heartbeatCts.Token);
    }

    private async Task HeartbeatLoopAsync(CancellationToken ct)
    {
        var interval = TimeSpan.FromSeconds(_config.HeartbeatIntervalSeconds);
        while (!ct.IsCancellationRequested)
        {
            try { await Task.Delay(interval, ct); }
            catch (OperationCanceledException) { break; }

            if (_connection is null || !_connection.IsConnected) break;

            _missedPings++;
            if (_missedPings >= 2) { OnConnectionClosed("heartbeat_timeout"); break; }

            var ping = Serializer.Serialize(new PingMessage { Ts = DateTimeOffset.UtcNow.ToUnixTimeMilliseconds() });
            try { await _connection.SendTextAsync(ping, ct); }
            catch { break; }
        }
    }

    private void EnsureConnected()
    {
        if (State != ClientState.Connected)
            throw new DraoxException($"Not connected (state: {State})");
    }

    private void RaiseEvent(Action action)
    {
        if (_syncCtx is not null && SynchronizationContext.Current != _syncCtx)
            _syncCtx.Post(_ => action(), null);
        else
            action();
    }

    private static void AddHandler(Dictionary<string, List<Action<DraoxEvent>>> dict, string key, Action<DraoxEvent> h)
    {
        if (!dict.TryGetValue(key, out var list)) dict[key] = list = new();
        list.Add(h);
    }

    private static void RemoveHandler(Dictionary<string, List<Action<DraoxEvent>>> dict, string key, Action<DraoxEvent> h)
    {
        if (dict.TryGetValue(key, out var list)) list.Remove(h);
    }

    private static string NewId() => $"req_{Guid.NewGuid():N}";

    private void SetState(ClientState state)
    {
        if (State == state) return;
        State = state;
        RaiseEvent(() => OnStateChanged?.Invoke(state));
    }
}
