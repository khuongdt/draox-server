using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using DraoxDemo.Services;
using Microsoft.Extensions.DependencyInjection;
using System.Windows;

namespace DraoxDemo.ViewModels;

public partial class LoginViewModel : ObservableObject
{
    private readonly AppState _state;
    private readonly ApiService _api;
    private readonly SocketService _socket;

    [ObservableProperty] private string _host = "localhost";
    [ObservableProperty] private string _tcpPort = "9000";
    [ObservableProperty] private string _udpPort = "9001";
    [ObservableProperty] private string _wsPort = "9002";
    [ObservableProperty] private bool _useTls = false;
    [ObservableProperty] private bool _protocolTcp = true;
    [ObservableProperty] private bool _protocolWs = false;
    [ObservableProperty] private bool _protocolUdp = false;
    [ObservableProperty] private string _username = "admin";
    [ObservableProperty] private string _statusMessage = string.Empty;
    [ObservableProperty] private bool _isConnecting = false;
    [ObservableProperty] private bool _isError = false;
    [ObservableProperty] private bool _showUdpWarning = false;

    private string _password = string.Empty;
    public string Password
    {
        get => _password;
        set { _password = value; ConnectCommand.NotifyCanExecuteChanged(); }
    }

    // Auto-switches to the active protocol's port; settable to override per-protocol
    public string CurrentPort
    {
        get => ProtocolWs ? WsPort : ProtocolUdp ? UdpPort : TcpPort;
        set
        {
            if (ProtocolWs) WsPort = value;
            else if (ProtocolUdp) UdpPort = value;
            else TcpPort = value;
            OnPropertyChanged();
        }
    }

    partial void OnProtocolTcpChanged(bool value)
    {
        if (value) { ProtocolWs = false; ProtocolUdp = false; ShowUdpWarning = false; }
        OnPropertyChanged(nameof(CurrentPort));
    }

    partial void OnProtocolWsChanged(bool value)
    {
        if (value) { ProtocolTcp = false; ProtocolUdp = false; ShowUdpWarning = false; }
        OnPropertyChanged(nameof(CurrentPort));
    }

    partial void OnProtocolUdpChanged(bool value)
    {
        ShowUdpWarning = value;
        if (value) { ProtocolTcp = false; ProtocolWs = false; }
        OnPropertyChanged(nameof(CurrentPort));
    }

    public LoginViewModel(AppState state, ApiService api, SocketService socket)
    {
        _state = state;
        _api = api;
        _socket = socket;
    }

    [RelayCommand(CanExecute = nameof(CanConnect))]
    private async Task ConnectAsync()
    {
        IsConnecting = true;
        IsError = false;
        StatusMessage = "Logging in via HTTP...";

        try
        {
            // Apply config to AppState
            _state.Host = Host.Trim();
            _state.TcpPort = int.TryParse(TcpPort, out var tp) ? tp : 9000;
            _state.UdpPort = int.TryParse(UdpPort, out var up) ? up : 9001;
            _state.WsPort = int.TryParse(WsPort, out var wp) ? wp : 9002;
            // AdminPort stays at AppState default (9100)
            _state.UseTls = UseTls;
            _state.Protocol = ProtocolWs ? DraoxProtocol.WebSocket
                            : ProtocolUdp ? DraoxProtocol.Udp
                            : DraoxProtocol.Tcp;

            // Step 1: HTTP login → JWT
            var loginResult = await _api.LoginAsync(Username.Trim(), Password);
            if (loginResult is null)
                throw new Exception("Login failed: no response");

            _state.Token = loginResult.Token;
            _state.UserId = loginResult.Username;

            StatusMessage = $"Authenticated as {loginResult.Username}. Connecting socket ({_state.Protocol})...";

            // Step 2: socket connect + auth
            await _socket.ConnectAsync();
            var sessionId = await _socket.AuthAsync();
            _state.SessionId = sessionId;

            StatusMessage = $"Connected! Session: {sessionId}";

            // Open main window
            var mainWindow = App.Services.GetRequiredService<Views.MainWindow>();
            mainWindow.Show();

            // Close login window
            Application.Current.Windows.OfType<Views.LoginWindow>().FirstOrDefault()?.Close();
        }
        catch (Exception ex)
        {
            IsError = true;
            StatusMessage = $"Error: {ex.Message}";
        }
        finally
        {
            IsConnecting = false;
        }
    }

    private bool CanConnect() => !IsConnecting && !string.IsNullOrWhiteSpace(Username) && !string.IsNullOrWhiteSpace(Password);
}
