using Avalonia;
using Avalonia.Controls;
using Avalonia.Controls.Shapes;
using Avalonia.Interactivity;
using Avalonia.Media;
using Avalonia.Threading;
using DualSenseEmu.Core;
using DualSenseEmu.Device;
using Microsoft.Win32;

namespace DualSenseEmu.Gui;

public partial class MainWindow : Window
{
    static readonly IBrush Green = new SolidColorBrush(Color.Parse("#3DDC84"));
    static readonly IBrush Red = new SolidColorBrush(Color.Parse("#FF5C5C"));
    static readonly IBrush Grey = new SolidColorBrush(Color.Parse("#8A93A6"));
    readonly PadMap _dsPad, _xboxPad;
    readonly EmulatorService _emu = new();
    readonly DispatcherTimer _timer = new() { Interval = TimeSpan.FromMilliseconds(33) };

    public MainWindow()
    {
        InitializeComponent();
        _dsPad = new PadMap(DsCanvas, xbox: false);
        _xboxPad = new PadMap(XboxCanvas, xbox: true);
        _emu.StatusChanged += s => Dispatcher.UIThread.Post(() =>
        {
            var colour = s.State switch { EmuState.Running => Green, EmuState.Error or EmuState.DriverMissing => Red, EmuState.Waiting => Amber, _ => Grey };
            StatusDot.Fill = colour;
            StatusWord.Foreground = colour;
            StatusWord.Text = s.State switch
            {
                EmuState.Running => "EMULATING", EmuState.Waiting => "WAITING", EmuState.Error => "ERROR",
                EmuState.DriverMissing => "NO DRIVER", _ => "OFF",
            };
            StatusText.Text = s.State == EmuState.Running ? "games see an Xbox 360 pad" : s.Message;
            InstallVigem.IsVisible = s.State == EmuState.DriverMissing;
            bool active = s.State is EmuState.Running or EmuState.Waiting or EmuState.Error or EmuState.DriverMissing; // it keeps retrying until stopped
            StartStop.Content = active ? "STOP EMULATION" : "START EMULATION";
            SegEmulate.Classes.Set("on", active);
            SegOff.Classes.Set("amber", !active);
            KnobArc.Stroke = active ? Cyan : AmberDim;
            KnobTick.Background = active ? Cyan : AmberDim;
        });
        _timer.Tick += (_, _) => RefreshLive();
        Closing += (_, e) =>
        {
            // only the X button hides to the tray; Windows shutdown / sign-out must be allowed to close us
            if (_exiting || e.CloseReason != WindowCloseReason.WindowClosing) return;
            e.Cancel = true; // keep emulating in the tray
            _timer.Stop(); // nobody sees the live view in the tray
            Hide();
        };
        Opened += async (_, _) => { _timer.Start(); await RefreshHideState(); };
        LoadTuning();
        RefreshStartup();
        _emu.Start(); // emulate straight away; Stop is one click
    }

    const string RunKey = @"Software\Microsoft\Windows\CurrentVersion\Run";
    const string RunValue = "DualSenseEmu";
    bool _exiting;

    // the title bar is drawn over by our content, so any empty spot moves the window (buttons handle their own clicks)
    void OnDragWindow(object? sender, Avalonia.Input.PointerPressedEventArgs e)
    {
        if (e.GetCurrentPoint(this).Properties.IsLeftButtonPressed) BeginMoveDrag(e);
    }

    public void ShowFromTray() { _timer.Start(); Show(); WindowState = WindowState.Normal; Activate(); }

    public void ExitApp() { _exiting = true; _timer.Stop(); _emu.Dispose(); }

    // Under `dotnet run` the process is dotnet.exe: registering that for startup would be wrong.
    static bool IsPublishedExe =>
        !string.Equals(System.IO.Path.GetFileNameWithoutExtension(Environment.ProcessPath), "dotnet", StringComparison.OrdinalIgnoreCase);

    void RefreshStartup()
    {
        StartupToggle.IsEnabled = IsPublishedExe;
        using var key = Registry.CurrentUser.OpenSubKey(RunKey);
        StartupToggle.IsChecked = key?.GetValue(RunValue) is string;
        if (!IsPublishedExe) ToolTip.SetTip(StartupToggle, "Run the published DualSenseEmu.exe to enable this.");
    }

