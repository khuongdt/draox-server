using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using DraoxDemo.Models;
using DraoxDemo.Services;
using DraoxDemo.Views;
using System.Collections.ObjectModel;
using System.Windows;

namespace DraoxDemo.ViewModels;

public partial class ClanDetailViewModel : ObservableObject
{
    private readonly ApiService _api;
    private readonly AppState _state;

    [ObservableProperty] private ClanDto? _clan;
    [ObservableProperty] private ObservableCollection<ClanMemberDto> _members = [];
    [ObservableProperty] private ObservableCollection<ChannelDto> _channels = [];
    [ObservableProperty] private bool _isLoading;
    [ObservableProperty] private string _errorMessage = string.Empty;

    public ClanDetailViewModel(ApiService api, AppState state)
    {
        _api = api;
        _state = state;
    }

    public void LoadClan(ClanDto clan)
    {
        Clan = clan;
        _ = LoadAsync();
    }

    [RelayCommand]
    private async Task LoadAsync()
    {
        if (Clan is null) return;
        IsLoading = true;
        ErrorMessage = string.Empty;
        try
        {
            var members = await _api.GetClanMembersAsync(Clan.Id);
            Members = new ObservableCollection<ClanMemberDto>(members);

            var channels = await _api.GetChannelsAsync();
            // Filter clan's own channels by name prefix convention or show all
            Channels = new ObservableCollection<ChannelDto>(channels);
        }
        catch (Exception ex) { ErrorMessage = ex.Message; }
        finally { IsLoading = false; }
    }

    [RelayCommand]
    private async Task LeaveAsync()
    {
        if (Clan is null) return;
        var confirm = MessageBox.Show($"Leave clan {Clan.Name}?", "Confirm",
            MessageBoxButton.YesNo, MessageBoxImage.Question);
        if (confirm != MessageBoxResult.Yes) return;
        try
        {
            await _api.LeaveClanAsync(Clan.Id);
            MainWindow.Current?.BackToClans();
        }
        catch (Exception ex) { ErrorMessage = ex.Message; }
    }

    [RelayCommand]
    private async Task DeleteAsync()
    {
        if (Clan is null) return;
        var confirm = MessageBox.Show($"Delete clan {Clan.Name}? This cannot be undone.", "Confirm Delete",
            MessageBoxButton.YesNo, MessageBoxImage.Warning);
        if (confirm != MessageBoxResult.Yes) return;
        try
        {
            await _api.DeleteClanAsync(Clan.Id);
            MainWindow.Current?.BackToClans();
        }
        catch (Exception ex) { ErrorMessage = ex.Message; }
    }

    [RelayCommand]
    private void OpenChat(ChannelDto channel)
    {
        MainWindow.Current?.OpenChat(channel);
    }

    [RelayCommand]
    private void OpenInvite()
    {
        if (Clan is null) return;
        var dialog = new Views.InviteDialog(_api, _state, Clan.Name);
        dialog.Owner = Application.Current.MainWindow;
        dialog.ShowDialog();
    }

    [RelayCommand]
    private void Back()
    {
        MainWindow.Current?.BackToClans();
    }
}
