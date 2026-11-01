using DualSenseEmu.Core;

namespace DualSenseEmu.Tests;

public class MappingTests
{
    static byte[] Usb(byte b0 = 0x08, byte b1 = 0, byte b2 = 0, byte lx = 128, byte ly = 128, byte l2 = 0)
    {
        var r = new byte[64];
        r[0] = ReportParser.UsbReportId;
        r[1] = lx; r[2] = ly; r[3] = 128; r[4] = 128; r[5] = l2; r[6] = 0;
        r[8] = b0; r[9] = b1; r[10] = b2; // hat 8 = released
        return r;
    }

    static DualSenseState Parse(byte[] r) { Assert.True(ReportParser.TryParse(r, out var s)); return s; }

    [Fact]
    public void Rejects_short_unknown_and_empty()
    {
        Assert.False(ReportParser.TryParse([], out _));
        Assert.False(ReportParser.TryParse(new byte[5] { 1, 0, 0, 0, 0 }, out _));
        Assert.False(ReportParser.TryParse(new byte[64], out _)); // id 0
    }

    [Fact]
    public void Usb_face_buttons_map_to_xbox()
    {
        var x = X360Mapper.Map(Parse(Usb(b0: 0x08 | 0x20 | 0x40))); // cross + circle
        Assert.Equal(X360Buttons.A | X360Buttons.B, x.Buttons);
    }

    [Fact]
    public void Bluetooth_report_is_offset_by_one()
    {
        var r = new byte[78];
        r[0] = ReportParser.BtReportId;
        r[2] = 255; r[3] = 0; r[4] = 128; r[5] = 128; r[6] = 200;
        r[9] = 0x08; r[10] = 0x01; // released hat, L1
        var x = X360Mapper.Map(Parse(r));
        Assert.Equal(32767, x.LeftX);
        Assert.Equal(200, x.LeftTrigger);
        Assert.Equal(X360Buttons.LeftShoulder, x.Buttons);
    }

    [Theory]
    [InlineData(0, X360Buttons.DpadUp)]
    [InlineData(1, X360Buttons.DpadUp | X360Buttons.DpadRight)]
    [InlineData(2, X360Buttons.DpadRight)]
    [InlineData(4, X360Buttons.DpadDown)]
    [InlineData(6, X360Buttons.DpadLeft)]
    [InlineData(7, X360Buttons.DpadUp | X360Buttons.DpadLeft)]
    [InlineData(8, X360Buttons.None)]
    public void Dpad_hat(byte hat, X360Buttons expected)
        => Assert.Equal(expected, X360Mapper.Map(Parse(Usb(b0: hat))).Buttons);

    [Fact]
    public void Axis_scaling_and_y_inversion()
    {
        Assert.Equal(-32768, X360Mapper.Axis(0));
        Assert.Equal(0, X360Mapper.Axis(128));
        Assert.Equal(32767, X360Mapper.Axis(255));
        // stick up (Y=0) must become Xbox up (+)
        Assert.Equal(32767, X360Mapper.Map(Parse(Usb(ly: 0))).LeftY);
        Assert.Equal(-32767, X360Mapper.Map(Parse(Usb(ly: 255))).LeftY);
    }

    [Fact]
    public void Triggers_and_system_buttons()
    {
        var x = X360Mapper.Map(Parse(Usb(b1: 0x30, b2: 0x01, l2: 77)));
        Assert.Equal(77, x.LeftTrigger);
        Assert.Equal(X360Buttons.Back | X360Buttons.Start | X360Buttons.Guide, x.Buttons);
    }

    [Theory]
    [InlineData(0x08, 85, ChargeState.Discharging)]
    [InlineData(0x0A, 100, ChargeState.Discharging)] // level 10 clamps to 100
    [InlineData(0x00, 5, ChargeState.Discharging)]
    [InlineData(0x13, 35, ChargeState.Charging)]
    [InlineData(0x2A, 100, ChargeState.Full)]
    [InlineData(0xA5, -1, ChargeState.Unknown)] // error nibble
    public void Battery_status_byte(byte status, int pct, ChargeState charge)
        => Assert.Equal((pct, charge), ReportParser.ParseBattery(status));

    [Fact]
    public void Battery_read_from_usb_and_bluetooth_reports()
    {
        var usb = Usb();
        usb[1 + 52] = 0x13; // charging, 35%
        var s = Parse(usb);
        Assert.Equal(35, s.BatteryPercent);
        Assert.Equal(ChargeState.Charging, s.Charge);

        var bt = new byte[78];
        bt[0] = ReportParser.BtReportId; bt[9] = 0x08;
        bt[2 + 52] = 0x06; // discharging, 65%
        Assert.Equal(65, Parse(bt).BatteryPercent);
    }

    // A real Bluetooth report captured from a DualSense lying idle on charge (2026-11-01). Unlike the made-up
    // arrays above, this fails if any offset is wrong. shortcut: no wired (USB) capture yet.
    const string RealBtIdle =
        "31C1807F827D00000108000000F442AD530000F9FF010086FE4A1FD8015A6CDA891480000000800000000009090000000000CE70DA89101000FCCE93C926978943000000000000000000B4CAC2EC";

