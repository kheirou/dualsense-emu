using DualSenseEmu.Core;
using HidSharp;
using Nefarius.ViGEm.Client;
using Nefarius.ViGEm.Client.Targets;
using Nefarius.ViGEm.Client.Targets.Xbox360;

namespace DualSenseEmu.Device;

public enum EmuState { Stopped, Waiting, Running, Error, DriverMissing }

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
        _task?.Wait(); // full wait: a timed-out Stop would leave IsRunning true and make the next Start a silent no-op
    }

    EmuStatus? _last;

    void Report(EmuState s, string msg)
    {
        var status = new EmuStatus(s, msg);
        if (status == _last) return; // the retry loop would otherwise repeat the same message every 2 s
        _last = status;
        StatusChanged?.Invoke(status);
    }

    // Created on first connect and kept across pad drops, so a Bluetooth blip does not unplug the Xbox pad from the game.
    ViGEmClient? _client;
    IXbox360Controller? _pad;

    /// <summary>Keep trying until stopped: the pad may be off at login, or drop off Bluetooth mid-game.</summary>
    void Run(CancellationToken ct)
    {
        try
        {
            while (!ct.IsCancellationRequested)
            {
                try { RunOnce(ct); }
                catch (Nefarius.ViGEm.Client.Exceptions.VigemBusNotFoundException)
                {
                    ReleasePad();
                    Report(EmuState.DriverMissing, "ViGEmBus driver is not installed.");
                }
                catch (IOException e) { Report(EmuState.Error, $"Controller error: {e.Message}. Retrying..."); }
                catch (Exception e)
                {
                    ReleasePad(); // not a pad-read problem: rebuild the virtual pad on the next try
                    Report(EmuState.Error, $"Controller error: {e.Message}. Retrying...");
                }
                ct.WaitHandle.WaitOne(2000);
            }
        }
        finally { ReleasePad(); }
        Report(EmuState.Stopped, "Stopped");
    }

    void ReleasePad()
    {
        try { _pad?.Disconnect(); } catch { /* already gone */ }
        _client?.Dispose();
        _pad = null; _client = null;
    }

    IXbox360Controller EnsurePad()
    {
        if (_pad is not null) return _pad;
        _client = new ViGEmClient(); // throws VigemBusNotFoundException when the driver is missing
        var pad = _client.CreateXbox360Controller();
        pad.AutoSubmitReport = false;
        pad.Connect();
        return _pad = pad;
    }

    void RunOnce(CancellationToken ct)
    {
        HidStream? stream = null;
        try
        {
            var device = DeviceList.Local.GetHidDevices()
                .FirstOrDefault(d => d.VendorID == SonyVid && DualSensePids.Contains(d.ProductID));
            if (device is null) { Report(EmuState.Waiting, "Waiting for a DualSense (USB or Bluetooth)..."); return; }

            var pad = EnsurePad(); // before touching the HID device, so a missing driver does not poke it every 2 s

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

            Report(EmuState.Running, $"Emulating: games see an Xbox 360 pad ({(bluetooth ? "Bluetooth" : "USB")})");

            var buf = new byte[device.GetMaxInputReportLength()];
            while (!ct.IsCancellationRequested)
            {
                int n;
                try { n = stream.Read(buf); }
                catch (TimeoutException) { continue; }
                if (!ReportParser.TryParse(buf.AsSpan(0, n), out var ds)) continue;
                var x = X360Mapper.Map(ds);
                Latest = new Snapshot(ds, x);
                Submit(pad, x);
            }
        }
        finally
        {
            if (_pad is not null) // real pad gone: release everything the game could see as held
                try { Submit(_pad, X360Mapper.Map(new DualSenseState(128, 128, 128, 128, 0, 0, DsButtons.None))); } catch { /* pad already gone */ }
            stream?.Dispose();
            Latest = null;
            Transport = Transport.None;
        }
    }

    static void Submit(IXbox360Controller pad, X360State x)
    {
        pad.SetButtonsFull((ushort)x.Buttons);
        pad.SetSliderValue(Xbox360Slider.LeftTrigger, x.LeftTrigger);
        pad.SetSliderValue(Xbox360Slider.RightTrigger, x.RightTrigger);
        pad.SetAxisValue(Xbox360Axis.LeftThumbX, x.LeftX);
        pad.SetAxisValue(Xbox360Axis.LeftThumbY, x.LeftY);
        pad.SetAxisValue(Xbox360Axis.RightThumbX, x.RightX);
        pad.SetAxisValue(Xbox360Axis.RightThumbY, x.RightY);
        pad.SubmitReport();
    }

    public void Dispose() => Stop();
}
