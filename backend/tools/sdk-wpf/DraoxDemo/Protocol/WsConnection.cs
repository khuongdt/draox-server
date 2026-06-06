using System.Net.WebSockets;
using System.Text;

namespace DraoxDemo.Protocol;

public class WsConnection : IConnection
{
    private ClientWebSocket? _ws;
    private CancellationTokenSource? _cts;
    private volatile bool _connected;

    public bool IsConnected => _connected;
    public event Action<string>? MessageReceived;
    public event Action<string>? Disconnected;

    public async Task ConnectAsync(string host, int port, bool useTls, CancellationToken ct = default)
    {
        _ws = new ClientWebSocket();
        var scheme = useTls ? "wss" : "ws";
        var uri = new Uri($"{scheme}://{host}:{port}");
        await _ws.ConnectAsync(uri, ct);
        _connected = true;
        _cts = new CancellationTokenSource();
        _ = ReceiveLoopAsync(_cts.Token);
    }

    public async Task SendAsync(string message, CancellationToken ct = default)
    {
        if (_ws is null || !_connected) return;
        var bytes = Encoding.UTF8.GetBytes(message);
        await _ws.SendAsync(bytes, WebSocketMessageType.Text, true, ct);
    }

    public async Task DisconnectAsync()
    {
        _connected = false;
        _cts?.Cancel();
        if (_ws?.State == WebSocketState.Open)
        {
            try { await _ws.CloseAsync(WebSocketCloseStatus.NormalClosure, "bye", CancellationToken.None); }
            catch { }
        }
        _ws?.Dispose();
    }

    private async Task ReceiveLoopAsync(CancellationToken ct)
    {
        var buffer = new byte[65536];
        var sb = new StringBuilder();
        try
        {
            while (!ct.IsCancellationRequested && _ws?.State == WebSocketState.Open)
            {
                var result = await _ws.ReceiveAsync(buffer, ct);
                if (result.MessageType == WebSocketMessageType.Close) break;

                sb.Append(Encoding.UTF8.GetString(buffer, 0, result.Count));
                if (result.EndOfMessage)
                {
                    var msg = sb.ToString();
                    sb.Clear();
                    if (!string.IsNullOrWhiteSpace(msg))
                        MessageReceived?.Invoke(msg);
                }
            }
        }
        catch (OperationCanceledException) { }
        catch (Exception) { }
        finally
        {
            if (_connected)
            {
                _connected = false;
                Disconnected?.Invoke("connection_lost");
            }
        }
    }

    public async ValueTask DisposeAsync()
    {
        await DisconnectAsync();
    }
}
