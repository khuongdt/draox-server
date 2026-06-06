using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using DraoxDemo.Models;
using DraoxDemo.Protocol;
using DraoxDemo.Services;
using DraoxDemo.Views;
using System.Collections.ObjectModel;
using System.Windows;

namespace DraoxDemo.ViewModels;

public partial class ChatViewModel : ObservableObject
{
    private readonly ApiService _api;
    private readonly SocketService _socket;
    private readonly AppState _state;
    private System.Timers.Timer? _typingDebounce;
    private string? _oldestId;

    [ObservableProperty] private ChannelDto? _channel;
    [ObservableProperty] private ObservableCollection<MessageDto> _messages = [];
    [ObservableProperty] private string _messageText = string.Empty;
    [ObservableProperty] private string? _replyToId;
    [ObservableProperty] private string _replyToText = string.Empty;
    [ObservableProperty] private ObservableCollection<string> _typingUsers = [];
    [ObservableProperty] private bool _isLoadingMore;
    [ObservableProperty] private bool _hasMore;
    [ObservableProperty] private string _errorMessage = string.Empty;

    public AppState AppState => _state;

    partial void OnMessageTextChanged(string value)
    {
        if (!string.IsNullOrWhiteSpace(value)) TriggerTyping();
    }

    public ChatViewModel(ApiService api, SocketService socket, AppState state)
    {
        _api = api;
        _socket = socket;
        _state = state;
        _socket.EventReceived += OnSocketEvent;
    }

    public void LoadChannel(ChannelDto channel)
    {
        Channel = channel;
        Messages.Clear();
        _oldestId = null;
        _ = LoadHistoryAsync();
    }

    private async Task LoadHistoryAsync()
    {
        if (Channel is null) return;
        IsLoadingMore = true;
        try
        {
            var resp = await _api.GetMessagesAsync(Channel.Id, 50, _oldestId);
            if (resp is not null)
            {
                HasMore = resp.HasMore;
                _oldestId = resp.OldestId;
                foreach (var msg in resp.Messages)
                {
                    msg.IsOwn = msg.SenderId == _state.UserId;
                    Messages.Insert(0, msg);
                }
            }
        }
        catch (Exception ex) { ErrorMessage = ex.Message; }
        finally { IsLoadingMore = false; }
    }

    [RelayCommand]
    private async Task LoadMoreAsync()
    {
        if (!HasMore || IsLoadingMore) return;
        await LoadHistoryAsync();
    }

    [RelayCommand(CanExecute = nameof(CanSend))]
    private async Task SendAsync()
    {
        if (Channel is null || string.IsNullOrWhiteSpace(MessageText)) return;
        var text = MessageText.Trim();
        MessageText = string.Empty;
        ReplyToId = null;
        ReplyToText = string.Empty;

        try
        {
            var msg = await _api.SendMessageAsync(Channel.Id, text, ReplyToId);
            if (msg is not null && Messages.All(m => m.Id != msg.Id))
                Messages.Add(msg);
        }
        catch (Exception ex) { ErrorMessage = ex.Message; }
    }

    private bool CanSend() => !string.IsNullOrWhiteSpace(MessageText) && Channel is not null;

    [RelayCommand]
    private void SetReplyTo(MessageDto msg)
    {
        ReplyToId = msg.Id;
        ReplyToText = msg.DisplayText.Length > 50 ? msg.DisplayText[..50] + "…" : msg.DisplayText;
    }

    [RelayCommand]
    private void ClearReply()
    {
        ReplyToId = null;
        ReplyToText = string.Empty;
    }

    [RelayCommand]
    private async Task DeleteMessageAsync(string messageId)
    {
        var confirm = MessageBox.Show("Delete this message?", "Confirm", MessageBoxButton.YesNo, MessageBoxImage.Warning);
        if (confirm != MessageBoxResult.Yes) return;
        try
        {
            await _api.DeleteMessageAsync(messageId);
            var msg = Messages.FirstOrDefault(m => m.Id == messageId);
            if (msg is not null) Messages.Remove(msg);
        }
        catch (Exception ex) { ErrorMessage = ex.Message; }
    }