    [Fact]
    public void Real_bluetooth_report_parses_to_an_idle_charging_pad()
    {
        var s = Parse(Convert.FromHexString(RealBtIdle));
        Assert.Equal(DsButtons.None, s.Buttons);                         // hat 8 = d-pad released, nothing pressed
        Assert.InRange(s.Lx, 120, 136); Assert.InRange(s.Ly, 120, 136);  // resting sticks sit near the centre (128)
        Assert.InRange(s.Rx, 120, 136); Assert.InRange(s.Ry, 120, 136);
        Assert.Equal(0, s.L2); Assert.Equal(0, s.R2);
        Assert.Equal(5, s.BatteryPercent);                               // status byte 0x10 = level 0, charging
        Assert.Equal(ChargeState.Charging, s.Charge);
    }

    [Fact]
    public void Short_report_has_unknown_battery()
    {
        var r = new byte[12]; r[0] = ReportParser.UsbReportId; r[8] = 0x08;
        Assert.Equal(-1, Parse(r).BatteryPercent);
    }

    [Fact]
    public void Default_profile_changes_nothing()
    {
        var s = Parse(Usb(b0: 0x28, lx: 255, ly: 0, l2: 77));
        Assert.Equal(X360Mapper.Map(s), X360Mapper.Map(s, new MapProfile()));
    }

    [Fact]
    public void Deadzone_zeroes_small_deflection_and_curve_softens_the_rest()
    {
        Assert.Equal(0, X360Mapper.Map(Parse(Usb(lx: 140)), new MapProfile { LeftDeadzone = 0.2 }).LeftX);
        Assert.True(Math.Abs(X360Mapper.Map(Parse(Usb(lx: 200)), new MapProfile { Curve = 2 }).LeftX)
                  < Math.Abs(X360Mapper.Map(Parse(Usb(lx: 200))).LeftX));
    }

    [Fact]
    public void Swap_and_invert_sticks()
    {
        var s = Parse(Usb(lx: 255, ly: 0));
        Assert.Equal(32767, X360Mapper.Map(s, new MapProfile { SwapSticks = true }).RightX);
        Assert.Equal(-32768, X360Mapper.Map(s, new MapProfile { InvertLeftY = true }).LeftY);
    }

    [Fact]
    public void Trigger_threshold()
    {
        Assert.Equal(0, X360Mapper.Trigger(40, 50));
        Assert.Equal(255, X360Mapper.Trigger(255, 50));
        Assert.Equal(0, X360Mapper.Trigger(255, 255));
    }

    [Fact]
    public void Remap_swaps_disables_and_maps_touchpad()
    {
        var p = new MapProfile { Remap = new() {
            [DsButtons.Cross] = X360Buttons.B, [DsButtons.Circle] = X360Buttons.None, [DsButtons.Touchpad] = X360Buttons.Guide } };
        var x = X360Mapper.Map(Parse(Usb(b0: 0x08 | 0x20 | 0x40, b2: 0x02)), p); // cross + circle + touchpad
        Assert.Equal(X360Buttons.B | X360Buttons.Guide, x.Buttons);
    }

    [Fact]
    public void Profile_roundtrips_and_bad_file_gives_defaults()
    {
        var f = Path.GetTempFileName();
        try
        {
            new MapProfile { Curve = 2, Remap = new() { [DsButtons.Mute] = X360Buttons.Back } }.Save(f);
            var p = MapProfile.Load(f);
            Assert.Equal(2, p.Curve);
            Assert.Equal(X360Buttons.Back, p.Remap![DsButtons.Mute]);
            File.WriteAllText(f, "{ not json");
            Assert.Equal(new MapProfile(), MapProfile.Load(f));
        }
        finally { File.Delete(f); }
    }

    [Fact]
    public void Output_report_usb_has_rumble_and_lightbar()
    {
        var r = OutputReport.Build(false, left: 200, right: 50, (1, 2, 3));
        Assert.Equal(OutputReport.UsbLength, r.Length);
        Assert.Equal(0x02, r[0]);
        Assert.Equal((50, 200), (r[3], r[4]));
        Assert.Equal(new byte[] { 1, 2, 3 }, r[45..48]);
        Assert.Equal(0, OutputReport.Build(false, 0, 0, null)[2]); // no lightbar flag
    }

    [Fact]
    public void Output_report_bluetooth_has_valid_crc()
    {
        var r = OutputReport.Build(true, 9, 8, null, btSeq: 3);
        Assert.Equal(OutputReport.BtLength, r.Length);
        Assert.Equal((0x31, 0x30, 0x10), (r[0], r[1], r[2]));
        Assert.Equal((9, 8), (r[6], r[5]));
        Assert.Equal(OutputReport.Crc32(r.AsSpan(0, 74)), BitConverter.ToUInt32(r, 74));
    }

    [Fact]
    public void Crc32_matches_zlib_and_colour_parses()
    {
        Assert.Equal(0x63da9f12U, OutputReport.Crc32("123456789"u8)); // zlib.crc32(0xA2 + body)
        Assert.Equal((255, 128, 0), OutputReport.ParseColour("#FF8000") is var (r, g, b) ? (r, g, b) : (-1, -1, -1));
        Assert.Null(OutputReport.ParseColour("nope"));
    }

    [Fact]
    public void DefaultFor_reports_builtin_targets()
    {
        Assert.Equal(X360Buttons.A, X360Mapper.DefaultFor(DsButtons.Cross));
        Assert.Null(X360Mapper.DefaultFor(DsButtons.Touchpad));
    }
}
