using System.Text.Json.Serialization;

namespace DraoxDemo.Models;

public class ChannelDto
{
    [JsonPropertyName("id")]
    public string Id { get; set; } = string.Empty;

    [JsonPropertyName("name")]
    public string Name { get; set; } = string.Empty;

    [JsonPropertyName("description")]
    public string? Description { get; set; }

    [JsonPropertyName("topic")]
    public string? Topic { get; set; }

    [JsonPropertyName("channel_type")]
    public string ChannelType { get; set; } = "Public";

    [JsonPropertyName("created_by")]
    public string CreatedBy { get; set; } = string.Empty;

    [JsonPropertyName("created_at")]
    public string CreatedAt { get; set; } = string.Empty;

    [JsonPropertyName("subscriber_count")]
    public int SubscriberCount { get; set; }

    [JsonPropertyName("is_system")]
    public bool IsSystem { get; set; }

    [JsonPropertyName("frozen")]
    public bool Frozen { get; set; }

    [JsonPropertyName("is_subscribed")]
    public bool IsSubscribed { get; set; }

    public string TypeIcon => ChannelType switch
    {
        "Private" => "🔒",
        "Announcement" => "📢",
        "Direct" => "💬",
        _ => "#"
    };

    public string TypeLabel => ChannelType?.ToUpperInvariant() ?? "PUBLIC";
}

public class CreateChannelRequest
{
    [JsonPropertyName("name")]
    public string Name { get; set; } = string.Empty;

    [JsonPropertyName("description")]
    public string? Description { get; set; }
}
