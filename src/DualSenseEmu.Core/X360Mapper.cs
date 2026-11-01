namespace DualSenseEmu.Core;

[Flags]
public enum X360Buttons : ushort
{
    None = 0,
    DpadUp = 0x0001, DpadDown = 0x0002, DpadLeft = 0x0004, DpadRight = 0x0008,
    Start = 0x0010, Back = 0x0020, LeftThumb = 0x0040, RightThumb = 0x0080,
    LeftShoulder = 0x0100, RightShoulder = 0x0200, Guide = 0x0400,
    A = 0x1000, B = 0x2000, X = 0x4000, Y = 0x8000,
}

public readonly record struct X360State(
    X360Buttons Buttons, byte LeftTrigger, byte RightTrigger,
    short LeftX, short LeftY, short RightX, short RightY);

public static class X360Mapper
{
    static readonly (DsButtons From, X360Buttons To)[] Default =
    [
        (DsButtons.Cross, X360Buttons.A), (DsButtons.Circle, X360Buttons.B),
        (DsButtons.Square, X360Buttons.X), (DsButtons.Triangle, X360Buttons.Y),
        (DsButtons.L1, X360Buttons.LeftShoulder), (DsButtons.R1, X360Buttons.RightShoulder),
        (DsButtons.Create, X360Buttons.Back), (DsButtons.Options, X360Buttons.Start),
        (DsButtons.L3, X360Buttons.LeftThumb), (DsButtons.R3, X360Buttons.RightThumb),
        (DsButtons.Ps, X360Buttons.Guide),
        (DsButtons.DpadUp, X360Buttons.DpadUp), (DsButtons.DpadDown, X360Buttons.DpadDown),
        (DsButtons.DpadLeft, X360Buttons.DpadLeft), (DsButtons.DpadRight, X360Buttons.DpadRight),
    ];

    public static X360State Map(DualSenseState s, MapProfile? p = null)
    {
        p ??= new MapProfile();
        var b = X360Buttons.None;
        foreach (var (from, to) in Default)
        {
            if ((s.Buttons & from) == 0) continue;
            b |= p.Remap is not null && p.Remap.TryGetValue(from, out var over) ? over : to;
        }
        if (p.Remap is not null)
            foreach (var (from, to) in p.Remap)
                if ((s.Buttons & from) != 0 && Array.FindIndex(Default, d => d.From == from) < 0) b |= to; // Touchpad, Mute

        var (lx, ly, rx, ry) = p.SwapSticks ? (s.Rx, s.Ry, s.Lx, s.Ly) : (s.Lx, s.Ly, s.Rx, s.Ry);
        var (ldz, rdz) = p.SwapSticks ? (p.RightDeadzone, p.LeftDeadzone) : (p.LeftDeadzone, p.RightDeadzone);
        var (linv, rinv) = p.SwapSticks ? (p.InvertRightY, p.InvertLeftY) : (p.InvertLeftY, p.InvertRightY);
        var (lX, lY) = Stick(lx, ly, ldz, p.Curve, linv);
        var (rX, rY) = Stick(rx, ry, rdz, p.Curve, rinv);
        return new X360State(b, Trigger(s.L2, p.TriggerThreshold), Trigger(s.R2, p.TriggerThreshold), lX, lY, rX, rY);
    }

    /// <summary>Radial deadzone then curve. With deadzone 0 and curve 1 this equals <see cref="Axis"/> exactly.</summary>
    public static (short X, short Y) Stick(byte x, byte y, double deadzone, double curve, bool invertY)
    {
        double vx = Axis(x) / 32768.0, vy = Axis(y, invert: !invertY) / 32768.0; // Xbox Y is up = +
        double mag = Math.Sqrt(vx * vx + vy * vy);
        if (mag <= deadzone || mag == 0) return (0, 0);
        double dz = Math.Clamp(deadzone, 0, 0.9);
        double scaled = Math.Pow((mag - dz) / (1 - dz), Math.Max(curve, 0.1));
        double f = scaled / mag * 32768;
        return ((short)Math.Clamp(vx * f, -32768, 32767), (short)Math.Clamp(vy * f, -32768, 32767));
    }

    public static byte Trigger(byte v, byte threshold)
        => v <= threshold ? (byte)0 : (byte)Math.Min(255, (v - threshold) * 255 / (255 - threshold));

    /// <summary>0-255 (centre 128) to -32768..32767. DualSense Y is up=0, Xbox Y is up=+.</summary>
    public static short Axis(byte v, bool invert = false)
    {
        int x = v == 255 ? 32767 : (v - 128) * 256;
        if (invert) x = x == -32768 ? 32767 : -x;
        return (short)Math.Clamp(x, -32768, 32767);
    }
}
