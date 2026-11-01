namespace DualSenseEmu.Core;

/// <summary>Builds DualSense output reports (rumble motors, lightbar). Layout per public Linux hid-playstation docs.</summary>
public static class OutputReport
{
    public const int UsbLength = 63, BtLength = 78;
    const byte UsbId = 0x02, BtId = 0x31;

    /// <param name="rgb">Lightbar colour, or null to leave it alone.</param>
    public static byte[] Build(bool bluetooth, byte left, byte right, (byte R, byte G, byte B)? rgb, byte btSeq = 0, byte? leds = null)
    {
        var r = new byte[bluetooth ? BtLength : UsbLength];
        int o; // index of valid_flag0
        if (bluetooth) { r[0] = BtId; r[1] = (byte)(btSeq << 4); r[2] = 0x10; o = 3; }
        else { r[0] = UsbId; o = 1; }

        r[o] = 0x01 | 0x02;       // compatible vibration + haptics select
        r[o + 2] = right;         // small motor
        r[o + 3] = left;          // big motor
        if (leds is { } mask)
        {
            r[o + 1] |= 0x10;     // player indicator control enable
            r[o + 43] = (byte)(mask & 0x1F);
        }
        if (rgb is var (red, green, blue))
        {
            r[o + 1] |= 0x04;      // lightbar control enable
            r[o + 38] = 0x02;     // valid_flag2: lightbar setup enable
            r[o + 41] = 0x02;     // lightbar setup: fade out the pairing animation
            r[o + 42] = 0x00;     // brightness: brightest
            (r[o + 44], r[o + 45], r[o + 46]) = (red, green, blue);
        }
        if (bluetooth)
        {
            uint crc = Crc32(r.AsSpan(0, BtLength - 4));
            BitConverter.TryWriteBytes(r.AsSpan(BtLength - 4), crc);
        }
        return r;
    }

    // the five LEDs left to right; player N lights the centre then spreads outwards
    static readonly byte[] PlayerLeds = [0x04, 0x0A, 0x15, 0x1B];

    /// <summary>Bit mask of the five player LEDs to light now; null = leave them alone.</summary>
    public static byte? ResolveLeds(MapProfile p, DualSenseState? ds) => p.PlayerMode switch
    {
        PlayerLedMode.Player when p.PlayerNumber is >= 1 and <= 4 => PlayerLeds[p.PlayerNumber - 1],
        PlayerLedMode.Battery when ds is { BatteryPercent: >= 0 } s => (byte)((1 << Math.Clamp((s.BatteryPercent + 19) / 20, 1, 5)) - 1),
        PlayerLedMode.Charging when ds is { } s => s.Charge is ChargeState.Charging or ChargeState.Full ? (byte)0x1F : (byte)0,
        _ => null,
    };

    static readonly (byte, byte, byte) Green = (0x3D, 0xDC, 0x84), Amber = (0xFF, 0xB1, 0x3D), Red = (0xFF, 0x3B, 0x30);

    /// <summary>The colour the lightbar should show now; null = leave the pad's own.</summary>
    public static (byte, byte, byte)? Resolve(MapProfile p, DualSenseState? ds)
    {
        var fixedColour = ParseColour(p.Lightbar);
        if (ds is not { } s) return fixedColour;
        return p.LightbarMode switch
        {
            LightbarMode.Battery when s.BatteryPercent >= 0 =>
                s.BatteryPercent < 20 ? Red : s.BatteryPercent < 40 ? Amber : Green,
            LightbarMode.Charging => s.Charge switch
            {
                ChargeState.Charging => Amber,
                ChargeState.Full => Green,
                _ => fixedColour,
            },
            _ => fixedColour,
        };
    }

    /// <summary>CRC-32 (zlib) over the 0xA2 seed byte then the report body; no package needed for 10 lines.</summary>
    public static uint Crc32(ReadOnlySpan<byte> body)
    {
        uint c = 0xFFFFFFFF;
        void Feed(byte b) { c ^= b; for (int i = 0; i < 8; i++) c = (c & 1) != 0 ? (c >> 1) ^ 0xEDB88320 : c >> 1; }
        Feed(0xA2);
        foreach (var b in body) Feed(b);
        return ~c;
    }

    /// <summary>"#RRGGBB" or "RRGGBB"; anything else is null (lightbar untouched).</summary>
    public static (byte, byte, byte)? ParseColour(string? hex)
    {
        hex = hex?.TrimStart('#');
        if (hex is not { Length: 6 } || !int.TryParse(hex, System.Globalization.NumberStyles.HexNumber, null, out int v)) return null;
        return ((byte)(v >> 16), (byte)(v >> 8), (byte)v);
    }
}
