using System.Net.WebSockets;
using System.Text;

namespace Draox.Client;

internal class WebSocketConnection : IConnection
{
    private ClientWebSocket? _ws;
    private CancellationTokenSource? _receiveCts;

    public event Action<string>? MessageReceived;
    public event Action<string>? Closed;

    public bool IsConnected => _ws?.State == WebSocketState.Open;

    public async Task ConnectAsync(DraoxConfig config, CancellationToken ct = default)
    {
        var scheme = config.UseTls ? "wss" : "ws";
        var uri = new Uri($"{scheme}://{config.Host}:{config.Port}");

        _ws = new ClientWebSocket();
        await _ws.ConnectAsync(uri, ct);

        _receiveCts = new CancellationTokenSource();
        _ = ReceiveLoopAsync(_receiveCts.Token);
    }

    public async Task DisconnectAsync()
    {
        _receiveCts?.Cancel();
        if (_ws is { State: WebSocketState.Open })
        {
            try { await _ws.CloseAsync(WebSocketCloseStatus.NormalClosure, "client_disconnect", CancellationToken.None); }
            catch { }
        }
        _ws?.Dispose();
        _ws = null;
    }

    public async Task SendTextAsync(string json, CancellationToken ct = default)
    {
        if (!IsConnected) throw new DraoxException("WebSocket is not connected");
        var bytes = Encoding.UTF8.GetBytes(json);
        await _ws!.SendAsync(bytes, WebSocketMessageType.Text, true, ct);
    }

    private async Task ReceiveLoopAsync(CancellationToken ct)
    {
        var buffer = new byte[64 * 1024];
        var sb = new StringBuilder();

        try
        {
            while (!ct.IsCancellationRequested && _ws?.State == WebSocketState.Open)
            {
                sb.Clear();
                WebSocketReceiveResult result;
                do
                {
                    result = await _ws.ReceiveAsync(buffer, ct);
                    if (result.MessageType == WebSocketMessageType.Close)
                    {
                        Closed?.Invoke("server_close");
                        return;
                    }
                    sb.Append(Encoding.UTF8.GetString(buffer, 0, result.Count));
                }
                while (!result.EndOfMessage);

                MessageReceived?.Invoke(sb.ToString());
            }
        }
        catch (OperationCanceledException) { }
        catch { Closed?.Invoke("receive_error"); }
    }
}
