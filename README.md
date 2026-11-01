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
