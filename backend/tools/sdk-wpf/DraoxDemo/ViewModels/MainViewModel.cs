using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using DraoxDemo.Services;
using Microsoft.Extensions.DependencyInjection;
using System.Windows;

namespace DraoxDemo.ViewModels;

public enum NavItem { Channels, Clans, ServerInfo }

public partial class MainViewModel : ObservableObject
{
    private readonly AppState _state;
    private readonly SocketService _socket;

    [ObservableProperty] private object? _currentView;
    [ObservableProperty] private NavItem _currentNav = NavItem.Channels;
    [ObservableProperty] private string _connectedAs = string.Empty;

    public MainViewModel(AppState state, SocketService socket)
    {
        _state = state;
        _socket = socket;
        ConnectedAs = $"{state.UserId} ({state.Protocol})";
    }

    public void Initialize()
    {
        NavigateTo(NavItem.Channels);
    }

    [RelayCommand]
    private void NavigateTo(NavItem nav)
    {
        CurrentNav = nav;
        CurrentView = nav switch
        {
            NavItem.Channels => App.Services.GetRequiredService<ChannelListViewModel>(),
            NavItem.Clans => App.Services.GetRequiredService<ClanListViewModel>(),
            NavItem.ServerInfo => App.Services.GetRequiredService<ServerInfoViewModel>(),
            _ => null
        };
    }

    public void OpenChat(Models.ChannelDto channel)
    {
        var vm = App.Services.GetRequiredService<ChatViewModel>();
        vm.LoadChannel(channel);
        CurrentView = vm;
    }

    public void OpenClanDetail(Models.ClanDto clan)
    {
        var vm = App.Services.GetRequiredService<ClanDetailViewModel>();
        vm.LoadClan(clan);
        CurrentView = vm;
    }

    public void BackToChannels() => NavigateTo(NavItem.Channels);
    public void BackToClans() => NavigateTo(NavItem.Clans);

    [RelayCommand]
    private async Task DisconnectAsync()
    {
        var result = MessageBox.Show("Disconnect and return to login?", "Disconnect",
            MessageBoxButton.YesNo, MessageBoxImage.Question);
        if (result != MessageBoxResult.Yes) return;

        await _socket.DisconnectAsync();
        _state.Token = null;
        _state.UserId = null;
        _state.SessionId = null;

        var loginWindow = App.Services.GetRequiredService<Views.LoginWindow>();
        loginWindow.Show();
        Application.Current.Windows.OfType<Views.MainWindow>().FirstOrDefault()?.Close();
    }
}