    [RelayCommand]
    private async Task EditMessageAsync(MessageDto msg)
    {
        var dialog = new Views.EditMessageDialog(msg.Text);
        if (dialog.ShowDialog() != true) return;
        try
        {
            var updated = await _api.EditMessageAsync(msg.Id, dialog.NewText);
            if (updated is not null)
            {
                var idx = Messages.IndexOf(msg);
                if (idx >= 0) Messages[idx] = updated;
            }
        }
        catch (Exception ex) { ErrorMessage = ex.Message; }
    }

    [RelayCommand]
    private async Task ReactAsync(ReactParam param)
    {
        try { await _api.ReactAsync(param.MessageId, param.Emoji); }
        catch (Exception ex) { ErrorMessage = ex.Message; }
    }

    [RelayCommand]
    private void Back()
    {
        _socket.EventReceived -= OnSocketEvent;
        MainWindow.Current?.BackToChannels();
    }

    private void TriggerTyping()
    {
        _typingDebounce?.Stop();
        _typingDebounce = new System.Timers.Timer(800) { AutoReset = false };
        _typingDebounce.Elapsed += async (_, _) =>
        {
            if (Channel is not null)
                await _socket.SendTypingAsync(Channel.Id);
        };
        _typingDebounce.Start();
    }

    private void OnSocketEvent(WireEvent evt)
    {
        if (Channel is null) return;

        Application.Current.Dispatcher.Invoke(() =>
        {
            if (evt.Category == "messaging" && evt.Name == "message_received")
            {
                var data = evt.GetData<MessageReceivedEventData>();
                if (data?.Message is not null && data.Message.ChannelId == Channel.Id
                    && Messages.All(m => m.Id != data.Message.Id))
                {
                    data.Message.IsOwn = data.Message.SenderId == _state.UserId;
                    Messages.Add(data.Message);
                }
            }
            else if (evt.Category == "messaging" && evt.Name == "message_deleted")
            {
                var data = evt.GetData<MessageDeletedEventData>();
                if (data is not null)
                {
                    var msg = Messages.FirstOrDefault(m => m.Id == data.MessageId);
                    if (msg is not null) Messages.Remove(msg);
                }
            }
            else if (evt.Category == "messaging" && evt.Name == "typing")
            {
                var data = evt.GetData<TypingEventData>();
                if (data?.ChannelId == Channel.Id && data.UserId != _state.UserId)
                {
                    if (data.IsTyping && !TypingUsers.Contains(data.Username))
                        TypingUsers.Add(data.Username);
                    else if (!data.IsTyping)
                        TypingUsers.Remove(data.Username);
                }
            }
        });
    }

    private class MessageReceivedEventData
    {
        [System.Text.Json.Serialization.JsonPropertyName("message")]
        public MessageDto? Message { get; set; }
    }

    private class MessageDeletedEventData
    {
        [System.Text.Json.Serialization.JsonPropertyName("message_id")]
        public string MessageId { get; set; } = string.Empty;
        [System.Text.Json.Serialization.JsonPropertyName("channel_id")]
        public string ChannelId { get; set; } = string.Empty;
    }

    private class TypingEventData
    {
        [System.Text.Json.Serialization.JsonPropertyName("channel_id")]
        public string ChannelId { get; set; } = string.Empty;
        [System.Text.Json.Serialization.JsonPropertyName("user_id")]
        public string UserId { get; set; } = string.Empty;
        [System.Text.Json.Serialization.JsonPropertyName("username")]
        public string Username { get; set; } = string.Empty;
        [System.Text.Json.Serialization.JsonPropertyName("is_typing")]
        public bool IsTyping { get; set; }
    }
}

public record ReactParam(string MessageId, string Emoji);
