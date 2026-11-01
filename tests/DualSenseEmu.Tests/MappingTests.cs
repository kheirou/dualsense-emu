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
}
