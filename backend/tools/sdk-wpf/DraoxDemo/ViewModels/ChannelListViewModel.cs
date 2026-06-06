using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using DraoxDemo.Models;
using DraoxDemo.Services;
using DraoxDemo.Views;
using System.Collections.ObjectModel;
using System.Windows;

namespace DraoxDemo.ViewModels;

public partial class ChannelListViewModel : ObservableObject
{
    private readonly ApiService _api;

    [ObservableProperty] private ObservableCollection<ChannelDto> _channels = [];
    [ObservableProperty] private string _searchText = string.Empty;
    [ObservableProperty] private bool _isLoading;
    [ObservableProperty] private bool _showCreatePanel;
    [ObservableProperty] private string _newChannelName = string.Empty;
    [ObservableProperty] private string _newChannelDesc = string.Empty;
    [ObservableProperty] private string _errorMessage = string.Empty;

    public IEnumerable<ChannelDto> FilteredChannels =>
        string.IsNullOrWhiteSpace(SearchText)
            ? Channels
            : Channels.Where(c => c.Name.Contains(SearchText, StringComparison.OrdinalIgnoreCase));

    partial void OnSearchTextChanged(string value) => OnPropertyChanged(nameof(FilteredChannels));

    public ChannelListViewModel(ApiService api)
    {
        _api = api;
        _ = LoadAsync();
    }

    [RelayCommand]
    private async Task LoadAsync()
    {
        IsLoading = true;
        ErrorMessage = string.Empty;
        try
        {
            var list = await _api.GetChannelsAsync();
            Channels = new ObservableCollection<ChannelDto>(list);
            OnPropertyChanged(nameof(FilteredChannels));
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

    [RelayCommand]
    private void ToggleCreatePanel()
    {
        ShowCreatePanel = !ShowCreatePanel;
        if (ShowCreatePanel) { NewChannelName = string.Empty; NewChannelDesc = string.Empty; }
    }

    [RelayCommand(CanExecute = nameof(CanCreate))]
    private async Task CreateAsync()
    {
        ErrorMessage = string.Empty;
        try
        {
            var ch = await _api.CreateChannelAsync(NewChannelName.Trim(), NewChannelDesc.Trim());
            if (ch is not null)
            {
                Channels.Add(ch);
                OnPropertyChanged(nameof(FilteredChannels));
                ShowCreatePanel = false;
                NewChannelName = string.Empty;
                NewChannelDesc = string.Empty;
            }
        }
        catch (Exception ex) { ErrorMessage = ex.Message; }
    }

    private bool CanCreate() => !string.IsNullOrWhiteSpace(NewChannelName);

    partial void OnNewChannelNameChanged(string value) => CreateCommand.NotifyCanExecuteChanged();

    [RelayCommand]
    private async Task DeleteAsync(string channelId)
    {
        var confirm = MessageBox.Show("Delete this channel?", "Confirm", MessageBoxButton.YesNo, MessageBoxImage.Warning);
        if (confirm != MessageBoxResult.Yes) return;

        try
        {
            await _api.DeleteChannelAsync(channelId);
            var ch = Channels.FirstOrDefault(c => c.Id == channelId);
            if (ch is not null) { Channels.Remove(ch); OnPropertyChanged(nameof(FilteredChannels)); }
        }
        catch (Exception ex) { ErrorMessage = ex.Message; }
    }

    [RelayCommand]
    private async Task JoinAsync(string channelId)
    {
        try
        {
            await _api.SubscribeChannelAsync(channelId);
            var ch = Channels.FirstOrDefault(c => c.Id == channelId);
            if (ch is not null) { ch.IsSubscribed = true; OnPropertyChanged(nameof(FilteredChannels)); }
        }
        catch (Exception ex) { ErrorMessage = ex.Message; }
    }

    [RelayCommand]
    private async Task LeaveAsync(string channelId)
    {
        try
        {
            await _api.UnsubscribeChannelAsync(channelId);
            var ch = Channels.FirstOrDefault(c => c.Id == channelId);
            if (ch is not null) { ch.IsSubscribed = false; OnPropertyChanged(nameof(FilteredChannels)); }
        }
        catch (Exception ex) { ErrorMessage = ex.Message; }
    }

    [RelayCommand]
    private void OpenChat(ChannelDto channel)
    {
        MainWindow.Current?.OpenChat(channel);
    }
}
