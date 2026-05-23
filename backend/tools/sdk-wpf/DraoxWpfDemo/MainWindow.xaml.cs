using System.Collections.ObjectModel;
using System.Linq;
using System.Windows;
using System.Windows.Input;
using System.Windows.Media;
using Draox.Client;
using Draox.Client.Plugins;

namespace DraoxWpfDemo;

public partial class MainWindow : Window
{
    private DraoxClient?   _client;
    private MessagingPlugin? _messaging;
    private string _myUserId = "user_001";

    public ObservableCollection<ChatMessage> Messages { get; } = new();

    public MainWindow()
    {
        InitializeComponent();
        LvMessages.ItemsSource = Messages;
    }

    // ── Connection ────────────────────────────────────────────────────────────

    private async void BtnConnect_Click(object sender, RoutedEventArgs e)
    {
        SetConnectBusy(true);
        try
        {
            var config = new DraoxConfig
            {
                Host     = TxtHost.Text.Trim(),
                Port     = int.TryParse(TxtPort.Text, out var p) ? p : 9002,
                Protocol = CmbProtocol.SelectedIndex == 1 ? DraoxProtocol.Tcp : DraoxProtocol.WebSocket,
            };

            _client = new DraoxClient(config);
            _client.OnStateChanged  += s => UpdateStatus(s);
            _client.OnDisconnected  += r => AddSystem($"Disconnected: {r}");
            _client.OnAuthenticated += () =>
            {
                TxtSession.Text = $"Session: {_client.SessionId?[..8]}…";
                SetAuthState(true);
            };

            await _client.ConnectAsync();
            AddSystem($"Connected to {config.Host}:{config.Port} ({config.Protocol})");
            BtnDisconnect.IsEnabled = true;
            BtnAuth.IsEnabled       = true;
            BtnConnect.IsEnabled    = false;
        }
        catch (Exception ex)
        {
            AddSystem($"Connect failed: {ex.Message}", isError: true);
            SetConnectBusy(false);
        }
    }

    private async void BtnDisconnect_Click(object sender, RoutedEventArgs e)
    {
        _messaging?.UnregisterListeners();
        if (_client is not null) await _client.DisconnectAsync();
        _client   = null;
        _messaging = null;

        TxtSession.Text         = "";
        BtnConnect.IsEnabled    = true;
        BtnDisconnect.IsEnabled = false;
        BtnAuth.IsEnabled       = false;
        BtnHistory.IsEnabled    = false;
        SetInputEnabled(false);
        AddSystem("Disconnected.");
    }

    private async void BtnAuth_Click(object sender, RoutedEventArgs e)
    {
        if (_client is null) return;
        BtnAuth.IsEnabled = false;
        try
        {
            var username = TxtUsername.Text.Trim();
            var password = PwdPassword.Password;
            await _client.LoginAsync(username, password);
            _myUserId = username;

            _messaging = new MessagingPlugin(_client);
            _messaging.OnMessage        += OnMessageReceived;
            _messaging.OnMessageDeleted += OnMessageDeleted;
            _messaging.OnTyping         += OnTyping;
            _messaging.RegisterListeners();

            AddSystem($"Logged in as '{_myUserId}'");
            BtnHistory.IsEnabled = true;
            SetInputEnabled(true);
        }
        catch (Exception ex)
        {
            AddSystem($"Login failed: {ex.Message}", isError: true);
            BtnAuth.IsEnabled = true;
        }
    }

    // ── Channel / History ─────────────────────────────────────────────────────

    private async void BtnHistory_Click(object sender, RoutedEventArgs e)
    {
        if (_messaging is null) return;
        var channel = TxtChannel.Text.Trim();
        TxtChannelHeader.Text = channel;
        Messages.Clear();

        try
        {
            var history = await _messaging.GetHistoryAsync(channel, limit: 30);
            if (history?.Messages is { Length: > 0 } msgs)
            {
                foreach (var msg in Enumerable.Reverse(msgs))
                    AddIncoming(msg.SenderId, msg.Text, msg.SentAt);
                AddSystem($"Loaded {msgs.Length} messages.");
            }
            else
            {
                AddSystem("No messages in this channel.");
            }
        }
        catch (Exception ex)
        {
            AddSystem($"History failed: {ex.Message}", isError: true);
        }
    }

    // ── Send message ──────────────────────────────────────────────────────────

    private async void BtnSend_Click(object sender, RoutedEventArgs e) => await SendMessage();

