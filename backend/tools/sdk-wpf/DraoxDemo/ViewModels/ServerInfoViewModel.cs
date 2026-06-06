using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using DraoxDemo.Models;
using DraoxDemo.Services;

namespace DraoxDemo.ViewModels;

public partial class ServerInfoViewModel : ObservableObject
{
    private readonly ApiService _api;
    private readonly AppState _state;

    [ObservableProperty] private ServerInfoDto? _serverInfo;
    [ObservableProperty] private HealthDto? _health;
    [ObservableProperty] private bool _isLoading;
    [ObservableProperty] private string _errorMessage = string.Empty;

    public string ConnectedAs => $"{_state.UserId} via {_state.Protocol}";
    public string ServerUrl => _state.AdminBaseUrl;

    public ServerInfoViewModel(ApiService api, AppState state)
    {
        _api = api;
        _state = state;
        _ = LoadAsync();
    }

    [RelayCommand]
    private async Task LoadAsync()
    {
        IsLoading = true;
        ErrorMessage = string.Empty;
        try
        {
            ServerInfo = await _api.GetServerInfoAsync();
            Health = await _api.GetHealthAsync();
        }
        catch (Exception ex)
        {
            ErrorMessage = ex.Message;
        }
        finally
        {
            IsLoading = false;
        }
    }
}
