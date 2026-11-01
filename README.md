# dualsense-emu

Makes a DualSense show up as a virtual Xbox 360 pad on Windows (ViGEmBus), for games that ignore the DualSense.

## Run

```powershell
mise exec -- dotnet run --project src\DualSenseEmu.Gui -c Release   # window + tray
mise exec -- dotnet run --project src\DualSenseEmu.App -c Release   # console only, Ctrl+C to stop
```

Needs the ViGEmBus driver; if it is missing the app shows an "Install ViGEmBus driver" button.
The app keeps retrying, so it can start before the pad is switched on.

## Installer (Velopack)

```powershell
mise run dev:release 0.1.0   # -> releases\DualSenseEmu-win-Setup.exe (+ portable zip and update packages)
```

- Per-user install to `%LOCALAPPDATA%\DualSenseEmu`, no admin, Start menu shortcut, uninstall from Windows Settings.
- Installing puts the exe in a stable folder, so "Start with Windows" keeps working across updates.
- Auto-update: on start the app checks GitHub Releases and applies a newer version when it exits.
  Publish the `releases\` files to a GitHub Release (e.g. `gh release create v0.2.0 releases\*`).
  Only works if the repo is public: update checks on a private repo fail silently.

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
