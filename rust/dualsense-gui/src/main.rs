#![windows_subsystem = "windows"]

mod app;
mod padview;
mod tray;

use eframe::egui;

fn main() -> eframe::Result {
    // One copy only: a second one would plug in a second virtual pad and fight over the real one.
    // shortcut: the second launch just exits (no "bring the first window forward"); use the tray icon.
    let lock = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(std::env::temp_dir().join("DualSenseEmu.lock"))
        .ok()
        .filter(|f| f.try_lock().is_ok());
    let Some(lock) = lock else { return Ok(()) };

    let tray_only = std::env::args().any(|a| a == "--tray");
    let (rgba, width, height) = tray::icon_rgba();
    let viewport = egui::ViewportBuilder::default()
        .with_title("DualSense Emulator")
        .with_inner_size([1040.0, 800.0])
        .with_resizable(false)
        .with_visible(!tray_only)
        .with_icon(egui::IconData {
            rgba,
            width,
            height,
        });
    eframe::run_native(
        "DualSense Emulator",
        eframe::NativeOptions {
            viewport,
            ..Default::default()
        },
        Box::new(|cc| Ok(Box::new(app::App::new(cc, lock)))),
    )
}
