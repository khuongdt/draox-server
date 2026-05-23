namespace Draox.Client;

internal interface IConnection
{
    bool IsConnected { get; }
    event Action<string>? MessageReceived;
    event Action<string>? Closed;

    Task ConnectAsync(DraoxConfig config, CancellationToken ct = default);
    Task DisconnectAsync();
    Task SendTextAsync(string json, CancellationToken ct = default);
}
