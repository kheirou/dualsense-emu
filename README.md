# dualsense-emu

Makes a DualSense show up as a virtual Xbox 360 pad on Windows (ViGEmBus), for games that ignore the DualSense.

## Run

```powershell
mise exec -- dotnet run --project src\DualSenseEmu.Gui -c Release   # window + tray
mise exec -- dotnet run --project src\DualSenseEmu.App -c Release   # console only, Ctrl+C to stop
```

Needs the ViGEmBus driver. The app keeps retrying, so it can start before the pad is switched on.

## Standalone exe

```powershell
mise run dev:publish     # builds dist\DualSenseEmu.exe (self-contained, ~45 MB, no .NET needed)
```

- Closing the window hides it to the tray (tray menu: Show / Exit). It keeps emulating. Only one copy runs at a time.
- "Start with Windows" registers the exe where it runs now, with `--tray` (published exe only). Rebuild or move it and the entry breaks.
- Header shows connection (Bluetooth / wired) and battery level with charge state.
- Live view draws a DualSense and an Xbox 360 pad side by side; pressed buttons light up on both.

## Hide the real pad (avoid double input)

In the window, switch on "Hide real pad". It uses HidHide (the app can install it) and asks for admin once.
It hides the pad as currently connected: after switching between USB and Bluetooth, toggle it off and on again.

## Dev

```powershell
mise run dev:ci          # build + tests
```
