namespace DraoxDemo.Protocol;

public interface IConnection : IAsyncDisposable
{
    bool IsConnected { get; }
    event Action<string>? MessageReceived;
    event Action<string>? Disconnected;

    Task ConnectAsync(string host, int port, bool useTls, CancellationToken ct = default);
    Task SendAsync(string message, CancellationToken ct = default);
    Task DisconnectAsync();
}
