using Avalonia;

namespace DualSenseEmu.Gui;

static class Program
{
    [STAThread]
    public static void Main(string[] args)
    {
        // One copy only: a second one would plug in a second virtual pad and fight over the real one.
        // shortcut: the second launch just exits (no "bring the first window forward"); use the tray icon.
        using var single = new Mutex(true, @"Local\DualSenseEmu", out bool first);
        if (!first) return;
        AppBuilder.Configure<App>().UsePlatformDetect().StartWithClassicDesktopLifetime(args);
    }
}
