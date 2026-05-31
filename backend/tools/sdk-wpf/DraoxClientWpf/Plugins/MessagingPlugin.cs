using System.Text.Json.Serialization;

namespace Draox.Client.Plugins;

/// <summary>
/// High-level helper for the built-in Messaging plugin.
/// Requires the server-side plugin-messaging crate to be loaded.
/// </summary>
public class MessagingPlugin
{
    private readonly DraoxClient _client;

    public MessagingPlugin(DraoxClient client) => _client = client;

    // ── Events ────────────────────────────────────────────────────────────────

    public event Action<MessageReceivedEvent>? OnMessage;
    public event Action<MessageDeletedEvent>?  OnMessageDeleted;
    public event Action<TypingEvent>?          OnTyping;

    public void RegisterListeners()   => _client.SubscribeCategory("msg", HandleMsgEvent);
    public void UnregisterListeners() => _client.UnsubscribeCategory("msg", HandleMsgEvent);

    // ── Request API ───────────────────────────────────────────────────────────

    public Task<SendMessageResponse?> SendMessageAsync(
        string channelId, string text, string? replyToId = null, CancellationToken ct = default)
        => _client.RequestAsync<SendMessageResponse>(
            "msg.send", new { channel_id = channelId, text, reply_to_id = replyToId }, ct);

    public Task<MessageHistoryResponse?> GetHistoryAsync(
        string channelId, int limit = 50, string? before = null, CancellationToken ct = default)
        => _client.RequestAsync<MessageHistoryResponse>(
            "msg.history", new { channel_id = channelId, limit, before }, ct);

    public Task DeleteMessageAsync(string messageId, CancellationToken ct = default)
        => _client.RequestAsync<object>("msg.delete", new { message_id = messageId }, ct);

    public Task<MessageDto?> EditMessageAsync(
        string messageId, string newText, CancellationToken ct = default)
        => _client.RequestAsync<MessageDto>(
            "msg.edit", new { message_id = messageId, text = newText }, ct);

    public Task SendTypingAsync(string channelId, CancellationToken ct = default)
        => _client.SendAsync("msg.typing", new { channel_id = channelId }, ct);

    public Task ReactAsync(string messageId, string emoji, CancellationToken ct = default)
        => _client.RequestAsync<object>("msg.react", new { message_id = messageId, emoji }, ct);

    // ── Internal ──────────────────────────────────────────────────────────────

    private void HandleMsgEvent(DraoxEvent evt)
    {
        switch (evt.Name)
        {
            case "received": OnMessage?.Invoke(evt.Data<MessageReceivedEvent>()!); break;
            case "deleted":  OnMessageDeleted?.Invoke(evt.Data<MessageDeletedEvent>()!); break;
            case "typing":   OnTyping?.Invoke(evt.Data<TypingEvent>()!); break;
        }
    }
}

// ── DTOs ──────────────────────────────────────────────────────────────────────

public class MessageDto
{
    [JsonPropertyName("id")]          public string  Id        { get; set; } = "";
    [JsonPropertyName("channel_id")]  public string  ChannelId { get; set; } = "";
    [JsonPropertyName("sender_id")]   public string  SenderId  { get; set; } = "";
    [JsonPropertyName("text")]        public string  Text      { get; set; } = "";
    [JsonPropertyName("reply_to_id")] public string? ReplyToId { get; set; }
    [JsonPropertyName("sent_at")]     public string  SentAt    { get; set; } = "";
    [JsonPropertyName("edited_at")]   public string? EditedAt  { get; set; }
}

public class SendMessageResponse
{
    [JsonPropertyName("message")] public MessageDto Message { get; set; } = new();
}

public class MessageHistoryResponse
{
    [JsonPropertyName("messages")]   public MessageDto[] Messages  { get; set; } = [];
    [JsonPropertyName("has_more")]   public bool         HasMore   { get; set; }
    [JsonPropertyName("oldest_id")]  public string?      OldestId  { get; set; }
}

public class MessageReceivedEvent
{
    [JsonPropertyName("message")] public MessageDto Message { get; set; } = new();
}

public class MessageDeletedEvent
{
    [JsonPropertyName("message_id")] public string MessageId { get; set; } = "";
    [JsonPropertyName("channel_id")] public string ChannelId { get; set; } = "";
}

public class TypingEvent
{
    [JsonPropertyName("channel_id")] public string ChannelId { get; set; } = "";
    [JsonPropertyName("user_id")]    public string UserId    { get; set; } = "";
    [JsonPropertyName("username")]   public string Username  { get; set; } = "";
    [JsonPropertyName("is_typing")]  public bool   IsTyping  { get; set; }
}
