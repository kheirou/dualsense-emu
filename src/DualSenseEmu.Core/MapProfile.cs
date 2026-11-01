using System.Text.Json;
using System.Text.Json.Serialization;

namespace DualSenseEmu.Core;

/// <summary>User tuning for the DualSense to Xbox translation. Defaults reproduce the plain 1:1 mapping.</summary>
public sealed record MapProfile
{
    /// <summary>Radial deadzone, 0-0.5 of stick travel.</summary>
    public double LeftDeadzone { get; init; }
    public double RightDeadzone { get; init; }
    /// <summary>Response curve exponent: 1 = linear, above 1 = finer control near centre.</summary>
    public double Curve { get; init; } = 1;
    public bool InvertLeftY { get; init; }
    public bool InvertRightY { get; init; }
    public bool SwapSticks { get; init; }
    /// <summary>Triggers read 0 below this (0-254); the rest is rescaled to the full range.</summary>
    public byte TriggerThreshold { get; init; }
    /// <summary>Overrides of the default button map; a DualSense button mapped to None is disabled.
    /// Touchpad and Mute are unmapped by default, so e.g. "Touchpad": "Guide" makes them useful.</summary>
    public Dictionary<DsButtons, X360Buttons>? Remap { get; init; }

    static readonly JsonSerializerOptions Json = new()
    {
        WriteIndented = true,
        Converters = { new JsonStringEnumConverter() },
    };

    public static string DefaultPath => Path.Combine(
        Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData), "DualSenseEmu", "profile.json");

    /// <summary>A missing or unreadable file gives the defaults: a hand-edited typo must not stop the pad working.</summary>
    public static MapProfile Load(string? path = null)
    {
        try { return JsonSerializer.Deserialize<MapProfile>(File.ReadAllText(path ?? DefaultPath), Json) ?? new(); }
        catch { return new(); }
    }

    public void Save(string? path = null)
    {
        path ??= DefaultPath;
        Directory.CreateDirectory(Path.GetDirectoryName(path)!);
        File.WriteAllText(path, JsonSerializer.Serialize(this, Json));
    }
}
