using DualSenseEmu.Core;
using HidSharp;
using Nefarius.ViGEm.Client;
using Nefarius.ViGEm.Client.Targets;
using Nefarius.ViGEm.Client.Targets.Xbox360;

namespace DualSenseEmu.Device;

public enum EmuState { Stopped, Waiting, Running, Error }

public enum Transport { None, Usb, Bluetooth }

public sealed record EmuStatus(EmuState State, string Message);

public sealed record Snapshot(DualSenseState Ds, X360State X360);

/// <summary>Reads a DualSense over HID and mirrors it to a virtual Xbox 360 pad until stopped.</summary>
public sealed class EmulatorService : IDisposable
{
    const int SonyVid = 0x054C;
    static readonly int[] DualSensePids = [0x0CE6, 0x0DF2]; // DualSense, DualSense Edge
    const int UsbInputLength = 64;
    const byte BtEnableFeatureId = 0x05; // reading it switches Bluetooth to the full 0x31 report

    CancellationTokenSource? _cts;
    Task? _task;

    public event Action<EmuStatus>? StatusChanged;

    /// <summary>Latest input; written from the reader thread, polled by the UI (a stale frame is harmless).</summary>
    public Snapshot? Latest { get; private set; }

    public bool IsRunning => _task is { IsCompleted: false };

    /// <summary>How the pad is connected right now (None while not emulating).</summary>
    public Transport Transport { get; private set; }

    public void Start()
    {
        if (IsRunning) return;
        _cts = new CancellationTokenSource();
        var ct = _cts.Token;
        _task = Task.Run(() => Run(ct));
    }

    public void Stop()
    {
        _cts?.Cancel();
        _task?.Wait(3000);
    }

    EmuStatus? _last;

    void Report(EmuState s, string msg)
    {
        var status = new EmuStatus(s, msg);
        if (status == _last) return; // the retry loop would otherwise repeat the same message every 2 s
        _last = status;
        StatusChanged?.Invoke(status);
    }

    /// <summary>Keep trying until stopped: the pad may be off at login, or drop off Bluetooth mid-game.</summary>
    void Run(CancellationToken ct)
    {
        while (!ct.IsCancellationRequested)
        {
            try { RunOnce(ct); }
            catch (Exception e)
            {
                Report(EmuState.Error, e is Nefarius.ViGEm.Client.Exceptions.VigemBusNotFoundException
                    ? "ViGEmBus driver not available. Retrying..." : $"Controller error: {e.Message}. Retrying...");
            }
            ct.WaitHandle.WaitOne(2000);
        }
        Report(EmuState.Stopped, "Stopped");
    }

    void RunOnce(CancellationToken ct)
    {
        HidStream? stream = null;
        IXbox360Controller? pad = null;
        ViGEmClient? client = null;
        try
        {
            var device = DeviceList.Local.GetHidDevices()
                .FirstOrDefault(d => d.VendorID == SonyVid && DualSensePids.Contains(d.ProductID));
            if (device is null) { Report(EmuState.Waiting, "Waiting for a DualSense (USB or Bluetooth)..."); return; }

            stream = device.Open();
            stream.ReadTimeout = 500; // lets Stop() take effect even if the pad goes quiet
            bool bluetooth = device.GetMaxInputReportLength() > UsbInputLength;
            Transport = bluetooth ? Transport.Bluetooth : Transport.Usb;
            if (bluetooth)
            {
                var feature = new byte[device.GetMaxFeatureReportLength()];
                feature[0] = BtEnableFeatureId;
                stream.GetFeature(feature);
            }

            client = new ViGEmClient();
            pad = client.CreateXbox360Controller();
            pad.AutoSubmitReport = false;
            pad.Connect();
            Report(EmuState.Running, $"Emulating Xbox 360 pad from {device.GetProductName()} over {(bluetooth ? "Bluetooth" : "USB")}");

            var buf = new byte[device.GetMaxInputReportLength()];
            while (!ct.IsCancellationRequested)
            {
                int n;
                try { n = stream.Read(buf); }
                catch (TimeoutException) { continue; }
                if (!ReportParser.TryParse(buf.AsSpan(0, n), out var ds)) continue;
                var x = X360Mapper.Map(ds);
                Latest = new Snapshot(ds, x);
                pad.SetButtonsFull((ushort)x.Buttons);
                pad.SetSliderValue(Xbox360Slider.LeftTrigger, x.LeftTrigger);
                pad.SetSliderValue(Xbox360Slider.RightTrigger, x.RightTrigger);
                pad.SetAxisValue(Xbox360Axis.LeftThumbX, x.LeftX);
                pad.SetAxisValue(Xbox360Axis.LeftThumbY, x.LeftY);
                pad.SetAxisValue(Xbox360Axis.RightThumbX, x.RightX);
                pad.SetAxisValue(Xbox360Axis.RightThumbY, x.RightY);
                pad.SubmitReport();
            }
        }
        finally
        {
            try { pad?.Disconnect(); } catch { /* already gone */ }
            client?.Dispose();
            stream?.Dispose();
            Latest = null;
            Transport = Transport.None;
        }
    }

    public void Dispose() => Stop();
}