    // shortcut: registers the exe where it runs now. Fine for the installed app (stable current\ folder); a standalone
    // dist\ exe breaks start-at-login if rebuilt or moved.
    void OnStartupToggle(object? sender, RoutedEventArgs e)
    {
        using var key = Registry.CurrentUser.OpenSubKey(RunKey, writable: true)!;
        if (StartupToggle.IsChecked == true) key.SetValue(RunValue, $"\"{Environment.ProcessPath}\" --tray");
        else key.DeleteValue(RunValue, throwOnMissingValue: false);
    }


    bool _loadingTuning;

    void LoadTuning()
    {
        var p = _emu.Profile;
        _loadingTuning = true;
        DeadzoneSlider.Value = p.LeftDeadzone;
        CurveSlider.Value = p.Curve;
        TriggerSlider.Value = p.TriggerThreshold;
        InvertYBox.IsChecked = p.InvertLeftY && p.InvertRightY;
        SwapBox.IsChecked = p.SwapSticks;
        LightbarBox.Text = p.Lightbar ?? "";
        ShowSwatch(p.Lightbar);
        BuildRemap(p);
        _loadingTuning = false;
    }

    // the sliders edit the shared tuning only; Remap (and anything else in the file) is kept as is
    void OnCheck(object? sender, RoutedEventArgs e) => OnTune(sender, e);

    void OnSlide(object? sender, Avalonia.Controls.Primitives.RangeBaseValueChangedEventArgs e) => OnTune(sender, e);

    void OnTune(object? sender, EventArgs e)
    {
        if (_loadingTuning) return;
        bool inv = InvertYBox.IsChecked == true;
        _emu.Profile = _emu.Profile with
        {
            LeftDeadzone = DeadzoneSlider.Value, RightDeadzone = DeadzoneSlider.Value,
            Curve = CurveSlider.Value, TriggerThreshold = (byte)TriggerSlider.Value,
            InvertLeftY = inv, InvertRightY = inv, SwapSticks = SwapBox.IsChecked == true,
        };
        _emu.Profile.Save();
    }


    static readonly string[] XboxTargets = Enum.GetNames<X360Buttons>().Where(n => n != "None").ToArray();

    // one row per DualSense button: (default) / None / any Xbox button. Choosing (default) drops the override.
    void BuildRemap(MapProfile p)
    {
        RemapPanel.Children.Clear();
        foreach (var b in Enum.GetValues<DsButtons>().Where(b => b != DsButtons.None))
        {
            var def = X360Mapper.DefaultFor(b);
            var combo = new ComboBox { Width = 150, Tag = b, ItemsSource = new[] { def is null ? "(unmapped)" : $"(default: {def})", "None" }.Concat(XboxTargets).ToArray() };
            combo.SelectedIndex = p.Remap is not null && p.Remap.TryGetValue(b, out var t)
                ? (t == X360Buttons.None ? 1 : 2 + Array.IndexOf(XboxTargets, t.ToString())) : 0;
            combo.SelectionChanged += OnRemap;
            var row = new StackPanel { Orientation = Avalonia.Layout.Orientation.Horizontal, Spacing = 8 };
            row.Children.Add(new TextBlock { Text = b.ToString(), Width = 80, VerticalAlignment = Avalonia.Layout.VerticalAlignment.Center });
            row.Children.Add(combo);
            RemapPanel.Children.Add(row);
        }
    }

    void OnRemap(object? sender, SelectionChangedEventArgs e)
    {
        if (_loadingTuning || sender is not ComboBox { Tag: DsButtons b } c) return;
        var map = new Dictionary<DsButtons, X360Buttons>(_emu.Profile.Remap ?? []);
        if (c.SelectedIndex <= 0) map.Remove(b);
        else map[b] = c.SelectedIndex == 1 ? X360Buttons.None : Enum.Parse<X360Buttons>(XboxTargets[c.SelectedIndex - 2]);
        _emu.Profile = _emu.Profile with { Remap = map.Count == 0 ? null : map };
        _emu.Profile.Save();
    }

    void ShowSwatch(string? hex)
        => LightSwatch.Background = OutputReport.ParseColour(hex) is var (r, g, b) ? new SolidColorBrush(Color.FromRgb(r, g, b)) : Brushes.Transparent;

