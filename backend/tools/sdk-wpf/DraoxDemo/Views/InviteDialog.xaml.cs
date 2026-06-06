using DraoxDemo.Services;
using DraoxDemo.ViewModels;
using System.Windows;
using System.Windows.Input;

namespace DraoxDemo.Views;

public partial class InviteDialog : Window
{
    private readonly InviteDialogViewModel _vm;

    public InviteDialog(ApiService api, AppState state, string targetName)
    {
        InitializeComponent();
        _vm = new InviteDialogViewModel(api, state, targetName);
        DataContext = _vm;
    }

    private void UserRow_Click(object sender, MouseButtonEventArgs e)
    {
        if (sender is FrameworkElement fe && fe.DataContext is SelectableUser user)
            _vm.ToggleUser(user);
    }

    private async void Invite_Click(object sender, RoutedEventArgs e)
    {
        await _vm.InviteCommand.ExecuteAsync(null);
    }

    private void Cancel_Click(object sender, RoutedEventArgs e)
    {
        DialogResult = false;
    }
}
