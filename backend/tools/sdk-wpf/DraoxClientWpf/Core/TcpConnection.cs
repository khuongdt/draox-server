using System.Net.Sockets;
using System.Text;

namespace Draox.Client;

// Line-delimited JSON over TCP — each message is one UTF-8 line terminated by \n.
internal class TcpConnection : IConnection
{
    private TcpClient? _tcp;
    private StreamReader? _reader;
    private StreamWriter? _writer;
    private CancellationTokenSource? _receiveCts;

    public event Action<string>? MessageReceived;
    public event Action<string>? Closed;

    public bool IsConnected => _tcp?.Connected == true;

    public async Task ConnectAsync(DraoxConfig config, CancellationToken ct = default)
    {
        _tcp = new TcpClient();
        await _tcp.ConnectAsync(config.Host, config.Port, ct);

        var stream = _tcp.GetStream();
        _reader = new StreamReader(stream, Encoding.UTF8);
        _writer = new StreamWriter(stream, Encoding.UTF8) { AutoFlush = true };

        _receiveCts = new CancellationTokenSource();
        _ = ReceiveLoopAsync(_receiveCts.Token);
    }

    public Task DisconnectAsync()
    {
        _receiveCts?.Cancel();
        _reader?.Dispose();
        _writer?.Dispose();
        _tcp?.Dispose();
        _tcp = null;
        return Task.CompletedTask;
    }

    public async Task SendTextAsync(string json, CancellationToken ct = default)
    {
        if (_writer is null) throw new DraoxException("TCP is not connected");
        await _writer.WriteLineAsync(json.AsMemory(), ct);
    }

    private async Task ReceiveLoopAsync(CancellationToken ct)
    {
        try
        {
            while (!ct.IsCancellationRequested && _reader is not null)
            {
                var line = await _reader.ReadLineAsync(ct);
                if (line is null) break;
                MessageReceived?.Invoke(line);
            }
        }
        catch (OperationCanceledException) { }
        catch { }
        finally { Closed?.Invoke("tcp_closed"); }
    }
}
