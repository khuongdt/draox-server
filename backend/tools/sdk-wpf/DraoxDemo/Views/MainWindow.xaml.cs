using DraoxDemo.ViewModels;
using Microsoft.Extensions.DependencyInjection;
using System.Windows;

namespace DraoxDemo.Views;

public partial class MainWindow : Window
{
    public static MainViewModel? Current { get; private set; }

    public MainWindow()
    {
        InitializeComponent();
        var vm = App.Services.GetRequiredService<MainViewModel>();
        DataContext = vm;
        Current = vm;
        vm.Initialize();
    }
}
