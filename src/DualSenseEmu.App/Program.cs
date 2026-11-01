using DualSenseEmu.Device;

using var emu = new EmulatorService();
var done = new ManualResetEventSlim();
emu.StatusChanged += s => (s.State == EmuState.Error ? Console.Error : Console.Out).WriteLine(s.Message);
Console.CancelKeyPress += (_, e) => { e.Cancel = true; done.Set(); };
Console.WriteLine("Ctrl+C to stop.");
emu.Start();
done.Wait();
emu.Stop();
