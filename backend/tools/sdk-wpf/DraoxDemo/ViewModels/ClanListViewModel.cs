using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using DraoxDemo.Models;
using DraoxDemo.Services;
using DraoxDemo.Views;
using System.Collections.ObjectModel;
using System.Windows;

namespace DraoxDemo.ViewModels;

public partial class ClanListViewModel : ObservableObject
{
    private readonly ApiService _api;
    private readonly AppState _state;

    [ObservableProperty] private ObservableCollection<ClanDto> _clans = [];
    [ObservableProperty] private string _searchText = string.Empty;
    [ObservableProperty] private bool _isLoading;
    [ObservableProperty] private bool _showCreatePanel;
    [ObservableProperty] private string _newClanName = string.Empty;
    [ObservableProperty] private string _newClanTag = string.Empty;
    [ObservableProperty] private string _errorMessage = string.Empty;

    public IEnumerable<ClanDto> FilteredClans =>
        string.IsNullOrWhiteSpace(SearchText)
            ? Clans
            : Clans.Where(c => c.Name.Contains(SearchText, StringComparison.OrdinalIgnoreCase)
                             || c.Tag.Contains(SearchText, StringComparison.OrdinalIgnoreCase));

    partial void OnSearchTextChanged(string value) => OnPropertyChanged(nameof(FilteredClans));

    public ClanListViewModel(ApiService api, AppState state)
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
            var list = await _api.GetClansAsync();
            Clans = new ObservableCollection<ClanDto>(list);
            OnPropertyChanged(nameof(FilteredClans));
        }
        catch (Exception ex) { ErrorMessage = ex.Message; }
        finally { IsLoading = false; }
    }

    [RelayCommand]
    private void ToggleCreatePanel()
    {
        ShowCreatePanel = !ShowCreatePanel;
        if (ShowCreatePanel) { NewClanName = string.Empty; NewClanTag = string.Empty; }
    }

    [RelayCommand(CanExecute = nameof(CanCreate))]
    private async Task CreateAsync()
    {
        ErrorMessage = string.Empty;
        try
        {
            var clan = await _api.CreateClanAsync(NewClanName.Trim(), NewClanTag.Trim());
            if (clan is not null)
            {
                Clans.Add(clan);
                OnPropertyChanged(nameof(FilteredClans));
                ShowCreatePanel = false;
                NewClanName = string.Empty;
                NewClanTag = string.Empty;
            }
        }
        catch (Exception ex) { ErrorMessage = ex.Message; }
    }

    private bool CanCreate() => !string.IsNullOrWhiteSpace(NewClanName) && !string.IsNullOrWhiteSpace(NewClanTag);
    partial void OnNewClanNameChanged(string value) => CreateCommand.NotifyCanExecuteChanged();
    partial void OnNewClanTagChanged(string value) => CreateCommand.NotifyCanExecuteChanged();

    [RelayCommand]
    private async Task DeleteAsync(string clanId)
    {
        var confirm = MessageBox.Show("Delete this clan?", "Confirm", MessageBoxButton.YesNo, MessageBoxImage.Warning);
        if (confirm != MessageBoxResult.Yes) return;
        try
        {
            await _api.DeleteClanAsync(clanId);
            var clan = Clans.FirstOrDefault(c => c.Id == clanId);
            if (clan is not null) { Clans.Remove(clan); OnPropertyChanged(nameof(FilteredClans)); }
        }
        catch (Exception ex) { ErrorMessage = ex.Message; }
    }

    [RelayCommand]
    private async Task JoinAsync(string clanId)
    {
        try
        {
            await _api.JoinClanAsync(clanId);
            var clan = Clans.FirstOrDefault(c => c.Id == clanId);
            if (clan is not null) { clan.IsMember = true; OnPropertyChanged(nameof(FilteredClans)); }
        }
        catch (Exception ex) { ErrorMessage = ex.Message; }
    }

    [RelayCommand]
    private async Task LeaveAsync(string clanId)
    {
        try
        {
            await _api.LeaveClanAsync(clanId);
            var clan = Clans.FirstOrDefault(c => c.Id == clanId);
            if (clan is not null) { clan.IsMember = false; OnPropertyChanged(nameof(FilteredClans)); }
        }
        catch (Exception ex) { ErrorMessage = ex.Message; }
    }

    [RelayCommand]
    private void OpenDetail(ClanDto clan)
    {
        MainWindow.Current?.OpenClanDetail(clan);
    }
}
