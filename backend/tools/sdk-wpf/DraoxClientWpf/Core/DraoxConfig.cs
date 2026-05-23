using System.Text.Json.Nodes;
using System.Text.Json.Serialization;

namespace Draox.Client;

public enum DraoxProtocol { WebSocket, Tcp }
public enum ConnectionRole { Primary, Notification, Control, Streaming }
public enum ClientState { Disconnected, Connecting, Connected, Reconnecting }

public class DraoxConfig
{
    public string Host { get; set; } = "localhost";
    public int Port { get; set; } = 9002;
    public int AdminPort { get; set; } = 9100;
    public DraoxProtocol Protocol { get; set; } = DraoxProtocol.WebSocket;
    public bool UseTls { get; set; } = false;
    public int TimeoutMs { get; set; } = 10_000;
    public int HeartbeatIntervalSeconds { get; set; } = 30;
    public ReconnectConfig Reconnect { get; set; } = new();
}

public class ReconnectConfig
{
    public bool Enabled { get; set; } = true;
    public int MaxAttempts { get; set; } = 5;
    public double BaseDelaySeconds { get; set; } = 1.0;
    public double MaxDelaySeconds { get; set; } = 30.0;
}

// ── Public SDK types ──────────────────────────────────────────────────────────

public class DraoxResponse
{
    public string Id { get; init; } = "";
    public bool Success { get; init; }
    public string? RawData { get; init; }
    public string? Error { get; init; }

    public T? Data<T>() => Serializer.Deserialize<T>(RawData);
}

public class DraoxEvent
{
    public string Category { get; init; } = "";
    public string Name { get; init; } = "";
    public string? RawData { get; init; }
    public string? Timestamp { get; init; }

    public T? Data<T>() => Serializer.Deserialize<T>(RawData);
}

public class DraoxException : Exception
{
    public DraoxException(string message) : base(message) { }
}

public class DraoxTimeoutException : DraoxException
{
    public DraoxTimeoutException(string requestId) : base($"Request '{requestId}' timed out") { }
}

// ── Internal wire protocol ────────────────────────────────────────────────────

internal class WireRequest
{
    [JsonPropertyName("id")]      public string Id { get; init; } = "";
    [JsonPropertyName("type")]    public string Type { get; init; } = "request";
    [JsonPropertyName("action")]  public string Action { get; init; } = "";
    [JsonPropertyName("payload")] public JsonNode? Payload { get; init; }
}

internal class PingMessage
{
    [JsonPropertyName("type")] public string Type { get; init; } = "ping";
    [JsonPropertyName("ts")]   public long Ts { get; init; }
}

internal class AuthResponseData
{
    [JsonPropertyName("session_id")] public string? SessionId { get; set; }
}
