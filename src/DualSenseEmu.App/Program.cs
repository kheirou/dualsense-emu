using DualSenseEmu.Core;
using HidSharp;
using Nefarius.ViGEm.Client;

const int SonyVid = 0x054C;
int[] dualSensePids = [0x0CE6, 0x0DF2]; // DualSense, DualSense Edge
const int UsbInputLength = 64;
const byte BtEnableFeatureId = 0x05; // reading it switches Bluetooth to the full 0x31 report

var device = DeviceList.Local.GetHidDevices()
    .FirstOrDefault(d => d.VendorID == SonyVid && dualSensePids.Contains(d.ProductID));
if (device is null) { Console.Error.WriteLine("No DualSense found. Connect it by USB or Bluetooth."); return 1; }

HidStream stream;
try { stream = device.Open(); }
catch (Exception e) { Console.Error.WriteLine($"Cannot open DualSense: {e.Message}"); return 1; }

bool bluetooth = device.GetMaxInputReportLength() > UsbInputLength;
if (bluetooth)
{
    var feature = new byte[device.GetMaxFeatureReportLength()];
    feature[0] = BtEnableFeatureId;
    stream.GetFeature(feature);
}

ViGEmClient client;
try { client = new ViGEmClient(); }
catch (Exception e) { Console.Error.WriteLine($"ViGEmBus driver not available: {e.Message}"); return 1; }

var pad = client.CreateXbox360Controller();
pad.AutoSubmitReport = false;
pad.Connect();

using var cts = new CancellationTokenSource();
Console.CancelKeyPress += (_, e) => { e.Cancel = true; cts.Cancel(); };
Console.WriteLine($"Emulating Xbox 360 pad from {device.GetProductName()} over {(bluetooth ? "Bluetooth" : "USB")}. Ctrl+C to stop.");

var buf = new byte[device.GetMaxInputReportLength()];
try
{
    while (!cts.IsCancellationRequested)
    {
        int n = stream.Read(buf);
        if (!ReportParser.TryParse(buf.AsSpan(0, n), out var ds)) continue;
        var x = X360Mapper.Map(ds);
        pad.SetButtonsFull((ushort)x.Buttons);
        pad.SetSliderValue(Nefarius.ViGEm.Client.Targets.Xbox360.Xbox360Slider.LeftTrigger, x.LeftTrigger);
        pad.SetSliderValue(Nefarius.ViGEm.Client.Targets.Xbox360.Xbox360Slider.RightTrigger, x.RightTrigger);
        pad.SetAxisValue(Nefarius.ViGEm.Client.Targets.Xbox360.Xbox360Axis.LeftThumbX, x.LeftX);
        pad.SetAxisValue(Nefarius.ViGEm.Client.Targets.Xbox360.Xbox360Axis.LeftThumbY, x.LeftY);
        pad.SetAxisValue(Nefarius.ViGEm.Client.Targets.Xbox360.Xbox360Axis.RightThumbX, x.RightX);
        pad.SetAxisValue(Nefarius.ViGEm.Client.Targets.Xbox360.Xbox360Axis.RightThumbY, x.RightY);
        pad.SubmitReport();
    }
}
catch (Exception e) when (e is IOException or TimeoutException)
{
    Console.Error.WriteLine($"Controller disconnected: {e.Message}");
    return 1;
}
finally { pad.Disconnect(); stream.Dispose(); }
return 0;
