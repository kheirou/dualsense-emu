//! System tray icon: click or "Show window" brings the window back, "Exit" quits.

use eframe::egui;
use std::sync::atomic::{AtomicBool, Ordering::Relaxed};
use std::sync::Arc;
use std::time::Duration;
use tray_icon::menu::{Menu, MenuEvent, MenuItem};
use tray_icon::{Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};

const ICON_PNG: &[u8] = include_bytes!("../assets/icon.png");

pub fn icon_rgba() -> (Vec<u8>, u32, u32) {
    let img = image::load_from_memory(ICON_PNG)
        .expect("embedded icon is a valid PNG")
        .into_rgba8();
    let (w, h) = img.dimensions();
    (img.into_raw(), w, h)
}

fn show_window(ctx: &egui::Context) {
    ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
    ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
    ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
    ctx.request_repaint();
}

fn quit(ctx: &egui::Context, exiting: &AtomicBool) {
    exiting.store(true, Relaxed);
    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    ctx.request_repaint();
    // a hidden window may never run the close; the OS unplugs the virtual pad when the process ends
    std::thread::spawn(|| {
        std::thread::sleep(Duration::from_millis(1500));
        std::process::exit(0);
    });
}

/// None when the tray cannot be created: the app still works, the window just cannot hide to the tray.
pub fn create(ctx: &egui::Context, exiting: Arc<AtomicBool>) -> Option<TrayIcon> {
    let menu = Menu::new();
    let show = MenuItem::with_id("show", "Show window", true, None);
    let exit = MenuItem::with_id("exit", "Exit", true, None);
    menu.append_items(&[&show, &exit]).ok()?;
    let (rgba, w, h) = icon_rgba();
    let tray = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_menu_on_left_click(false)
        .with_tooltip("DualSense Emulator")
        .with_icon(Icon::from_rgba(rgba, w, h).ok()?)
        .build()
        .ok()?;
    let c = ctx.clone();
    MenuEvent::set_event_handler(Some(move |e: MenuEvent| match e.id.0.as_str() {
        "show" => show_window(&c),
        "exit" => quit(&c, &exiting),
        _ => {}
    }));
    let c = ctx.clone();
    TrayIconEvent::set_event_handler(Some(move |e: TrayIconEvent| {
        if let TrayIconEvent::Click {
            button: MouseButton::Left,
            button_state: MouseButtonState::Up,
            ..
        } = e
        {
            show_window(&c);
        }
    }));
    Some(tray)
}
