using System.Text.Json.Serialization;

namespace DraoxDemo.Models;

public class MessageDto
{
    [JsonPropertyName("id")]
    public string Id { get; set; } = string.Empty;

    [JsonPropertyName("channel_id")]
    public string ChannelId { get; set; } = string.Empty;

    [JsonPropertyName("sender_id")]
    public string SenderId { get; set; } = string.Empty;

    [JsonPropertyName("text")]
    public string Text { get; set; } = string.Empty;

    [JsonPropertyName("reply_to_id")]
    public string? ReplyToId { get; set; }

    [JsonPropertyName("sender_name")]
    public string SenderName { get; set; } = string.Empty;

    [JsonPropertyName("sent_at")]
    public string SentAt { get; set; } = string.Empty;

    [JsonPropertyName("edited_at")]
    public string? EditedAt { get; set; }

    [JsonPropertyName("reactions")]
    public List<ReactionDto> Reactions { get; set; } = [];

    // Derived: true if text starts with [image] prefix
    [JsonIgnore]
    public bool IsImage => Text.StartsWith("[image]", StringComparison.OrdinalIgnoreCase);
    [JsonIgnore]
    public string ImageUrl => IsImage ? Text[7..] : string.Empty;
    [JsonIgnore]
    public string DisplayText => IsImage ? $"🖼 {ImageUrl}" : Text;
    [JsonIgnore]
    public bool IsEdited => EditedAt != null;

    // Set by ChatViewModel after loading — not from server JSON
    [JsonIgnore]
    public bool IsOwn { get; set; }

    [JsonIgnore]
    public DateTime CreatedAt
    {
        get
        {
            if (DateTime.TryParse(SentAt, out var dt))
                return dt.ToLocalTime();
            return DateTime.MinValue;
        }
    }
}

public class ReactionDto
{
    [JsonPropertyName("emoji")]
    public string Emoji { get; set; } = string.Empty;

    [JsonPropertyName("users")]
    public List<string> Users { get; set; } = [];

    public int Count => Users.Count;
}

public class SendMessageRequest
{
    [JsonPropertyName("channel_id")]
    public string ChannelId { get; set; } = string.Empty;

    [JsonPropertyName("text")]
    public string Text { get; set; } = string.Empty;

    [JsonPropertyName("reply_to_id")]
    public string? ReplyToId { get; set; }
}

public class MessageHistoryResponse
{
    [JsonPropertyName("messages")]
    public List<MessageDto> Messages { get; set; } = [];

    [JsonPropertyName("has_more")]
    public bool HasMore { get; set; }

    [JsonPropertyName("oldest_id")]
    public string? OldestId { get; set; }
}

public class EditMessageRequest
{
    [JsonPropertyName("text")]
    public string Text { get; set; } = string.Empty;
}

public class ReactRequest
{
    [JsonPropertyName("emoji")]
    public string Emoji { get; set; } = string.Empty;
}
