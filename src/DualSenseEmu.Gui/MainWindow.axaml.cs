using System.Diagnostics;
using Avalonia.Controls;
using Avalonia.Interactivity;
using Avalonia.Threading;
using DualSenseEmu.Device;

namespace DualSenseEmu.Gui;

public partial class MainWindow : Window
{
    static readonly string HidHideCli = Path.Combine(
        Environment.GetFolderPath(Environment.SpecialFolder.ProgramFiles),
        @"Nefarius Software Solutions\HidHide\x64\HidHideCLI.exe");

    readonly EmulatorService _emu = new();
    readonly DispatcherTimer _timer = new() { Interval = TimeSpan.FromMilliseconds(33) };

    public MainWindow()
    {
        InitializeComponent();
        _emu.StatusChanged += s => Dispatcher.UIThread.Post(() =>
        {
            StatusText.Text = s.Message;
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

    void RefreshLive()
    {
        if (_emu.Latest is not { } snap)
        {
            ButtonsText.Text = "-";
            foreach (var bar in new[] { L2, R2, LX, LY, RX, RY }) bar.Value = 0;
            return;
        }
        var ds = snap.Ds;
        var pressed = Enum.GetValues<DualSenseEmu.Core.DsButtons>()
            .Where(b => b != DualSenseEmu.Core.DsButtons.None && ds.Buttons.HasFlag(b)).Select(b => b.ToString()).ToList();
        if (ds.DpadRight) pressed.Add("DpadRight");
        ButtonsText.Text = pressed.Count == 0 ? "(none)" : string.Join("  ", pressed);
        L2.Value = ds.L2; R2.Value = ds.R2;
        LX.Value = ds.Lx; LY.Value = ds.Ly; RX.Value = ds.Rx; RY.Value = ds.Ry;
    }

    async void OnHideToggle(object? sender, RoutedEventArgs e)
    {
        bool hide = HideToggle.IsChecked == true;
        HideToggle.IsEnabled = false;
        HideText.Text = "Waiting for admin approval (UAC)...";
        string? script = FindScript();
        if (script is null) HideText.Text = "scripts\\hide-dualsense.ps1 not found.";
        else
        {
            int code = await Task.Run(() => RunScript(script, hide));
            if (code != 0) HideText.Text = "Hide script failed or was cancelled.";
        }
        await RefreshHideState();
        HideToggle.IsEnabled = true;
    }

    // Whitelist the process that actually reads the pad (this exe, or dotnet.exe under `dotnet run`).
    static int RunScript(string script, bool hide)
    {
        var psi = new ProcessStartInfo("pwsh") { CreateNoWindow = true, UseShellExecute = false };
        foreach (var a in new[] { "-NoProfile", "-File", script, "-Dotnet", Environment.ProcessPath! }) psi.ArgumentList.Add(a);
        if (!hide) psi.ArgumentList.Add("-Undo");
        using var p = Process.Start(psi);
        if (p is null) return -1;
        p.WaitForExit();
        return p.ExitCode;
    }

    // shortcut: walks up from the exe to find the repo script; a published exe should ship it beside itself.
    static string? FindScript()
    {
        for (var dir = new DirectoryInfo(AppContext.BaseDirectory); dir is not null; dir = dir.Parent)
        {
            var f = Path.Combine(dir.FullName, "scripts", "hide-dualsense.ps1");
            if (File.Exists(f)) return f;
        }
        return null;
    }

    async Task RefreshHideState()
    {
        if (!File.Exists(HidHideCli)) { HideToggle.IsEnabled = false; HideText.Text = "HidHide not installed (winget install Nefarius.HidHide)."; return; }
        string state = await Task.Run(() =>
        {
            using var p = Process.Start(new ProcessStartInfo(HidHideCli, "--cloak-state") { RedirectStandardOutput = true, CreateNoWindow = true });
            var o = p!.StandardOutput.ReadToEnd(); p.WaitForExit(); return o;
        });
        bool on = state.Contains("--cloak-on");
        HideToggle.IsChecked = on;
        HideText.Text = on ? "Real pad is hidden from games." : "Real pad is visible to games (double input possible).";
    }
}