    private async void TxtInput_KeyDown(object sender, KeyEventArgs e)
    {
        if (e.Key == Key.Enter && !Keyboard.IsKeyDown(Key.LeftShift))
        {
            e.Handled = true;
            await SendMessage();
        }
    }

    private async Task SendMessage()
    {
        if (_messaging is null) return;
        var text = TxtInput.Text.Trim();
        if (string.IsNullOrEmpty(text)) return;

        TxtInput.Clear();
        var channel = TxtChannel.Text.Trim();
        try
        {
            var resp = await _messaging.SendMessageAsync(channel, text);
            if (resp?.Message is { } m)
                AddOutgoing(_myUserId, m.Text, m.SentAt);
        }
        catch (Exception ex)
        {
            AddSystem($"Send failed: {ex.Message}", isError: true);
        }
    }

    // ── Incoming events ───────────────────────────────────────────────────────

    private void OnMessageReceived(MessageReceivedEvent evt)
    {
        var msg = evt.Message;
        // Skip echo of our own messages (already added by SendMessage above).
        if (msg.SenderId == _myUserId) return;
        AddIncoming(msg.SenderId, msg.Text, msg.SentAt);
    }

    private void OnMessageDeleted(MessageDeletedEvent evt)
        => AddSystem($"Message {evt.MessageId[..8]}… deleted in #{evt.ChannelId}");

    private void OnTyping(TypingEvent evt)
    {
        if (evt.IsTyping && evt.UserId != _myUserId)
            AddSystem($"{evt.Username} is typing…");
    }

    // ── UI helpers ────────────────────────────────────────────────────────────

    private void AddSystem(string text, bool isError = false)
    {
        var brush = isError
            ? new SolidColorBrush(Color.FromRgb(0xF3, 0x8B, 0xA8))
            : new SolidColorBrush(Color.FromRgb(0x6C, 0x70, 0x86));
        Append(new ChatMessage { Sender = "system", Text = text,
            Time = Now(), SenderColor = brush });
    }

    private void AddIncoming(string sender, string text, string? time = null)
        => Append(new ChatMessage { Sender = sender, Text = text,
            Time = FormatTime(time), SenderColor = new SolidColorBrush(Color.FromRgb(0x89, 0xB4, 0xFA)) });

    private void AddOutgoing(string sender, string text, string? time = null)
        => Append(new ChatMessage { Sender = sender, Text = text,
            Time = FormatTime(time), SenderColor = new SolidColorBrush(Color.FromRgb(0xA6, 0xE3, 0xA1)) });

    private void Append(ChatMessage msg)
    {
        Dispatcher.Invoke(() =>
        {
            Messages.Add(msg);
            LvMessages.ScrollIntoView(msg);
        });
    }

    private void UpdateStatus(ClientState state)
    {
        Dispatcher.Invoke(() =>
        {
            TxtStatus.Text = state.ToString();
            StatusDot.Fill = state switch
            {
                ClientState.Connected    => new SolidColorBrush(Color.FromRgb(0xA6, 0xE3, 0xA1)),
                ClientState.Reconnecting => new SolidColorBrush(Color.FromRgb(0xF9, 0xE2, 0xAF)),
                _                        => new SolidColorBrush(Color.FromRgb(0xF3, 0x8B, 0xA8)),
            };
        });
    }

    private void SetConnectBusy(bool connecting)
    {
        BtnConnect.IsEnabled = !connecting;
    }

    private void SetAuthState(bool authed)
    {
        Dispatcher.Invoke(() =>
        {
            BtnAuth.IsEnabled = !authed;
        });
    }

    private void SetInputEnabled(bool enabled)
    {
        TxtInput.IsEnabled  = enabled;
        BtnSend.IsEnabled   = enabled;
        BtnHistory.IsEnabled = enabled;
    }

    private static string Now() => DateTime.Now.ToString("HH:mm:ss");

    private static string FormatTime(string? iso)
    {
        if (string.IsNullOrEmpty(iso)) return Now();
        return DateTime.TryParse(iso, out var dt) ? dt.ToLocalTime().ToString("HH:mm") : Now();
    }
}

// ── Message model ─────────────────────────────────────────────────────────────

public class ChatMessage
{
    public string Sender { get; set; } = "";
    public string Text   { get; set; } = "";
    public string Time   { get; set; } = "";
    public Brush  SenderColor { get; set; } = Brushes.Gray;
}
