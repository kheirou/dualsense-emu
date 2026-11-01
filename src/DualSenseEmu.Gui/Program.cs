using Avalonia;
using Velopack;
using Velopack.Sources;

namespace DualSenseEmu.Gui;

static class Program
{
    const string RepoUrl = "https://github.com/kheirou/dualsense-emu";

    [STAThread]
    public static void Main(string[] args)
    {
        // Must run first: Velopack uses it for install / update / uninstall hooks, then returns.
        VelopackApp.Build().Run();

        // One copy only: a second one would plug in a second virtual pad and fight over the real one.
        // shortcut: the second launch just exits (no "bring the first window forward"); use the tray icon.
        using var single = new Mutex(true, @"Local\DualSenseEmu", out bool first);
        if (!first) return;

        _ = UpdateInBackground();
        AppBuilder.Configure<App>().UsePlatformDetect().StartWithClassicDesktopLifetime(args);
    }

    /// <summary>Downloads a newer release, applied when the app exits, so play is never interrupted.</summary>
    static async Task UpdateInBackground()
    {
        try
        {
            var mgr = new UpdateManager(new GithubSource(RepoUrl, accessToken: null, prerelease: false));
            if (!mgr.IsInstalled) return; // dev run, not installed via Setup.exe
            var update = await mgr.CheckForUpdatesAsync();
            if (update is null) return;
            await mgr.DownloadUpdatesAsync(update);
            mgr.WaitExitThenApplyUpdates(update.TargetFullRelease);
        }
        catch
        {
            // shortcut: update failures are silent (offline, rate limit, or the repo is private); the app works without them.
        }
    }
}
