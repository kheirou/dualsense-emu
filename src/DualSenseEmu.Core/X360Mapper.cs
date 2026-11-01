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
    public static X360State Map(DualSenseState s)
    {
        var b = X360Buttons.None;
        void Set(DsButtons from, X360Buttons to) { if ((s.Buttons & from) != 0) b |= to; }
        Set(DsButtons.Cross, X360Buttons.A);
        Set(DsButtons.Circle, X360Buttons.B);
        Set(DsButtons.Square, X360Buttons.X);
        Set(DsButtons.Triangle, X360Buttons.Y);
        Set(DsButtons.L1, X360Buttons.LeftShoulder);
        Set(DsButtons.R1, X360Buttons.RightShoulder);
        Set(DsButtons.Create, X360Buttons.Back);
        Set(DsButtons.Options, X360Buttons.Start);
        Set(DsButtons.L3, X360Buttons.LeftThumb);
        Set(DsButtons.R3, X360Buttons.RightThumb);
        Set(DsButtons.Ps, X360Buttons.Guide);
        Set(DsButtons.DpadUp, X360Buttons.DpadUp);
        Set(DsButtons.DpadDown, X360Buttons.DpadDown);
        Set(DsButtons.DpadLeft, X360Buttons.DpadLeft);
        if (s.DpadRight) b |= X360Buttons.DpadRight;

        return new X360State(b, s.L2, s.R2,
            Axis(s.Lx), Axis(s.Ly, invert: true), Axis(s.Rx), Axis(s.Ry, invert: true));
    }

    /// <summary>0-255 (centre 128) to -32768..32767. DualSense Y is up=0, Xbox Y is up=+.</summary>
    public static short Axis(byte v, bool invert = false)
    {
        int x = v == 255 ? 32767 : (v - 128) * 256;
        if (invert) x = x == -32768 ? 32767 : -x;
        return (short)Math.Clamp(x, -32768, 32767);
    }
}
