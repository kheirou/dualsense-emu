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

    [Fact]
    public void Chain_stops_on_first_failure_and_covers_pad_app_and_cloak()
    {
        var chain = HidHide.BuildChain(true, ["HID\\A\\1", "HID\\B\\2"], @"C:\Apps\DualSenseEmu.exe");
        Assert.Equal(3, chain.Split(" && ").Length - 1);               // 4 commands joined by &&
        Assert.DoesNotContain(" & ", chain.Replace(" && ", "|"));      // no bare & between commands
        Assert.Contains("--dev-hide \"HID\\A\\1\"", chain);
        Assert.Contains("--app-reg \"C:\\Apps\\DualSenseEmu.exe\"", chain);
        Assert.EndsWith("--cloak-on", chain);
    }

    [Fact]
    public void Undo_chain_reverses_everything()
    {
        var chain = HidHide.BuildChain(false, ["HID\\A\\1"], @"C:\x.exe");
        Assert.Contains("--dev-unhide", chain);
        Assert.Contains("--app-unreg", chain);
        Assert.EndsWith("--cloak-off", chain);
    }

    [Theory]
    [InlineData("HID\\VID_054C&PID_0CE6\\1\" & calc & \"")]   // quote breaks out of the quoting
    [InlineData("HID\\VID_054C&PID_0CE6\\1 2")]                // space
    [InlineData("HID\\VID_054C&PID_0CE6\\%PATH%")]             // cmd variable expansion
    public void Unsafe_instance_paths_are_dropped(string evil)
    {
        var json = "[{\"devices\":[{\"deviceInstancePath\":" + System.Text.Json.JsonSerializer.Serialize(evil) + "}]}]";
        Assert.Empty(HidHide.ParseDualSensePaths(json));
    }

    [Theory]
    [InlineData("C:\\a\"b.exe")]
    [InlineData("C:\\%TEMP%\\x.exe")]
    public void Unsafe_app_path_is_refused(string app)
        => Assert.Throws<ArgumentException>(() => HidHide.BuildChain(true, ["HID\\A\\1"], app));
}
