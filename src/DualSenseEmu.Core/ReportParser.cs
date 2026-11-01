namespace DualSenseEmu.Core;

/// <summary>Parses DualSense input reports (USB 0x01, Bluetooth 0x31). Layout per public Linux hid-playstation docs.</summary>
public static class ReportParser
{
    public const byte UsbReportId = 0x01;
    public const byte BtReportId = 0x31;
    const int UsbMinLength = 11;
    const int BtMinLength = 12;

    public static bool TryParse(ReadOnlySpan<byte> report, out DualSenseState state)
    {
        state = default;
        if (report.IsEmpty) return false;

        int o; // offset of the first axis byte
        switch (report[0])
        {
            case UsbReportId when report.Length >= UsbMinLength: o = 1; break;
            case BtReportId when report.Length >= BtMinLength: o = 2; break; // extra seq/tag byte
            default: return false;
        }

        byte b0 = report[o + 7], b1 = report[o + 8], b2 = report[o + 9];
        var btn = DsButtons.None;
        if ((b0 & 0x10) != 0) btn |= DsButtons.Square;
        if ((b0 & 0x20) != 0) btn |= DsButtons.Cross;
        if ((b0 & 0x40) != 0) btn |= DsButtons.Circle;
        if ((b0 & 0x80) != 0) btn |= DsButtons.Triangle;
        if ((b1 & 0x01) != 0) btn |= DsButtons.L1;
        if ((b1 & 0x02) != 0) btn |= DsButtons.R1;
        if ((b1 & 0x10) != 0) btn |= DsButtons.Create;
        if ((b1 & 0x20) != 0) btn |= DsButtons.Options;
        if ((b1 & 0x40) != 0) btn |= DsButtons.L3;
        if ((b1 & 0x80) != 0) btn |= DsButtons.R3;
        if ((b2 & 0x01) != 0) btn |= DsButtons.Ps;
        if ((b2 & 0x02) != 0) btn |= DsButtons.Touchpad;
        if ((b2 & 0x04) != 0) btn |= DsButtons.Mute;

        // d-pad hat: 0=N,1=NE,2=E,3=SE,4=S,5=SW,6=W,7=NW, 8=released
        int hat = b0 & 0x0F;
        if (hat is 7 or 0 or 1) btn |= DsButtons.DpadUp;
        if (hat is 3 or 4 or 5) btn |= DsButtons.DpadDown;
        if (hat is 5 or 6 or 7) btn |= DsButtons.DpadLeft;
        if (hat is 1 or 2 or 3) btn |= DsButtons.DpadRight;

        var (pct, charge) = report.Length > o + StatusOffset ? ParseBattery(report[o + StatusOffset]) : (-1, ChargeState.Unknown);
        state = new DualSenseState(report[o], report[o + 1], report[o + 2], report[o + 3],
                                   report[o + 4], report[o + 5], btn, pct, charge);
        return true;
    }

    const int StatusOffset = 52; // battery/charge byte, relative to the first axis byte (hid-playstation layout)

    /// <summary>Low nibble = level 0-10, high nibble = state (0 discharging, 1 charging, 2 full, others = error).</summary>
    public static (int Percent, ChargeState Charge) ParseBattery(byte status)
    {
        int level = status & 0x0F;
        return (status >> 4) switch
        {
            0x0 => (Math.Min(level * 10 + 5, 100), ChargeState.Discharging),
            0x1 => (Math.Min(level * 10 + 5, 100), ChargeState.Charging),
            0x2 => (100, ChargeState.Full),
            _ => (-1, ChargeState.Unknown),
        };
    }
}
