using System.Text.Json;
using System.Text.Json.Serialization;

namespace DraoxDemo.Protocol;

public class WireRequest
{
    [JsonPropertyName("id")]
    public string Id { get; set; } = string.Empty;

    [JsonPropertyName("type")]
    public string Type { get; set; } = "request";

    [JsonPropertyName("action")]
    public string Action { get; set; } = string.Empty;

    [JsonPropertyName("payload")]
    public object? Payload { get; set; }

    [JsonPropertyName("token")]
    public string? Token { get; set; }
}

public class WireResponse
{
    [JsonPropertyName("type")]
    public string Type { get; set; } = string.Empty;

    [JsonPropertyName("id")]
    public string Id { get; set; } = string.Empty;

    [JsonPropertyName("success")]
    public bool Success { get; set; }

    [JsonPropertyName("data")]
    public JsonElement? Data { get; set; }

    [JsonPropertyName("error")]
    public string? Error { get; set; }

    public T? GetData<T>()
    {
        if (Data is null) return default;
        return JsonSerializer.Deserialize<T>(Data.Value.GetRawText(), WireSerializer.Options);
    }
}

public class WireEvent
{
    [JsonPropertyName("type")]
    public string Type { get; set; } = string.Empty;

    [JsonPropertyName("category")]
    public string Category { get; set; } = string.Empty;

    [JsonPropertyName("name")]
    public string Name { get; set; } = string.Empty;

    [JsonPropertyName("data")]
    public JsonElement? Data { get; set; }

    [JsonPropertyName("timestamp")]
    public string? Timestamp { get; set; }

    public T? GetData<T>()
    {
        if (Data is null) return default;
        return JsonSerializer.Deserialize<T>(Data.Value.GetRawText(), WireSerializer.Options);
    }
}

public class WirePing
{
    [JsonPropertyName("type")]
    public string Type { get; set; } = "ping";

    [JsonPropertyName("ts")]
    public long Ts { get; set; }
}

public static class WireSerializer
{
    public static readonly JsonSerializerOptions Options = new()
    {
        PropertyNamingPolicy = JsonNamingPolicy.SnakeCaseLower,
        DefaultIgnoreCondition = JsonIgnoreCondition.WhenWritingNull,
    };

    public static string Serialize<T>(T obj) => JsonSerializer.Serialize(obj, Options);

    public static T? Deserialize<T>(string json) => JsonSerializer.Deserialize<T>(json, Options);
}
