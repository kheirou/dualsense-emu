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
    static readonly IBrush Green = new SolidColorBrush(Color.Parse("#4CC38A"));
    static readonly IBrush Red = new SolidColorBrush(Color.Parse("#E5484D"));
    static readonly IBrush Grey = new SolidColorBrush(Color.Parse("#7A8296"));
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
            _state = s.State;
            var colour = s.State switch { EmuState.Running => Green, EmuState.Error or EmuState.DriverMissing => Red, EmuState.Waiting => Amber, _ => Grey };
            StatusDot.Fill = colour;
            StatusWord.Foreground = colour;
            StatusWord.Text = s.State switch
            {
                EmuState.Running => "Emulating", EmuState.Waiting => "Waiting", EmuState.Error => "Error",
                EmuState.DriverMissing => "No driver", _ => "Off",
            };
            StatusText.Text = s.State == EmuState.Running ? "games see an Xbox 360 pad" : s.Message;
            InstallVigem.IsVisible = s.State == EmuState.DriverMissing;
            bool active = s.State is EmuState.Running or EmuState.Waiting or EmuState.Error or EmuState.DriverMissing; // it keeps retrying until stopped
            StartStop.Content = active ? "Stop emulation" : "Start emulation";
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


    bool _loadingTuning = true; // sliders fire ValueChanged inside InitializeComponent, before the other controls exist

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
        Pick(LightModePanel, (int)p.LightbarMode);
        Pick(PlayerModePanel, (int)p.PlayerMode);
        Pick(PlayerNumPanel, p.PlayerNumber);
        PlayerNumPanel.IsVisible = p.PlayerMode == PlayerLedMode.Player;
        if (PresetPanel.Children.Count == 0) BuildPresets();
        StaticPanel.IsVisible = p.LightbarMode == LightbarMode.Static;
        ShowColour(p.Lightbar);
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
    static readonly IBrush TileOn = new SolidColorBrush(Color.Parse("#4FB7D6"));
    static readonly IBrush TileOff = new SolidColorBrush(Color.Parse("#262B38"));
    DsButtons _remapSel = DsButtons.Cross;

    X360Buttons? RemapOf(DsButtons b)
        => _emu.Profile.Remap is not null && _emu.Profile.Remap.TryGetValue(b, out var t) ? t : null;

    // tiles show "Cross -> A"; pick one, then choose its target from the always-visible list below
    void BuildRemap(MapProfile p)
    {
        RemapPanel.Children.Clear();
        foreach (var b in Enum.GetValues<DsButtons>().Where(b => b != DsButtons.None))
        {
            var over = RemapOf(b);
            string target = over is null ? (X360Mapper.DefaultFor(b)?.ToString() ?? "-") : over == X360Buttons.None ? "off" : over.ToString()!;
            var text = new StackPanel();
            text.Children.Add(new TextBlock { Text = b.ToString(), FontWeight = FontWeight.SemiBold, HorizontalAlignment = Avalonia.Layout.HorizontalAlignment.Center });
            text.Children.Add(new TextBlock { Text = "→ " + target, FontSize = 12, Foreground = over is null ? Grey : Amber, HorizontalAlignment = Avalonia.Layout.HorizontalAlignment.Center });
            var tile = new Button { Tag = b, Width = 104, Height = 48, Margin = new Thickness(0, 0, 8, 8), Content = text,
                BorderThickness = new Thickness(1), BorderBrush = b == _remapSel ? TileOn : TileOff };
            tile.Click += (_, _) => { _remapSel = b; BuildRemap(_emu.Profile); };
            RemapPanel.Children.Add(tile);
        }
        RemapTitle.Text = $"2. {_remapSel} acts as:";
        RemapTargets.Children.Clear();
        var cur = RemapOf(_remapSel);
        var def = X360Mapper.DefaultFor(_remapSel);
        AddTarget(def is null ? "(unmapped)" : $"(default: {def})", null, cur is null);
        AddTarget("Off", X360Buttons.None, cur == X360Buttons.None);
        foreach (var n in XboxTargets) { var t = Enum.Parse<X360Buttons>(n); AddTarget(n, t, cur == t); }
    }

    void AddTarget(string label, X360Buttons? target, bool selected)
    {
        var r = new RadioButton { Content = label, GroupName = "rt", IsChecked = selected, Margin = new Thickness(0, 0, 16, 4) };
        r.Click += (_, _) =>
        {
            var map = new Dictionary<DsButtons, X360Buttons>(_emu.Profile.Remap ?? []);
            if (target is null) map.Remove(_remapSel); else map[_remapSel] = target.Value;
            _emu.Profile = _emu.Profile with { Remap = map.Count == 0 ? null : map };
            _emu.Profile.Save();
            BuildRemap(_emu.Profile);
        };
        RemapTargets.Children.Add(r);
    }

    void SetLightbar(string? hex, bool fromPicker)
    {
        _emu.Profile = _emu.Profile with { Lightbar = hex };
        _emu.Profile.Save();
        _emu.RefreshOutput();
        _loadingTuning = true; // keep the picker and the text box in step without re-entering
        if (hex is null) { LightbarBox.Text = ""; ShowColour(null); }
        else
        {
            if (fromPicker) LightbarBox.Text = hex;
            else ShowColour(hex);
        }
        _loadingTuning = false;
    }

    static void Pick(Panel panel, int value)
    {
        foreach (var r in panel.Children.OfType<RadioButton>()) r.IsChecked = int.Parse((string)r.Tag!) == value;
    }

    static int Chosen(object? sender) => int.Parse((string)((RadioButton)sender!).Tag!);

    void OnLightMode(object? sender, RoutedEventArgs e)
    {
        var mode = (LightbarMode)Chosen(sender);
        StaticPanel.IsVisible = mode == LightbarMode.Static;
        _emu.Profile = _emu.Profile with { LightbarMode = mode };
        _emu.Profile.Save();
        _emu.RefreshOutput();
    }

    void OnPlayerMode(object? sender, RoutedEventArgs e)
    {
        var mode = (PlayerLedMode)Chosen(sender);
        PlayerNumPanel.IsVisible = mode == PlayerLedMode.Player;
        byte n = _emu.Profile.PlayerNumber == 0 ? (byte)1 : _emu.Profile.PlayerNumber;
        _emu.Profile = _emu.Profile with { PlayerMode = mode, PlayerNumber = n };
        _emu.Profile.Save();
        Pick(PlayerNumPanel, n);
        _emu.RefreshOutput();
    }

    void OnPlayerNum(object? sender, RoutedEventArgs e)
    {
        _emu.Profile = _emu.Profile with { PlayerNumber = (byte)Chosen(sender) };
        _emu.Profile.Save();
        _emu.RefreshOutput();
    }

    static readonly (string Name, string Hex)[] Presets =
    [
        ("Red", "#FF3B30"), ("Orange", "#FF9500"), ("Yellow", "#FFD60A"), ("Green", "#3DDC84"), ("Cyan", "#3FE0FF"),
        ("Blue", "#3B6CFF"), ("Purple", "#9B5CFF"), ("Pink", "#FF5CA8"), ("White", "#FFFFFF"),
    ];

    void BuildPresets()
    {
        foreach (var (name, hex) in Presets)
        {
            var b = new Button { Width = 40, Height = 40, CornerRadius = new CornerRadius(20), Margin = new Thickness(0, 0, 8, 8),
                Background = new SolidColorBrush(Color.Parse(hex)), BorderBrush = TileOff, BorderThickness = new Thickness(1) };
            ToolTip.SetTip(b, name);
            b.Click += (_, _) => SetLightbar(hex, fromPicker: false);
            PresetPanel.Children.Add(b);
        }
    }

    void ShowColour(string? hex)
    {
        var c = OutputReport.ParseColour(hex);
        (RedSlider.Value, GreenSlider.Value, BlueSlider.Value) = c is var (r, g, b) ? (r, g, b) : (0, 0, 0);
        LightSwatch.Background = c is var (r2, g2, b2) ? new SolidColorBrush(Color.FromRgb(r2, g2, b2)) : Brushes.Transparent;
    }

    void OnRgb(object? sender, Avalonia.Controls.Primitives.RangeBaseValueChangedEventArgs e)
    {
        if (_loadingTuning) return;
        var hex = $"#{(int)RedSlider.Value:X2}{(int)GreenSlider.Value:X2}{(int)BlueSlider.Value:X2}";
        LightSwatch.Background = new SolidColorBrush(Color.Parse(hex));
        SetLightbar(hex, fromPicker: true);
    }

    void OnLightbar(object? sender, TextChangedEventArgs e)
    {
        if (_loadingTuning) return;
        var hex = LightbarBox.Text?.Trim();
        if (string.IsNullOrEmpty(hex)) SetLightbar(null, fromPicker: false);
        else if (OutputReport.ParseColour(hex) is not null) SetLightbar(hex, fromPicker: false); // else still typing
    }

    void OnLightbarClear(object? sender, RoutedEventArgs e) => SetLightbar(null, fromPicker: false);

    void OnResetTuning(object? sender, RoutedEventArgs e)
    {
        _emu.Profile = new MapProfile();
        _emu.Profile.Save();
        LoadTuning();
    }

    void OnStartStop(object? sender, RoutedEventArgs e)
    {
        if (_emu.Emulating) _emu.Stop(); else _emu.Start();
    }

    void OnEmulate(object? sender, RoutedEventArgs e) { _emu.Start(); }

    void OnOff(object? sender, RoutedEventArgs e) { _emu.Stop(); }

    static readonly IBrush Amber = new SolidColorBrush(Color.Parse("#E0A23F"));
    static readonly IBrush Blue = new SolidColorBrush(Color.Parse("#4FB7D6"));
    static readonly IBrush Cyan = new SolidColorBrush(Color.Parse("#4FB7D6"));
    static readonly IBrush AmberDim = new SolidColorBrush(Color.Parse("#E0A23F"));
    const double BatteryInnerWidth = 24; // fill width at 100%

    EmuState _state = EmuState.Stopped;
    string? _lastStatus;

    void RefreshDevice(Snapshot? snap)
    {
        var json = StatusFile.ToJson(_state, _emu.Transport, snap?.Ds);
        if (json != _lastStatus) { _lastStatus = json; StatusFile.Write(json); }
        var t = _emu.Transport;
        ConnText.Text = t switch { Transport.Bluetooth => "Bluetooth", Transport.Usb => "USB cable", _ => "Not connected" };
        ConnDot.Fill = t == Transport.None ? Grey : Green;
        TransTop.Text = TransBottom.Text = snap is null || !_emu.Emulating ? "Idle" : "Translating";

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
        ChargeText.Text = ds.Charge switch { ChargeState.Charging => "Charging", ChargeState.Full => "Full", _ => "" };
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
