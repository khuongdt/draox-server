using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using DraoxDemo.Models;
using DraoxDemo.Services;
using System.Collections.ObjectModel;

namespace DraoxDemo.ViewModels;

public class SelectableUser : ObservableObject
{
    private bool _isSelected;
    public string UserId { get; init; } = string.Empty;
    public string Username { get; init; } = string.Empty;

    public bool IsSelected
    {
        get => _isSelected;
        set => SetProperty(ref _isSelected, value);
    }
}

public partial class InviteDialogViewModel : ObservableObject
{
    private readonly ApiService _api;
    private readonly AppState _state;
    private readonly string _targetName;

    [ObservableProperty] private ObservableCollection<SelectableUser> _users = [];
    [ObservableProperty] private string _filterText = string.Empty;
    [ObservableProperty] private bool _isLoading;
    [ObservableProperty] private string _statusMessage = string.Empty;

    public string TargetName => _targetName;

    public IEnumerable<SelectableUser> FilteredUsers =>
        string.IsNullOrWhiteSpace(FilterText)
            ? Users
            : Users.Where(u => u.Username.Contains(FilterText, StringComparison.OrdinalIgnoreCase));

    public IEnumerable<SelectableUser> SelectedUsers =>
        Users.Where(u => u.IsSelected);

    public int SelectedCount => Users.Count(u => u.IsSelected);

    partial void OnFilterTextChanged(string value) => OnPropertyChanged(nameof(FilteredUsers));

    public InviteDialogViewModel(ApiService api, AppState state, string targetName)
    {
        _api = api;
        _state = state;
        _targetName = targetName;
        _ = LoadUsersFromGeneralAsync();
    }

    private async Task LoadUsersFromGeneralAsync()
    {
        IsLoading = true;
        try
        {
            // Get recent message senders from #general channel as a proxy for active users
            var channels = await _api.GetChannelsAsync();
            var general = channels.FirstOrDefault(c =>
                c.Name.Equals("general", StringComparison.OrdinalIgnoreCase));

            if (general is not null)
            {
                var messages = await _api.GetMessagesAsync(general.Id, 100, null);
                if (messages is not null)
                {
                    var seen = new HashSet<string>();
                    var userList = new List<SelectableUser>();
                    foreach (var msg in messages.Messages)
                    {
                        if (!string.IsNullOrEmpty(msg.SenderId) && seen.Add(msg.SenderId)
                            && msg.SenderId != _state.UserId)
                        {
                            userList.Add(new SelectableUser
                            {
                                UserId = msg.SenderId,
                                Username = msg.SenderName
                            });
                        }
                    }
                    Users = new ObservableCollection<SelectableUser>(userList);
                    OnPropertyChanged(nameof(FilteredUsers));
                }
            }
        }
        catch (Exception ex) { StatusMessage = $"Could not load users: {ex.Message}"; }
        finally { IsLoading = false; }
    }

    [RelayCommand]
    private async Task InviteAsync()
    {
        var selected = SelectedUsers.ToList();
        if (selected.Count == 0) return;

        StatusMessage = "Sending invites...";
        var failed = 0;
        try
        {
            // Invite = send a message to each user mentioning the target
            var channels = await _api.GetChannelsAsync();
            var general = channels.FirstOrDefault(c =>
                c.Name.Equals("general", StringComparison.OrdinalIgnoreCase));

            if (general is not null)
            {
                foreach (var user in selected)
                {
                    try
                    {
                        var mention = $"@{user.Username} You are invited to join {_targetName}!";
                        await _api.SendMessageAsync(general.Id, mention, null);
                    }
                    catch { failed++; }
                }
            }

            StatusMessage = failed == 0
                ? $"Invited {selected.Count} user(s) successfully."
                : $"Done — {selected.Count - failed} sent, {failed} failed.";
        }
        catch (Exception ex) { StatusMessage = ex.Message; }
    }

    public void ToggleUser(SelectableUser user)
    {
        user.IsSelected = !user.IsSelected;
        OnPropertyChanged(nameof(SelectedCount));
    }
}
