using Avalonia;
using Avalonia.Controls;
using Avalonia.Controls.Shapes;
using Avalonia.Interactivity;
using Avalonia.Media;
using Avalonia.Threading;
using DualSenseEmu.Core;
using DualSenseEmu.Device;

namespace DualSenseEmu.Gui;

public partial class MainWindow : Window
{
    static readonly IBrush Green = new SolidColorBrush(Color.Parse("#3DDC84"));
    static readonly IBrush Red = new SolidColorBrush(Color.Parse("#FF5C5C"));
    static readonly IBrush Grey = new SolidColorBrush(Color.Parse("#8A93A6"));
    static readonly IBrush ChipOff = new SolidColorBrush(Color.Parse("#12141A"));
    static readonly IBrush ChipOn = new SolidColorBrush(Color.Parse("#6C8CFF"));
    static readonly IBrush TextOff = new SolidColorBrush(Color.Parse("#8A93A6"));
    static readonly IBrush TextOn = Brushes.White;
    const double PadSize = 120, DotSize = 18;

    readonly Dictionary<string, Border> _chips = new();
    readonly EmulatorService _emu = new();
    readonly DispatcherTimer _timer = new() { Interval = TimeSpan.FromMilliseconds(33) };

    public MainWindow()
    {
        InitializeComponent();
        BuildChips();
        _emu.StatusChanged += s => Dispatcher.UIThread.Post(() =>
        {
            StatusText.Text = s.Message;
            StatusDot.Fill = s.State switch { EmuState.Running => Green, EmuState.Error => Red, _ => Grey };
            StartStop.Content = s.State == EmuState.Running ? "Stop" : "Start";
        });
        _timer.Tick += (_, _) => RefreshLive();
        _timer.Start();
        Closing += (_, _) => { _timer.Stop(); _emu.Dispose(); };
        Opened += async (_, _) => await RefreshHideState();
        _emu.Start(); // emulate straight away; Stop is one click
    }

    void OnStartStop(object? sender, RoutedEventArgs e)
    {
        if (_emu.IsRunning) _emu.Stop(); else _emu.Start();
    }

    void BuildChips()
    {
        var names = Enum.GetValues<DsButtons>().Where(b => b != DsButtons.None).Select(b => b.ToString()).Append("DpadRight");
        foreach (var n in names)
        {
            var label = new TextBlock { Text = n, FontSize = 12, Foreground = TextOff };
            var chip = new Border
            {
                Child = label, Background = ChipOff, CornerRadius = new CornerRadius(8),
                Padding = new Thickness(10, 5), Margin = new Thickness(0, 0, 6, 6),
            };
            _chips[n] = chip;
            Chips.Children.Add(chip);
        }
    }

    static void MoveDot(Ellipse dot, byte x, byte y)
    {
        // 0-255 raw, centre 128 -> dot inside the pad circle
        Canvas.SetLeft(dot, x / 255.0 * (PadSize - DotSize));
        Canvas.SetTop(dot, y / 255.0 * (PadSize - DotSize));
    }

    void RefreshLive()
    {
        var snap = _emu.Latest;
        var ds = snap?.Ds ?? default;
        // idle: sticks centred, nothing pressed
        MoveDot(LeftDot, snap is null ? (byte)128 : ds.Lx, snap is null ? (byte)128 : ds.Ly);
        MoveDot(RightDot, snap is null ? (byte)128 : ds.Rx, snap is null ? (byte)128 : ds.Ry);
        L2.Value = snap is null ? 0 : ds.L2;
        R2.Value = snap is null ? 0 : ds.R2;
        foreach (var (name, chip) in _chips)
        {
            bool on = snap is not null && (name == "DpadRight" ? ds.DpadRight : ds.Buttons.HasFlag(Enum.Parse<DsButtons>(name)));
            chip.Background = on ? ChipOn : ChipOff;
            ((TextBlock)chip.Child!).Foreground = on ? TextOn : TextOff;
        }
    }

    async void OnHideToggle(object? sender, RoutedEventArgs e)
    {
        bool hide = HideToggle.IsChecked == true;
        HideToggle.IsEnabled = false;
        HideText.Text = "Waiting for admin approval (UAC)...";
        // Whitelist the process that reads the pad (DualSenseEmu.exe, or dotnet.exe under `dotnet run`).
        bool ok = await HidHide.SetHiddenAsync(hide, Environment.ProcessPath!);
        await RefreshHideState();
        if (!ok) HideText.Text = "Not changed: approval declined, or no DualSense connected.";
        HideToggle.IsEnabled = true;
    }

    async void OnInstallHidHide(object? sender, RoutedEventArgs e)
    {
        InstallHidHide.IsEnabled = false;
        HideText.Text = "Installing HidHide (approve the prompts)...";
        bool ok = await HidHide.InstallAsync();
        await RefreshHideState();
        if (!ok || !HidHide.IsInstalled) HideText.Text = "HidHide install failed or was cancelled. A restart may be needed after installing.";
        InstallHidHide.IsEnabled = true;
    }

    async Task RefreshHideState()
    {
        bool installed = HidHide.IsInstalled;
        InstallHidHide.IsVisible = !installed;
        HideToggle.IsEnabled = installed;
        if (!installed) { HideText.Text = "HidHide is not installed. It is needed to hide the real pad from games."; return; }
        bool on = await Task.Run(HidHide.IsCloakOn);
        HideToggle.IsChecked = on;
        HideText.Text = on ? "Real pad is hidden from games." : "Real pad is visible to games (double input possible).";
    }
}
