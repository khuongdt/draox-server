using DraoxDemo.Services;
using DraoxDemo.ViewModels;
using DraoxDemo.Views;
using Microsoft.Extensions.DependencyInjection;
using System.Windows;

namespace DraoxDemo;

public partial class App : Application
{
    public static IServiceProvider Services { get; private set; } = null!;

    protected override void OnStartup(StartupEventArgs e)
    {
        base.OnStartup(e);

        var services = new ServiceCollection();
        ConfigureServices(services);
        Services = services.BuildServiceProvider();

        var loginWindow = Services.GetRequiredService<LoginWindow>();
        loginWindow.Show();
    }

    private static void ConfigureServices(IServiceCollection services)
    {
        // Singleton state
        services.AddSingleton<AppState>();

        // Services
        services.AddSingleton<ApiService>();
        services.AddSingleton<SocketService>();

        // ViewModels
        services.AddTransient<LoginViewModel>();
        services.AddTransient<MainViewModel>();
        services.AddTransient<ChannelListViewModel>();
        services.AddTransient<ChatViewModel>();
        services.AddTransient<ClanListViewModel>();
        services.AddTransient<ClanDetailViewModel>();
        services.AddTransient<ServerInfoViewModel>();

        // Windows / Views
        services.AddTransient<LoginWindow>();
        services.AddTransient<MainWindow>();
    }
}
