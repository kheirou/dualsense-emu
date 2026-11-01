using DualSenseEmu.Device;

namespace DualSenseEmu.Tests;

public class HidHideTests
{
    // Trimmed real output of `HidHideCLI --dev-gaming`.
    const string Sample = """
    [ { "friendlyName" : "Controller (XBOX 360 For Windows)" , "devices" : [
    { "present" : true , "deviceInstancePath" : "HID\\VID_045E&PID_028E&IG_00\\3&34f4d81e&0&0000" } ] } ,
    { "friendlyName" : "Sony Interactive Entertainment DualSense Wireless Controller" , "devices" : [
    { "present" : true , "deviceInstancePath" : "HID\\{00001124-0000-1000-8000-00805f9b34fb}_VID&0002054c_PID&0ce6\\8&2c76ced&0&0000" } ] } ,
    { "friendlyName" : "Wireless Controller" , "devices" : [
    { "present" : true , "deviceInstancePath" : "HID\\VID_054C&PID_0DF2\\7&1&0&0000" } ] } ]
    """;

    [Fact]
    public void Finds_only_dualsense_and_edge()
    {
        var paths = HidHide.ParseDualSensePaths(Sample);
        Assert.Equal(2, paths.Count);
        Assert.DoesNotContain(paths, p => p.Contains("045E"));
    }

    [Fact]
    public void Empty_list_gives_nothing() => Assert.Empty(HidHide.ParseDualSensePaths("[]"));
}
