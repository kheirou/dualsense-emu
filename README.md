# dualsense-emu

DualSense to virtual Xbox 360 pad (ViGEmBus). Work in progress.


## Run

```powershell
mise exec -- dotnet run --project src\DualSenseEmu.App -c Release
```

## Hide the real pad (avoid double input)

Needs HidHide (`winget install Nefarius.HidHide`) and gsudo (`winget install gerardog.gsudo`); the script elevates itself:

```powershell
pwsh scripts\hide-dualsense.ps1          # hide real DualSense, whitelist dotnet, cloak on
pwsh scripts\hide-dualsense.ps1 -Undo    # reverse it
pwsh scripts\hide-dualsense.ps1 -WhatIf  # preview only
```

## GUI

```powershell
mise exec -- dotnet run --project src\DualSenseEmu.Gui -c Release
```

Window shows status, Start/Stop, a "Hide real pad" toggle (asks for admin via the script) and a live input view.
The console app (`src\DualSenseEmu.App`) still works and shares the same emulator code (`src\DualSenseEmu.Device`).

The GUI talks to HidHide directly (one UAC prompt to hide or unhide) and can install HidHide for you. The PowerShell script remains as a CLI alternative.


## Standalone exe, tray, start at login

```powershell
mise run dev:publish     # builds dist\DualSenseEmu.exe (self-contained, ~45 MB, no .NET needed)
```

- Closing the window hides it to the tray (tray menu: Show / Exit). It keeps emulating.
- "Start with Windows" in the window registers the exe with `--tray` (only enabled for the published exe).
- Header shows connection (Bluetooth / wired) and battery level with charge state.
- Live view draws a DualSense and an Xbox 360 pad side by side; pressed buttons light up on both.
