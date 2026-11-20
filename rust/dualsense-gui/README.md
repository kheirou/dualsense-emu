# dualsense-gui

egui window for the all-Rust rewrite. Dark sidebar + cream panel: Home (hero with Start/Stop, live pad diagrams, summary cards), Sticks, Buttons (remap), Lightbar, Settings (hide real pad, start with Windows). Tray icon (closing the window hides it; tray menu: Show / Exit), one instance at a time, `--tray` starts hidden. Uses `dualsense-core` and `dualsense-device`.

Run: `cargo run -p dualsense-gui` (from `rust/`).

- Not tested automatically: the window itself (only the pad-drawing helpers have unit tests). Checked by eye with a screenshot; the HID/ViGEm path still needs a real pad.
- Missing vs the Avalonia app: Velopack install/auto-update (phase 4), "bring the first window forward" on a second launch, custom title bar.
