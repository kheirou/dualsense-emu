using DualSenseEmu.Device;

using var emu = new EmulatorService();
var done = new ManualResetEventSlim();
int exit = 0;
emu.StatusChanged += s =>
{
    if (s.State == EmuState.Error) { Console.Error.WriteLine(s.Message); exit = 1; }
    else Console.WriteLine(s.Message);
    if (s.State != EmuState.Running) done.Set();
};
Console.CancelKeyPress += (_, e) => { e.Cancel = true; done.Set(); };
Console.WriteLine("Ctrl+C to stop.");
emu.Start();
done.Wait();
emu.Stop();
return exit;
