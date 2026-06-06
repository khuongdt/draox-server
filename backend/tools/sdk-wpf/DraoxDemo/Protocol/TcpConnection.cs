using System.IO;
using System.Net.Security;
using System.Net.Sockets;
using System.Text;

namespace DraoxDemo.Protocol;

public class TcpConnection : IConnection
{
    private TcpClient? _client;
    private StreamReader? _reader;
    private StreamWriter? _writer;
    private CancellationTokenSource? _cts;
    private volatile bool _connected;

    public bool IsConnected => _connected;
    public event Action<string>? MessageReceived;
    public event Action<string>? Disconnected;

    public async Task ConnectAsync(string host, int port, bool useTls, CancellationToken ct = default)
    {
        _client = new TcpClient { NoDelay = true };
        await _client.ConnectAsync(host, port, ct);

        Stream stream = _client.GetStream();
        if (useTls)
        {
            var ssl = new SslStream(stream, false, (_, _, _, _) => true);
            await ssl.AuthenticateAsClientAsync(host, null, false);
            stream = ssl;
        }

        var encoding = new UTF8Encoding(encoderShouldEmitUTF8Identifier: false);
        _reader = new StreamReader(stream, encoding);
        _writer = new StreamWriter(stream, encoding) { AutoFlush = true, NewLine = "\n" };

        _connected = true;
        _cts = new CancellationTokenSource();
        _ = ReceiveLoopAsync(_cts.Token);
    }

    public async Task SendAsync(string message, CancellationToken ct = default)
    {
        if (_writer is null || !_connected) return;
        await _writer.WriteLineAsync(message.AsMemory(), ct);
    }

    public Task DisconnectAsync()
    {
        _connected = false;
        _cts?.Cancel();
        _reader?.Dispose();
        _writer?.Dispose();
        _client?.Dispose();
        return Task.CompletedTask;
    }

    private async Task ReceiveLoopAsync(CancellationToken ct)
    {
        try
        {
            while (!ct.IsCancellationRequested && _reader is not null)
            {
                var line = await _reader.ReadLineAsync(ct);
                if (line is null) break;
                if (!string.IsNullOrWhiteSpace(line))
                    MessageReceived?.Invoke(line);
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
