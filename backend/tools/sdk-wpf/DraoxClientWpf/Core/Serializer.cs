using System.Text.Json;
using System.Text.Json.Nodes;

namespace Draox.Client;

internal static class Serializer
{
    private static readonly JsonSerializerOptions Opts = new()
    {
        DefaultIgnoreCondition = System.Text.Json.Serialization.JsonIgnoreCondition.WhenWritingNull,
        PropertyNamingPolicy = JsonNamingPolicy.CamelCase,
    };

    public static string Serialize(object obj) =>
        JsonSerializer.Serialize(obj, obj.GetType(), Opts);

    public static T? Deserialize<T>(string? json)
    {
        if (string.IsNullOrEmpty(json)) return default;
        return JsonSerializer.Deserialize<T>(json, Opts);
    }

    // Converts an arbitrary object to JsonNode so it serializes with its runtime type.
    public static JsonNode? ToNode(object? obj)
    {
        if (obj is null) return null;
        var json = JsonSerializer.Serialize(obj, obj.GetType(), Opts);
        return JsonNode.Parse(json);
    }

    public static ParsedMessage? Parse(string? json)
    {
        if (string.IsNullOrEmpty(json)) return null;

        JsonNode? node;
        try { node = JsonNode.Parse(json); }
        catch { return null; }

        if (node is null) return null;
        var type = node["type"]?.GetValue<string>();

        return type switch
        {
            "response" => new ParsedMessage
            {
                Type    = "response",
                Id      = node["id"]?.GetValue<string>(),
                Success = node["success"]?.GetValue<bool>() ?? false,
                RawData = node["data"]?.ToJsonString(),
                Error   = node["error"]?.GetValue<string>(),
            },
            "event" => new ParsedMessage
            {
                Type      = "event",
                Category  = node["category"]?.GetValue<string>(),
                Name      = node["name"]?.GetValue<string>(),
                RawData   = node["data"]?.ToJsonString(),
                Timestamp = node["timestamp"]?.GetValue<string>(),
            },
            "pong" => new ParsedMessage { Type = "pong" },
            _      => new ParsedMessage { Type = type ?? "unknown" },
        };
    }
}

internal class ParsedMessage
{
    public string? Type;
    public string? Id;
    public bool    Success;
    public string? RawData;
    public string? Error;
    public string? Category;
    public string? Name;
    public string? Timestamp;
}
