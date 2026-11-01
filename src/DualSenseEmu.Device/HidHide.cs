using System.ComponentModel;
using System.Diagnostics;
using System.Text.Json;
using System.Text.RegularExpressions;

namespace DualSenseEmu.Device;

/// <summary>Drives HidHide's CLI: hide the real DualSense from games and whitelist this app so it can still read it.</summary>
public static class HidHide
{
    public static readonly string CliPath = Path.Combine(
        Environment.GetFolderPath(Environment.SpecialFolder.ProgramFiles),
        @"Nefarius Software Solutions\HidHide\x64\HidHideCLI.exe");

    public static bool IsInstalled => File.Exists(CliPath);

    // Sony VID 054C; PID 0CE6 = DualSense, 0DF2 = DualSense Edge (USB and Bluetooth instance paths)
    static readonly Regex SonyPad = new(@"VID[_&](0002)?054C.*PID[_&](0CE6|0DF2)", RegexOptions.IgnoreCase);

    // The paths end up inside an elevated cmd line, so accept only the characters real instance paths use
    // (no quote, space or % that could break out of the quotes).
    static readonly Regex SafePath = new(@"^[A-Za-z0-9_{}&\\\-]+$");

    /// <summary>Pulls DualSense instance paths out of `HidHideCLI --dev-gaming` JSON.</summary>
    public static IReadOnlyList<string> ParseDualSensePaths(string json)
    {
        var found = new List<string>();
        using var doc = JsonDocument.Parse(json);
        foreach (var group in doc.RootElement.EnumerateArray())
        {
            if (!group.TryGetProperty("devices", out var devices)) continue;
            foreach (var d in devices.EnumerateArray())
                if (d.TryGetProperty("deviceInstancePath", out var p) && p.GetString() is { } path && SonyPad.IsMatch(path) && SafePath.IsMatch(path))
                    found.Add(path);
        }
        return found.Distinct().ToList();
    }

    public static bool IsCloakOn() => RunCli("--cloak-state").Contains("--cloak-on");

    public static IReadOnlyList<string> FindDualSense() => ParseDualSensePaths(RunCli("--dev-gaming"));

    /// <summary>One UAC prompt: hide (or unhide) the pad, whitelist/unwhitelist <paramref name="appPath"/>, switch cloaking.</summary>
    public static Task<bool> SetHiddenAsync(bool hide, string appPath) => Task.Run(() =>
    {
        var pads = FindDualSense();
        if (pads.Count == 0) return false;
        // HidHide needs admin; chaining the calls in one elevated cmd means a single prompt.
        // shortcut: only pads connected right now are hidden; switching USB <-> Bluetooth needs a re-toggle.
        return RunElevated("cmd.exe", $"/c \"{BuildChain(hide, pads, appPath)}\"");
    });

    /// <summary>The HidHideCLI calls joined with && so the first failure stops the rest (and the exit code is the failing one).</summary>
    public static string BuildChain(bool hide, IEnumerable<string> pads, string appPath)
    {
        if (appPath.Contains('"') || appPath.Contains('%')) throw new ArgumentException("Unsafe app path", nameof(appPath));
        var cmds = pads.Select(p => $"\"{CliPath}\" {(hide ? "--dev-hide" : "--dev-unhide")} \"{p}\"").ToList();
        cmds.Add($"\"{CliPath}\" {(hide ? "--app-reg" : "--app-unreg")} \"{appPath}\"");
        cmds.Add($"\"{CliPath}\" {(hide ? "--cloak-on" : "--cloak-off")}");
        return string.Join(" && ", cmds);
    }

    public const string HidHidePackage = "Nefarius.HidHide", ViGEmBusPackage = "ViGEm.ViGEmBus";

    /// <summary>Installs a driver package with winget (one UAC prompt). Used for HidHide and ViGEmBus.</summary>
    public static Task<bool> InstallAsync(string wingetId = HidHidePackage) => Task.Run(() =>
        RunElevated("winget", $"install --id {wingetId} -e --accept-package-agreements --accept-source-agreements"));

    static bool RunElevated(string file, string args)
    {
        try
        {
            using var p = Process.Start(new ProcessStartInfo(file, args) { UseShellExecute = true, Verb = "runas", WindowStyle = ProcessWindowStyle.Hidden });
            if (p is null) return false;
            p.WaitForExit();
            return p.ExitCode == 0;
        }
        catch (Win32Exception) { return false; } // user declined the UAC prompt
    }

    static string RunCli(string args)
    {
        using var p = Process.Start(new ProcessStartInfo(CliPath, args) { RedirectStandardOutput = true, CreateNoWindow = true })!;
        var o = p.StandardOutput.ReadToEnd();
        p.WaitForExit();
        return o;
    }
}