    void OnLightbar(object? sender, TextChangedEventArgs e)
    {
        if (_loadingTuning) return;
        var hex = LightbarBox.Text?.Trim();
        ShowSwatch(hex);
        if (!string.IsNullOrEmpty(hex) && OutputReport.ParseColour(hex) is null) return; // still typing
        _emu.Profile = _emu.Profile with { Lightbar = string.IsNullOrEmpty(hex) ? null : hex };
        _emu.Profile.Save();
        _emu.RefreshOutput();
    }

    void OnResetTuning(object? sender, RoutedEventArgs e)
    {
        _emu.Profile = new MapProfile();
        _emu.Profile.Save();
        LoadTuning();
    }

    void OnStartStop(object? sender, RoutedEventArgs e)
    {
        if (_emu.IsRunning) _emu.Stop(); else _emu.Start();
    }

    void OnEmulate(object? sender, RoutedEventArgs e) { if (!_emu.IsRunning) _emu.Start(); }

    void OnOff(object? sender, RoutedEventArgs e) { if (_emu.IsRunning) _emu.Stop(); }

    static readonly IBrush Amber = new SolidColorBrush(Color.Parse("#FFC83D"));
    static readonly IBrush Blue = new SolidColorBrush(Color.Parse("#6C8CFF"));
    static readonly IBrush Cyan = new SolidColorBrush(Color.Parse("#3FE0FF"));
    static readonly IBrush AmberDim = new SolidColorBrush(Color.Parse("#E0A23F"));
    const double BatteryInnerWidth = 24; // fill width at 100%

    void RefreshDevice(Snapshot? snap)
    {
        var t = _emu.Transport;
        ConnText.Text = t switch { Transport.Bluetooth => "Bluetooth", Transport.Usb => "USB cable", _ => "Not connected" };
        ConnDot.Fill = t == Transport.None ? Grey : Green;
        TransTop.Text = TransBottom.Text = snap is null ? "IDLE" : "TRANSLATING...";

        if (snap is null || snap.Ds.BatteryPercent < 0)
        {
            BatteryText.Text = "--";
            BatteryFill.Width = 0;
            ChargeText.Text = "";
            return;
        }
        var ds = snap.Ds;
        BatteryFill.Width = BatteryInnerWidth * ds.BatteryPercent / 100.0;
        BatteryFill.Background = ds.Charge == ChargeState.Charging ? Blue
            : ds.BatteryPercent < 20 ? Red : ds.BatteryPercent < 40 ? Amber : Green;
        BatteryText.Text = $"{ds.BatteryPercent}%";
        ChargeText.Text = ds.Charge switch { ChargeState.Charging => "⚡Charging", ChargeState.Full => "Full", _ => "" };
    }

    void RefreshLive()
    {
        var snap = _emu.Latest;
        RefreshDevice(snap);
        if (snap is null)
        {
            // idle: sticks centred, nothing pressed
            _dsPad.Update(_ => false, 0, 0, 128, 128, 128, 128);
            _xboxPad.Update(_ => false, 0, 0, 128, 128, 128, 128);
            return;
        }
        var ds = snap.Ds;
        var x = snap.X360;
        _dsPad.Update(k => Enum.TryParse<DsButtons>(k, out var b) && ds.Buttons.HasFlag(b),
            ds.L2, ds.R2, ds.Lx, ds.Ly, ds.Rx, ds.Ry);
        _xboxPad.Update(k => Enum.TryParse<X360Buttons>(k, out var b) && x.Buttons.HasFlag(b),
            x.LeftTrigger, x.RightTrigger, ds.Lx, ds.Ly, ds.Rx, ds.Ry);
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

    // the emulator keeps retrying, so once the driver is in, it picks it up within 2 s
    async void OnInstallVigem(object? sender, RoutedEventArgs e)
    {
        InstallVigem.IsEnabled = false;
        StatusText.Text = "Installing ViGEmBus (approve the prompt)...";
        if (!await HidHide.InstallAsync(HidHide.ViGEmBusPackage)) StatusText.Text = "ViGEmBus install failed or was cancelled.";
        InstallVigem.IsEnabled = true;
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
        if (!IsPublishedExe) { HideToggle.IsEnabled = false; HideText.Text = "Run the published DualSenseEmu.exe to hide the pad (dotnet.exe would be whitelisted for every .NET app)."; return; }
        bool on = await Task.Run(HidHide.IsCloakOn);
        HideToggle.IsChecked = on;
        HideText.Text = on ? "Real pad is hidden from games." : "Real pad is visible to games (double input possible).";
    }
}
