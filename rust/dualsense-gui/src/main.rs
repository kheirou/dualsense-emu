#![windows_subsystem = "windows"]

mod app;
mod padview;
mod tray;

use eframe::egui;

const REPO_URL: &str = "https://github.com/kheirou/dualsense-emu";

/// Downloads a newer release, applied when the app exits, so play is never interrupted.
fn update_in_background() {
    use velopack::{sources::GithubSource, UpdateCheck, UpdateManager};
    // Err when not installed via Setup.exe (a dev run)
    let Ok(manager) = UpdateManager::new(GithubSource::new(REPO_URL, None, false), None, None)
    else {
        return;
    };
    // shortcut: update failures are silent (offline, rate limit, or a private repo); the app works without them.
    let Ok(UpdateCheck::UpdateAvailable(update)) = manager.check_for_updates() else {
        return;
    };
    if manager.download_updates(&update, None).is_ok() {
        // no restart: the update applies when the user exits from the tray; the new version starts at next login
        let _ = manager.wait_exit_then_apply_updates(
            &update.TargetFullRelease,
            true,
            false,
            Vec::<String>::new(),
        );
    }
}

fn main() -> eframe::Result {
    // Must run first: Velopack uses it for install / update / uninstall hooks, then exits.
    velopack::VelopackApp::build().run();

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

    std::thread::spawn(update_in_background);

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
