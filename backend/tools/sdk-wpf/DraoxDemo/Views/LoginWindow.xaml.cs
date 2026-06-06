using DraoxDemo.ViewModels;
using Microsoft.Extensions.DependencyInjection;
using System.Windows;

namespace DraoxDemo.Views;

public partial class LoginWindow : Window
{
    public LoginWindow()
    {
        InitializeComponent();
        DataContext = App.Services.GetRequiredService<LoginViewModel>();
    }

    private void PasswordBox_PasswordChanged(object sender, RoutedEventArgs e)
    {
        if (DataContext is LoginViewModel vm)
            vm.Password = PasswordBox.Password;
    }
}
