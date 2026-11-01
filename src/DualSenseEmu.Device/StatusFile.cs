using System.Text.Json;
using DualSenseEmu.Core;

namespace DualSenseEmu.Device;

/// <summary>Current pad status as JSON in %APPDATA%\DualSenseEmu\status.json, for status bars (YASB) to read.</summary>
public static class StatusFile
{
    public static string DefaultPath => Path.Combine(
        Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData), "DualSenseEmu", "status.json");

    public static string ToJson(EmuState state, Transport transport, DualSenseState? ds)
        => JsonSerializer.Serialize(new
        {
            state = state.ToString().ToLowerInvariant(),
            transport = transport.ToString().ToLowerInvariant(),
            battery = ds is { BatteryPercent: >= 0 } ? ds.Value.BatteryPercent : (int?)null,
            charge = ds?.Charge.ToString().ToLowerInvariant(),
        });

    /// <summary>Write via temp file + replace so a reader never sees half a file.</summary>
    public static void Write(string json, string? path = null)
    {
        path ??= DefaultPath;
        try
        {
            Directory.CreateDirectory(Path.GetDirectoryName(path)!);
            var tmp = path + ".tmp";
            File.WriteAllText(tmp, json);
            File.Move(tmp, path, overwrite: true);
        }
        catch (IOException) { /* a reader has it open; the next change rewrites it */ }
    }
}
