using Avalonia;
using Avalonia.Controls;
using Avalonia.Controls.ApplicationLifetimes;
using Avalonia.Markup.Xaml;
using Avalonia.Platform;

namespace DualSenseEmu.Gui;

public class App : Application
{
    public override void Initialize() => AvaloniaXamlLoader.Load(this);

    public override void OnFrameworkInitializationCompleted()
    {
        if (ApplicationLifetime is IClassicDesktopStyleApplicationLifetime desktop)
        {
            // closing the window only hides it to the tray; the Exit menu item really quits
            desktop.ShutdownMode = ShutdownMode.OnExplicitShutdown;
            var window = new MainWindow();
            if (desktop.Args?.Contains("--tray") != true) window.Show();

            var show = new NativeMenuItem("Show window");
            show.Click += (_, _) => window.ShowFromTray();
            var exit = new NativeMenuItem("Exit");
            exit.Click += (_, _) => { window.ExitApp(); desktop.Shutdown(); };
            var tray = new TrayIcon
            {
                Icon = new WindowIcon(AssetLoader.Open(new Uri("avares://DualSenseEmu/Assets/icon.png"))),
                ToolTipText = "DualSense Emulator",
                Menu = new NativeMenu { show, exit },
            };
            tray.Clicked += (_, _) => window.ShowFromTray();
            TrayIcon.SetIcons(this, new TrayIcons { tray });
        }
        base.OnFrameworkInitializationCompleted();
    }
}
