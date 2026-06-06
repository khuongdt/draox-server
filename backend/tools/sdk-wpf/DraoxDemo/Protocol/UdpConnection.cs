using System.Net;
using System.Net.Sockets;
using System.Text;

namespace DraoxDemo.Protocol;

/// <summary>
/// UDP is stateless: no session resume, no heartbeat, no guaranteed delivery.
/// Each datagram must be a complete JSON message.
/// </summary>
public class UdpConnection : IConnection
{
    private UdpClient? _client;
    private IPEndPoint? _remote;
    private CancellationTokenSource? _cts;
    private volatile bool _connected;

    public bool IsConnected => _connected;
    public event Action<string>? MessageReceived;
    public event Action<string>? Disconnected;

    public Task ConnectAsync(string host, int port, bool useTls, CancellationToken ct = default)
    {
        // UDP ignores TLS — note in UI
        _remote = new IPEndPoint(Dns.GetHostAddresses(host)[0], port);
        _client = new UdpClient();
        _client.Connect(_remote);
        _connected = true;
        _cts = new CancellationTokenSource();
        _ = ReceiveLoopAsync(_cts.Token);
        return Task.CompletedTask;
    }

    public async Task SendAsync(string message, CancellationToken ct = default)
    {
        if (_client is null || !_connected) return;
        var bytes = Encoding.UTF8.GetBytes(message);
        await _client.SendAsync(bytes, bytes.Length);
    }

    public Task DisconnectAsync()
    {
        _connected = false;
        _cts?.Cancel();
        _client?.Dispose();
        return Task.CompletedTask;
    }

    private async Task ReceiveLoopAsync(CancellationToken ct)
    {
        try
        {
            while (!ct.IsCancellationRequested && _client is not null)
            {
                var result = await _client.ReceiveAsync(ct);
                var msg = Encoding.UTF8.GetString(result.Buffer);
                if (!string.IsNullOrWhiteSpace(msg))
                    MessageReceived?.Invoke(msg);
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

    public ValueTask DisposeAsync()
    {
        DisconnectAsync();
        return ValueTask.CompletedTask;
    }
}
