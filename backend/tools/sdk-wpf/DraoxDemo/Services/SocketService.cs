using DraoxDemo.Protocol;

namespace DraoxDemo.Services;

public class SocketService : IAsyncDisposable
{
    private readonly AppState _state;
    private readonly RequestBroker _broker = new();
    private IConnection? _connection;
    private CancellationTokenSource? _heartbeatCts;

    public event Action<WireEvent>? EventReceived;
    public event Action<string>? Disconnected;
    public bool IsConnected => _connection?.IsConnected == true;

    public SocketService(AppState state)
    {
        _state = state;
    }

    public async Task ConnectAsync(CancellationToken ct = default)
    {
        _connection = _state.Protocol switch
        {
            DraoxProtocol.WebSocket => new WsConnection(),
            DraoxProtocol.Udp => new UdpConnection(),
            _ => new TcpConnection(),
        };

        _connection.MessageReceived += OnRawMessage;
        _connection.Disconnected += reason =>
        {
            _heartbeatCts?.Cancel();
            _broker.FailAll(new Exception("Disconnected: " + reason));
            Disconnected?.Invoke(reason);
        };

        await _connection.ConnectAsync(_state.Host, _state.SocketPort, _state.UseTls, ct);

        if (_state.Protocol != DraoxProtocol.Udp)
            StartHeartbeat();
    }

    public async Task<string?> AuthAsync(CancellationToken ct = default)
    {
        var resp = await RequestAsync("auth", new
        {
            user_id = _state.UserId,
            token = _state.Token,
        }, ct);

        if (resp.Success)
        {
            var data = resp.GetData<AuthResponse>();
            return data?.SessionId;
        }
        throw new Exception(resp.Error ?? "Auth failed");
    }

    public async Task<WireResponse> RequestAsync(string action, object? payload = null, CancellationToken ct = default)
    {
        if (_connection is null) throw new InvalidOperationException("Not connected");

        var (id, task) = _broker.CreatePending();
        var req = new WireRequest
        {
            Id = id,
            Action = action,
            Payload = payload,
            Token = _state.Token,
        };
        await _connection.SendAsync(WireSerializer.Serialize(req), ct);
        return await task.WaitAsync(ct);
    }

    public async Task SendTypingAsync(string channelId)
    {
        if (_connection is null || !IsConnected) return;
        var req = new WireRequest
        {
            Id = $"req_{Guid.NewGuid():N}",
            Action = "messaging.typing",
            Payload = new { channel_id = channelId },
            Token = _state.Token,
        };
        await _connection.SendAsync(WireSerializer.Serialize(req));
    }

    public async Task DisconnectAsync()
    {
        _heartbeatCts?.Cancel();
        if (_connection is not null)
            await _connection.DisconnectAsync();
    }

    private void OnRawMessage(string raw)
    {
        try
        {
            using var doc = System.Text.Json.JsonDocument.Parse(raw);
            var type = doc.RootElement.GetProperty("type").GetString();

            if (type == "response")
            {
                var resp = WireSerializer.Deserialize<WireResponse>(raw);
                if (resp is not null) _broker.TryComplete(resp);
            }
            else if (type == "event")
            {
                var evt = WireSerializer.Deserialize<WireEvent>(raw);
                if (evt is not null) EventReceived?.Invoke(evt);
            }
            // pong: ignore
        }
        catch { }
    }

    private void StartHeartbeat()
    {
        _heartbeatCts = new CancellationTokenSource();
        var ct = _heartbeatCts.Token;
        _ = Task.Run(async () =>
        {
            while (!ct.IsCancellationRequested)
            {
                await Task.Delay(30_000, ct);
                if (_connection?.IsConnected == true)
                {
                    var ping = WireSerializer.Serialize(new WirePing { Ts = DateTimeOffset.UtcNow.ToUnixTimeMilliseconds() });
                    await _connection.SendAsync(ping);
                }
            }
        }, ct);
    }

    public async ValueTask DisposeAsync()
    {
        await DisconnectAsync();
        if (_connection is not null)
            await _connection.DisposeAsync();
    }

    private class AuthResponse
    {
        [System.Text.Json.Serialization.JsonPropertyName("session_id")]
        public string? SessionId { get; set; }
    }
}
