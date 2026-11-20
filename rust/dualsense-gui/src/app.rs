//! The window: sidebar + Home / Sticks / Buttons / Lightbar / Settings pages. Mirrors the Avalonia GUI it replaces.

use crate::padview::{self, PadInput};
use dualsense_core::output::parse_colour;
use dualsense_core::profile::{DsButton, LightbarMode, MapProfile, PlayerLedMode, X360Button};
use dualsense_core::{mapper, state::ChargeState};
use dualsense_device::hidhide;
use dualsense_device::service::{EmuState, EmuStatus, EmulatorService, Transport};
use dualsense_device::status_file;
use eframe::egui::{
    self, Align2, Color32, CornerRadius, FontId, Margin, Rect, RichText, Sense, Shape, Stroke,
    StrokeKind, Vec2,
};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering::Relaxed};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

// Cream main panel on a tan window, dark sidebar; the hero and live strip are dark scenes.
const TAN: Color32 = Color32::from_rgb(0xB9, 0xB2, 0xA4);
const SIDEBAR: Color32 = Color32::from_rgb(0x2A, 0x27, 0x33);
const SIDE_ON: Color32 = Color32::from_rgb(0x3A, 0x36, 0x45);
const SIDE_TEXT: Color32 = Color32::from_rgb(0xF2, 0xED, 0xE0);
const SIDE_DIM: Color32 = Color32::from_rgb(0x9D, 0x98, 0xA8);
const SCENE_DIM: Color32 = Color32::from_rgb(0xC3, 0xD3, 0xCF);
const CREAM: Color32 = Color32::from_rgb(0xEB, 0xE5, 0xD7);
const CARD: Color32 = Color32::from_rgb(0xE2, 0xDB, 0xC9);
const RAISED: Color32 = Color32::from_rgb(0xD8, 0xD0, 0xBB);
const HOVER: Color32 = Color32::from_rgb(0xCE, 0xC5, 0xAE);
const BORDER: Color32 = Color32::from_rgb(0xCF, 0xC7, 0xB2);
const OUTLINE: Color32 = Color32::from_rgb(0xA3, 0x9A, 0x82);
const TILE_OFF: Color32 = Color32::from_rgb(0xBF, 0xB6, 0x9F);
const TEXT: Color32 = Color32::from_rgb(0x22, 0x1F, 0x27);
const SOFT: Color32 = Color32::from_rgb(0x4B, 0x46, 0x40);
const DIM: Color32 = Color32::from_rgb(0x70, 0x6A, 0x5C);
const ACCENT: Color32 = Color32::from_rgb(0x3A, 0x66, 0xB0);
const AMBER: Color32 = Color32::from_rgb(0xC9, 0x8A, 0x1B);
const GREEN: Color32 = Color32::from_rgb(0x3B, 0x8A, 0x55);
const RED: Color32 = Color32::from_rgb(0xC2, 0x44, 0x3E);
const NAV: [(Tab, &str, Color32); 5] = [
    (Tab::Dashboard, "Home", SIDE_TEXT),
    (Tab::Sticks, "Sticks", RED),
    (Tab::Buttons, "Buttons", AMBER),
    (Tab::Lightbar, "Lightbar", GREEN),
    (Tab::Settings, "Settings", ACCENT),
];
const PAD_SCALE: f32 = 1.2;

const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const RUN_VALUE: &str = "DualSenseEmu";

const DS_BUTTONS: [(&str, DsButton); 17] = [
    ("Square", DsButton::Square),
    ("Cross", DsButton::Cross),
    ("Circle", DsButton::Circle),
    ("Triangle", DsButton::Triangle),
    ("L1", DsButton::L1),
    ("R1", DsButton::R1),
    ("Create", DsButton::Create),
    ("Options", DsButton::Options),
    ("L3", DsButton::L3),
    ("R3", DsButton::R3),
    ("Ps", DsButton::Ps),
    ("Touchpad", DsButton::Touchpad),
    ("Mute", DsButton::Mute),
    ("DpadUp", DsButton::DpadUp),
    ("DpadDown", DsButton::DpadDown),
    ("DpadLeft", DsButton::DpadLeft),
    ("DpadRight", DsButton::DpadRight),
];

const X360_BUTTONS: [(&str, X360Button); 15] = [
    ("DpadUp", X360Button::DpadUp),
    ("DpadDown", X360Button::DpadDown),
    ("DpadLeft", X360Button::DpadLeft),
    ("DpadRight", X360Button::DpadRight),
    ("Start", X360Button::Start),
    ("Back", X360Button::Back),
    ("LeftThumb", X360Button::LeftThumb),
    ("RightThumb", X360Button::RightThumb),
    ("LeftShoulder", X360Button::LeftShoulder),
    ("RightShoulder", X360Button::RightShoulder),
    ("Guide", X360Button::Guide),
    ("A", X360Button::A),
    ("B", X360Button::B),
    ("X", X360Button::X),
    ("Y", X360Button::Y),
];

const PRESETS: [(&str, &str); 9] = [
    ("Red", "#FF3B30"),
    ("Orange", "#FF9500"),
    ("Yellow", "#FFD60A"),
    ("Green", "#3DDC84"),
    ("Cyan", "#3FE0FF"),
    ("Blue", "#3B6CFF"),
    ("Purple", "#9B5CFF"),
    ("Pink", "#FF5CA8"),
    ("White", "#FFFFFF"),
];

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Dashboard,
    Sticks,
    Buttons,
    Lightbar,
    Settings,
}

/// Results of work done off the UI thread (UAC prompts and installs block for a long time).
enum Msg {
    HideState { installed: bool, on: bool },
    HideDone { ok: bool, installed: bool, on: bool },
    HidHideInstalled { ok: bool, installed: bool, on: bool },
    VigemInstalled(bool),
}

struct HideUi {
    installed: bool,
    on: bool,
    busy: bool,
    text: String,
}

pub struct App {
    emu: Arc<EmulatorService>,
    status: Arc<Mutex<EmuStatus>>,
    profile: MapProfile,
    profile_path: PathBuf,
    dirty: bool,  // profile changed, not yet saved
    hidden: bool, // window is hidden in the tray
    tab: Tab,
    remap_sel: DsButton,
    hex_text: String,
    hide: HideUi,
    startup_on: bool,
    vigem_note: Option<String>,
    tx: Sender<Msg>,
    rx: Receiver<Msg>,
    exiting: Arc<AtomicBool>,
    quit_status: Arc<AtomicBool>,
    status_thread: Option<JoinHandle<()>>,
    icon: egui::TextureHandle,
    _tray: Option<tray_icon::TrayIcon>,
    _lock: std::fs::File,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>, lock: std::fs::File) -> Self {
        let ctx = cc.egui_ctx.clone();
        style(&ctx);
        let profile_path = MapProfile::default_path();
        let profile = MapProfile::load(&profile_path);
        let status = Arc::new(Mutex::new(EmuStatus {
            state: EmuState::Stopped,
            message: String::new(),
        }));
        let emu = {
            let (status, ctx) = (status.clone(), ctx.clone());
            Arc::new(EmulatorService::new(profile.clone(), move |s| {
                *status.lock().unwrap_or_else(|e| e.into_inner()) = s.clone();
                ctx.request_repaint();
            }))
        };
        emu.start(); // emulate straight away; Stop is one click

        let exiting = Arc::new(AtomicBool::new(false));
        let quit_status = Arc::new(AtomicBool::new(false));
        // keeps status.json fresh even while the window is hidden in the tray
        let status_thread = {
            let (emu, status, quit) = (emu.clone(), status.clone(), quit_status.clone());
            std::thread::spawn(move || {
                let mut last = String::new();
                while !quit.load(Relaxed) {
                    let state = status.lock().unwrap_or_else(|e| e.into_inner()).state;
                    let json = status_file::to_json(
                        state,
                        emu.transport(),
                        emu.latest().map(|s| s.ds).as_ref(),
                    );
                    if json != last {
                        status_file::write(&json, None);
                        last = json;
                    }
                    std::thread::sleep(Duration::from_millis(250));
                }
            })
        };

        let (tx, rx) = channel();
        spawn_hide_state(&tx, &ctx);
        let hex_text = profile.lightbar.clone().unwrap_or_default();
        let (rgba, w, h) = crate::tray::icon_rgba();
        let icon = ctx.load_texture(
            "icon",
            egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &rgba),
            Default::default(),
        );
        Self {
            emu,
            status,
            profile,
            profile_path,
            dirty: false,
            hidden: false,
            tab: Tab::Dashboard,
            remap_sel: DsButton::Cross,
            hex_text,
            hide: HideUi {
                installed: hidhide::is_installed(),
                on: false,
                busy: false,
                text: String::new(),
            },
            startup_on: startup_enabled(),
            vigem_note: None,
            tx,
            rx,
            exiting: exiting.clone(),
            quit_status,
            status_thread: Some(status_thread),
            icon,
            _tray: crate::tray::create(&ctx, exiting),
            _lock: lock,
        }
    }

    fn apply(&mut self) {
        self.emu.set_profile(self.profile.clone());
        self.dirty = true; // saved once the pointer is released, not on every slider tick
    }

    fn poll(&mut self, ctx: &egui::Context) {
        while let Ok(m) = self.rx.try_recv() {
            match m {
                Msg::HideState { installed, on } => {
                    self.hide.installed = installed;
                    self.hide.on = on;
                    self.hide.text = hide_text(installed, on);
                }
                Msg::HideDone { ok, installed, on } => {
                    self.hide.busy = false;
                    self.hide.installed = installed;
                    self.hide.on = on;
                    self.hide.text = if ok {
                        hide_text(installed, on)
                    } else {
                        "Not changed: approval declined, or no DualSense connected.".into()
                    };
                }
                Msg::HidHideInstalled { ok, installed, on } => {
                    self.hide.busy = false;
                    self.hide.installed = installed;
                    self.hide.on = on;
                    self.hide.text = if ok && installed {
                        hide_text(installed, on)
                    } else {
                        "HidHide install failed or was cancelled. A restart may be needed after installing.".into()
                    };
                }
                Msg::VigemInstalled(ok) => {
                    self.vigem_note =
                        (!ok).then(|| "ViGEmBus install failed or was cancelled.".to_string());
                }
            }
        }
        // closing the window only hides it to the tray; the tray Exit item really quits
        if ctx.input(|i| i.viewport().close_requested()) && !self.exiting.load(Relaxed) {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
            self.hidden = true;
        }
    }
}

impl eframe::App for App {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll(ctx);
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        if ctx.input(|i| i.viewport().focused == Some(true)) {
            self.hidden = false; // the tray's Show window focuses it
        }
        // 30 fps only while the live pad view is on screen; the status thread wakes it on changes
        let fps = match (self.hidden, self.tab) {
            (true, _) => Duration::from_secs(1),
            (_, Tab::Dashboard) => Duration::from_millis(33),
            _ => Duration::from_millis(250),
        };
        ctx.request_repaint_after(fps);
        if self.dirty && !ctx.input(|i| i.pointer.any_down()) {
            self.dirty = false;
            let _ = self.profile.save(&self.profile_path); // a read-only profile folder must not stop the pad
        }
        let full = ui.max_rect().shrink(12.0);
        let side = Rect::from_min_size(full.min, Vec2::new(190.0, full.height()));
        let main = Rect::from_min_max(egui::pos2(side.max.x + 12.0, full.min.y), full.max);
        ui.painter().rect_filled(side, 18, SIDEBAR);
        ui.painter().rect_filled(main, 18, CREAM);
        ui.scope_builder(egui::UiBuilder::new().max_rect(side.shrink(14.0)), |ui| {
            self.sidebar(ui)
        });
        ui.scope_builder(egui::UiBuilder::new().max_rect(main.shrink(20.0)), |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    let title = match self.tab {
                        Tab::Dashboard => None,
                        Tab::Sticks => Some("Sticks and triggers"),
                        Tab::Buttons => Some("Button remapping"),
                        Tab::Lightbar => Some("Lightbar and player LEDs"),
                        Tab::Settings => Some("Settings"),
                    };
                    if let Some(t) = title {
                        ui.label(RichText::new(t).size(22.0).color(TEXT));
                        ui.add_space(10.0);
                    }
                    match self.tab {
                        Tab::Dashboard => self.dashboard(ui),
                        Tab::Sticks => self.sticks(ui),
                        Tab::Buttons => self.buttons(ui),
                        Tab::Lightbar => self.lightbar(ui),
                        Tab::Settings => card(ui, |ui| {
                            ui.set_width(ui.available_width().max(0.0));
                            self.settings(ui, &ctx);
                        }),
                    }
                });
        });
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        TAN.to_normalized_gamma_f32()
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.quit_status.store(true, Relaxed);
        self.emu.stop();
        if let Some(t) = self.status_thread.take() {
            let _ = t.join();
        }
    }
}

// ---------- shell: sidebar, hero, tiles ----------

impl App {
    fn sidebar(&mut self, ui: &mut egui::Ui) {
        let rect = ui.max_rect();
        let icon = self.icon.clone();
        ui.horizontal(|ui| {
            ui.add(
                egui::Image::new(&icon)
                    .fit_to_exact_size(Vec2::splat(30.0))
                    .corner_radius(8),
            );
            ui.label(RichText::new("DualSense Emu").size(18.0).color(SIDE_TEXT));
        });
        ui.add_space(18.0);
        for (tab, name, tick) in NAV {
            let (r, resp) =
                ui.allocate_exact_size(Vec2::new(ui.available_width(), 38.0), Sense::click());
            let on = self.tab == tab;
            let p = ui.painter();
            if on || resp.hovered() {
                p.rect_filled(
                    r,
                    10,
                    if on {
                        SIDE_ON
                    } else {
                        SIDE_ON.gamma_multiply(0.5)
                    },
                );
            }
            let tick = if on { tick } else { tick.gamma_multiply(0.5) };
            p.rect_filled(
                Rect::from_min_size(r.min + Vec2::new(0.0, 11.0), Vec2::new(3.0, 16.0)),
                2,
                tick,
            );
            p.text(
                r.min + Vec2::new(18.0, 19.0),
                Align2::LEFT_CENTER,
                name,
                FontId::proportional(15.0),
                if on { SIDE_TEXT } else { SIDE_DIM },
            );
            if resp.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            if resp.clicked() {
                self.tab = tab;
            }
        }
        let (transport, dot) = match self.emu.transport() {
            Transport::Bluetooth => ("Bluetooth", GREEN),
            Transport::Usb => ("USB", GREEN),
            Transport::None => ("No pad", SIDE_DIM),
        };
        let pill = Rect::from_min_size(
            egui::pos2(rect.min.x, rect.max.y - 44.0),
            Vec2::new(rect.width(), 44.0),
        );
        let p = ui.painter();
        p.rect_filled(pill, 12, SIDE_ON);
        p.text(
            pill.left_center() + Vec2::new(12.0, 0.0),
            Align2::LEFT_CENTER,
            concat!("v", env!("CARGO_PKG_VERSION")),
            FontId::proportional(14.0),
            SIDE_TEXT,
        );
        p.circle_filled(pill.right_center() - Vec2::new(66.0, 0.0), 4.0, dot);
        p.text(
            pill.right_center() - Vec2::new(12.0, 0.0),
            Align2::RIGHT_CENTER,
            transport,
            FontId::proportional(12.0),
            SIDE_DIM,
        );
    }

    fn hero(&mut self, ui: &mut egui::Ui, status: &EmuStatus, active: bool) {
        let (rect, _) =
            ui.allocate_exact_size(Vec2::new(ui.available_width(), 230.0), Sense::hover());
        paint_scene(ui.painter(), rect);
        let snap = self.emu.latest();
        let word = match status.state {
            EmuState::Running => "Emulating",
            EmuState::Waiting => "Waiting for pad",
            EmuState::Error => "Error",
            EmuState::DriverMissing => "Driver missing",
            EmuState::Stopped => "Off",
        };
        let transport = match self.emu.transport() {
            Transport::Bluetooth => "Bluetooth",
            Transport::Usb => "USB cable",
            Transport::None => "Not connected",
        };
        let battery = snap
            .as_ref()
            .map(|s| s.ds)
            .filter(|d| d.battery_percent >= 0)
            .map_or("--".to_string(), |d| {
                let c = match d.charge {
                    ChargeState::Charging => " charging",
                    ChargeState::Full => " full",
                    _ => "",
                };
                format!("{}%{c}", d.battery_percent)
            });
        let note = self.vigem_note.clone().unwrap_or_else(|| {
            if status.state == EmuState::Running {
                "Games see an Xbox 360 pad.".into()
            } else {
                status.message.clone()
            }
        });
        let p = ui.painter().clone();
        p.text(
            rect.min + Vec2::new(24.0, 20.0),
            Align2::LEFT_TOP,
            "Controller emulation",
            FontId::proportional(30.0),
            SIDE_TEXT,
        );
        let g = p.layout(
            note,
            FontId::proportional(14.0),
            SIDE_TEXT,
            rect.width() * 0.6,
        );
        p.galley(rect.min + Vec2::new(24.0, 62.0), g, SIDE_TEXT);
        for (i, (k, v)) in [
            ("Pad", transport.to_string()),
            ("Battery", battery),
            ("Emulation", word.to_string()),
        ]
        .into_iter()
        .enumerate()
        {
            let x = rect.min.x + 24.0 + i as f32 * 150.0;
            p.text(
                egui::pos2(x, rect.max.y - 66.0),
                Align2::LEFT_TOP,
                k,
                FontId::proportional(16.0),
                SIDE_TEXT,
            );
            p.text(
                egui::pos2(x, rect.max.y - 42.0),
                Align2::LEFT_TOP,
                v,
                FontId::proportional(12.5),
                SCENE_DIM,
            );
        }
        if status.state == EmuState::DriverMissing {
            let r = Rect::from_min_size(rect.min + Vec2::new(24.0, 100.0), Vec2::new(190.0, 30.0));
            if ui
                .put(r, egui::Button::new("Install ViGEmBus driver"))
                .clicked()
            {
                self.vigem_note = Some("Installing ViGEmBus (approve the prompt)...".into());
                let (tx, ctx) = (self.tx.clone(), ui.ctx().clone());
                std::thread::spawn(move || {
                    let _ = tx.send(Msg::VigemInstalled(hidhide::install(
                        hidhide::VIGEM_BUS_PACKAGE,
                    )));
                    ctx.request_repaint();
                });
            }
        }
        // the big button: green Start, amber Stop, pressed in on click
        let br = Rect::from_min_size(
            egui::pos2(rect.max.x - 24.0 - 200.0, rect.max.y - 24.0 - 52.0),
            Vec2::new(200.0, 52.0),
        );
        let resp = ui.interact(br, ui.scope_id().with("play"), Sense::click());
        let (face, lip, label) = if active {
            (AMBER, Color32::from_rgb(0x8F, 0x5F, 0x10), "Stop")
        } else {
            (GREEN, Color32::from_rgb(0x2A, 0x62, 0x3C), "Start")
        };
        let down = if resp.is_pointer_button_down_on() {
            3.0
        } else {
            0.0
        };
        let f = br.translate(Vec2::new(0.0, down));
        p.rect_filled(br.translate(Vec2::new(0.0, 4.0)), 14, lip);
        p.rect_filled(
            f,
            14,
            if resp.hovered() {
                face.gamma_multiply(1.1)
            } else {
                face
            },
        );
        p.text(
            f.center(),
            Align2::CENTER_CENTER,
            label,
            FontId::proportional(20.0),
            SIDE_TEXT,
        );
        if resp.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        if resp.clicked() {
            if self.emu.is_emulating() {
                self.emu.stop()
            } else {
                self.emu.start()
            }
        }
    }

    fn tiles(&mut self, ui: &mut egui::Ui) {
        let w = (ui.available_width() - 36.0) / 4.0;
        let p = &self.profile;
        let sticks = format!(
            "Deadzone {:.0}% · Curve {:.1}",
            p.left_deadzone * 100.0,
            p.curve
        );
        let flags = format!(
            "Invert Y {} · Swap {}",
            if p.invert_left_y && p.invert_right_y {
                "on"
            } else {
                "off"
            },
            if p.swap_sticks { "on" } else { "off" }
        );
        let remaps = p.remap.as_ref().map_or(0, |m| m.len());
        let maps = if remaps == 0 {
            "No remaps".to_string()
        } else {
            format!("{remaps} remapped")
        };
        let bar = match p.lightbar_mode {
            LightbarMode::Static => p
                .lightbar
                .clone()
                .unwrap_or_else(|| "Pad's own colour".into()),
            LightbarMode::Battery => "Battery level".into(),
            LightbarMode::Charging => "Charging status".into(),
        };
        let leds = match p.player_mode {
            PlayerLedMode::Leave => "Player LEDs as is",
            PlayerLedMode::Player => "Player LEDs: number",
            PlayerLedMode::Battery => "Player LEDs: battery",
            PlayerLedMode::Charging => "Player LEDs: charging",
        };
        let hide = if !self.hide.installed {
            "HidHide missing"
        } else if self.hide.on {
            "Real pad hidden"
        } else {
            "Real pad visible"
        };
        let startup = format!(
            "Start with Windows: {}",
            if self.startup_on { "on" } else { "off" }
        );
        let mut go = None;
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = 12.0;
            if tile(ui, w, RED, "Sticks", &sticks, &flags, "Change") {
                go = Some(Tab::Sticks);
            }
            if tile(
                ui,
                w,
                AMBER,
                "Buttons",
                &maps,
                "Pick a button, choose what it acts as",
                "Change",
            ) {
                go = Some(Tab::Buttons);
            }
            if tile(ui, w, GREEN, "Lightbar", &bar, leds, "Change") {
                go = Some(Tab::Lightbar);
            }
            if tile(ui, w, ACCENT, "System", hide, &startup, "Manage") {
                go = Some(Tab::Settings);
            }
        });
        if let Some(t) = go {
            self.tab = t;
        }
        ui.add_space(6.0);
        ui.label(
            RichText::new("Changes reach the pad instantly and are saved when you let go.")
                .size(12.0)
                .color(DIM),
        );
    }
}

/// A coloured-top summary card; true when clicked.
fn tile(
    ui: &mut egui::Ui,
    w: f32,
    top: Color32,
    title: &str,
    big: &str,
    small: &str,
    link: &str,
) -> bool {
    let (r, resp) = ui.allocate_exact_size(Vec2::new(w, 150.0), Sense::click());
    let p = ui.painter();
    p.rect_filled(r, 12, if resp.hovered() { RAISED } else { CARD });
    p.rect_filled(
        Rect::from_min_size(r.min, Vec2::new(w, 5.0)),
        CornerRadius {
            nw: 12,
            ne: 12,
            sw: 0,
            se: 0,
        },
        top,
    );
    p.text(
        r.min + Vec2::new(14.0, 18.0),
        Align2::LEFT_TOP,
        title,
        FontId::proportional(13.0),
        DIM,
    );
    let g = p.layout(big.to_string(), FontId::proportional(15.0), TEXT, w - 28.0);
    let h = g.size().y;
    p.galley(r.min + Vec2::new(14.0, 38.0), g, TEXT);
    let g = p.layout(
        small.to_string(),
        FontId::proportional(12.5),
        SOFT,
        w - 28.0,
    );
    p.galley(r.min + Vec2::new(14.0, 46.0 + h), g, SOFT);
    p.text(
        egui::pos2(r.min.x + 14.0, r.max.y - 20.0),
        Align2::LEFT_CENTER,
        format!("{link} ›"),
        FontId::proportional(13.0),
        top,
    );
    if resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    resp.clicked()
}

/// The hero backdrop: sky gradient, sun, mountain, two hills. Hand-drawn so it needs no image file.
fn paint_scene(p: &egui::Painter, r: Rect) {
    const TOP: Color32 = Color32::from_rgb(0x24, 0x55, 0x80);
    const HORIZON: Color32 = Color32::from_rgb(0x7F, 0xB0, 0xC4);
    const FAR: Color32 = Color32::from_rgb(0x4F, 0x7B, 0x78);
    const GROUND: Color32 = Color32::from_rgb(0x1B, 0x3B, 0x37);
    let (x0, x1) = (r.min.x, r.max.x);
    let (top, hz, bot) = (r.min.y + 16.0, r.min.y + r.height() * 0.55, r.max.y - 16.0);
    p.rect_filled(r, 16, GROUND);
    p.rect_filled(
        Rect::from_min_max(r.min, egui::pos2(x1, top + 1.0)),
        CornerRadius {
            nw: 16,
            ne: 16,
            sw: 0,
            se: 0,
        },
        TOP,
    );
    let mut sky = egui::Mesh::default();
    for (x, y, c) in [
        (x0, top, TOP),
        (x1, top, TOP),
        (x1, hz, HORIZON),
        (x0, hz, HORIZON),
    ] {
        sky.colored_vertex(egui::pos2(x, y), c);
    }
    sky.add_triangle(0, 1, 2);
    sky.add_triangle(0, 2, 3);
    p.add(sky);
    let sun = egui::pos2(x1 - 70.0, r.min.y + 60.0);
    p.circle_filled(sun, 40.0, Color32::from_white_alpha(22));
    p.circle_filled(sun, 22.0, Color32::from_rgb(0xFA, 0xEE, 0xC6));
    p.add(Shape::convex_polygon(
        vec![
            egui::pos2(x1 - 330.0, hz + 20.0),
            egui::pos2(x1 - 250.0, hz - 60.0),
            egui::pos2(x1 - 170.0, hz + 20.0),
        ],
        Color32::from_rgb(0x5C, 0x5E, 0x80),
        Stroke::NONE,
    ));
    hill(p, (x0, x1), (hz + 15.0, 12.0, 0.0), bot, FAR);
    hill(p, (x0, x1), (hz + 55.0, 18.0, 2.0), bot, GROUND);
}

/// A filled sine hill: `(base, amplitude, phase)` down to `bot`.
fn hill(
    p: &egui::Painter,
    (x0, x1): (f32, f32),
    (base, amp, phase): (f32, f32, f32),
    bot: f32,
    col: Color32,
) {
    const N: u32 = 40;
    let mut m = egui::Mesh::default();
    for i in 0..=N {
        let t = i as f32 / N as f32;
        let x = x0 + (x1 - x0) * t;
        m.colored_vertex(egui::pos2(x, base + amp * (t * 6.0 + phase).sin()), col);
        m.colored_vertex(egui::pos2(x, bot), col);
    }
    for i in 0..N {
        let a = 2 * i;
        m.add_triangle(a, a + 1, a + 2);
        m.add_triangle(a + 1, a + 3, a + 2);
    }
    p.add(m);
}

fn card(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::new()
        .fill(CARD)
        .stroke(Stroke::new(1.0, BORDER))
        .corner_radius(14)
        .inner_margin(Margin::same(20))
        .show(ui, |ui| ui.vertical(add));
}

// ---------- dashboard ----------

impl App {
    fn dashboard(&mut self, ui: &mut egui::Ui) {
        let status = self
            .status
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        let active = matches!(
            status.state,
            EmuState::Running | EmuState::Waiting | EmuState::Error | EmuState::DriverMissing
        ); // it keeps retrying until stopped
        self.hero(ui, &status, active);
        ui.add_space(12.0);
        self.live_card(ui);
        ui.add_space(12.0);
        self.tiles(ui);
    }

    fn settings(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.horizontal(|ui| {
            ui.add_enabled_ui(self.hide.installed && !self.hide.busy, |ui| {
                let mut on = self.hide.on;
                if toggle(ui, &mut on).clicked() {
                    self.hide.busy = true;
                    self.hide.text = "Waiting for admin approval (UAC)...".into();
                    let (tx, ctx) = (self.tx.clone(), ctx.clone());
                    std::thread::spawn(move || {
                        // whitelist the process that reads the pad: this exe
                        let exe = std::env::current_exe()
                            .map(|p| p.display().to_string())
                            .unwrap_or_default();
                        let ok = hidhide::set_hidden(on, &exe);
                        let installed = hidhide::is_installed();
                        let _ = tx.send(Msg::HideDone {
                            ok,
                            installed,
                            on: installed && hidhide::is_cloak_on(),
                        });
                        ctx.request_repaint();
                    });
                }
            });
            ui.vertical(|ui| {
                ui.label(RichText::new("Hide real pad").size(14.0).color(TEXT));
                let text = if self.hide.text.is_empty() {
                    hide_text(self.hide.installed, self.hide.on)
                } else {
                    self.hide.text.clone()
                };
                ui.add(egui::Label::new(RichText::new(text).size(12.0).color(DIM)).wrap());
            });
        });
        if !self.hide.installed && !self.hide.busy && ui.button("Install HidHide").clicked() {
            self.hide.busy = true;
            self.hide.text = "Installing HidHide (approve the prompts)...".into();
            let (tx, ctx) = (self.tx.clone(), ctx.clone());
            std::thread::spawn(move || {
                let ok = hidhide::install(hidhide::HIDHIDE_PACKAGE);
                let installed = hidhide::is_installed();
                let _ = tx.send(Msg::HidHideInstalled {
                    ok,
                    installed,
                    on: installed && hidhide::is_cloak_on(),
                });
                ctx.request_repaint();
            });
        }
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            let mut on = self.startup_on;
            if toggle(ui, &mut on).clicked() {
                set_startup(on);
                self.startup_on = startup_enabled();
            }
            ui.label(
                RichText::new("Start with Windows (in tray)")
                    .size(14.0)
                    .color(TEXT),
            );
        });
    }

    fn live_card(&mut self, ui: &mut egui::Ui) {
        let snap = self.emu.latest();
        let emulating = self.emu.is_emulating() && snap.is_some();
        let (ds, x) = match &snap {
            Some(s) => (Some(s.ds), Some(s.x360)),
            None => (None, None),
        };
        let ds_buttons = ds.map_or(0, |d| d.buttons);
        let x_buttons = x.map_or(0, |x| x.buttons);
        let (lx, ly, rx, ry) = ds.map_or((128, 128, 128, 128), |d| (d.lx, d.ly, d.rx, d.ry));
        egui::Frame::new()
            .fill(Color32::from_rgb(0x23, 0x20, 0x2C))
            .corner_radius(14)
            .inner_margin(Margin::same(16))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Live input").size(16.0).color(SIDE_TEXT));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let badge = if emulating { "Translating" } else { "Idle" };
                        ui.label(RichText::new(badge).size(13.0).color(SIDE_DIM));
                    });
                });
                ui.columns(2, |cols| {
                    cols[0].vertical_centered(|ui| {
                        padview::show(
                            ui,
                            PAD_SCALE,
                            &PadInput {
                                xbox: false,
                                pressed: &|k| {
                                    DS_BUTTONS
                                        .iter()
                                        .any(|(n, b)| *n == k && ds_buttons & b.bit() != 0)
                                },
                                l2: ds.map_or(0, |d| d.l2),
                                r2: ds.map_or(0, |d| d.r2),
                                left: (lx, ly),
                                right: (rx, ry),
                            },
                        );
                        ui.label(RichText::new("PS5 DualSense").size(14.0).color(SIDE_DIM));
                    });
                    cols[1].vertical_centered(|ui| {
                        padview::show(
                            ui,
                            PAD_SCALE,
                            &PadInput {
                                xbox: true,
                                pressed: &|k| {
                                    X360_BUTTONS
                                        .iter()
                                        .any(|(n, b)| *n == k && x_buttons & b.bits() != 0)
                                },
                                l2: x.map_or(0, |x| x.left_trigger),
                                r2: x.map_or(0, |x| x.right_trigger),
                                left: (lx, ly),
                                right: (rx, ry),
                            },
                        );
                        ui.label(RichText::new("Xbox 360 pad").size(14.0).color(SIDE_DIM));
                    });
                });
            });
    }
}

fn hide_text(installed: bool, on: bool) -> String {
    if !installed {
        "HidHide is not installed. It is needed to hide the real pad from games.".into()
    } else if on {
        "Real pad is hidden from games.".into()
    } else {
        "Real pad is visible to games (double input possible).".into()
    }
}

/// iOS-style switch; the caller reads the new value from `on` after a click.
fn toggle(ui: &mut egui::Ui, on: &mut bool) -> egui::Response {
    let (rect, mut resp) = ui.allocate_exact_size(Vec2::new(40.0, 20.0), Sense::click());
    if resp.clicked() {
        *on = !*on;
        resp.mark_changed();
    }
    let t = ui.ctx().animate_bool(resp.id, *on);
    let fill = if *on {
        ACCENT.gamma_multiply(0.35 + 0.65 * t)
    } else {
        TILE_OFF
    };
    ui.painter().rect(
        rect,
        CornerRadius::same(10),
        fill,
        Stroke::new(1.0, if *on { ACCENT } else { DIM }),
        StrokeKind::Inside,
    );
    let x = egui::lerp((rect.left() + 10.0)..=(rect.right() - 10.0), t);
    ui.painter()
        .circle_filled(egui::pos2(x, rect.center().y), 7.0, Color32::WHITE);
    resp
}

// ---------- tuning tabs ----------

impl App {
    fn sticks(&mut self, ui: &mut egui::Ui) {
        card(ui, |ui| {
            ui.set_width(ui.available_width().max(0.0));
            let mut changed = false;
            ui.label(RichText::new("Stick deadzone").size(14.0).color(DIM));
            let mut dz = self.profile.left_deadzone;
            if ui
                .add(egui::Slider::new(&mut dz, 0.0..=0.4).step_by(0.005))
                .changed()
            {
                (self.profile.left_deadzone, self.profile.right_deadzone) = (dz, dz);
                changed = true;
            }
            ui.label(
                RichText::new("Stick curve (1 = linear)")
                    .size(14.0)
                    .color(DIM),
            );
            changed |= ui
                .add(egui::Slider::new(&mut self.profile.curve, 1.0..=3.0).step_by(0.01))
                .changed();
            ui.label(RichText::new("Trigger threshold").size(14.0).color(DIM));
            changed |= ui
                .add(egui::Slider::new(
                    &mut self.profile.trigger_threshold,
                    0..=100,
                ))
                .changed();
            let mut inv = self.profile.invert_left_y && self.profile.invert_right_y;
            if ui.checkbox(&mut inv, "Invert Y (both sticks)").changed() {
                (self.profile.invert_left_y, self.profile.invert_right_y) = (inv, inv);
                changed = true;
            }
            changed |= ui
                .checkbox(&mut self.profile.swap_sticks, "Swap sticks")
                .changed();
            if ui.button("Reset all tuning").clicked() {
                self.profile = MapProfile::default();
                self.hex_text.clear();
                changed = true;
            }
            if changed {
                self.apply();
            }
        });
    }

    fn buttons(&mut self, ui: &mut egui::Ui) {
        card(ui, |ui| {
            ui.set_width(ui.available_width().max(0.0));
            ui.label(
                RichText::new("1. Pick a DualSense button   (amber = remapped)")
                    .size(14.0)
                    .color(DIM),
            );
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = Vec2::splat(8.0);
                for (name, b) in DS_BUTTONS {
                    let over = self.profile.remap.as_ref().and_then(|r| r.get(&b)).copied();
                    let target = match over {
                        None => {
                            mapper::default_for(b).map_or("-".to_string(), |t| format!("{t:?}"))
                        }
                        Some(X360Button::None) => "off".to_string(),
                        Some(t) => format!("{t:?}"),
                    };
                    let (rect, resp) =
                        ui.allocate_exact_size(Vec2::new(104.0, 48.0), Sense::click());
                    let p = ui.painter();
                    p.rect(
                        rect,
                        CornerRadius::same(6),
                        if resp.hovered() { HOVER } else { RAISED },
                        Stroke::new(
                            1.0,
                            if b == self.remap_sel {
                                ACCENT
                            } else {
                                TILE_OFF
                            },
                        ),
                        StrokeKind::Inside,
                    );
                    p.text(
                        rect.center() - Vec2::new(0.0, 8.0),
                        egui::Align2::CENTER_CENTER,
                        name,
                        FontId::proportional(14.0),
                        TEXT,
                    );
                    p.text(
                        rect.center() + Vec2::new(0.0, 10.0),
                        egui::Align2::CENTER_CENTER,
                        format!("→ {target}"),
                        FontId::proportional(12.0),
                        if over.is_none() { DIM } else { AMBER },
                    );
                    if resp.clicked() {
                        self.remap_sel = b;
                    }
                }
            });
            ui.add_space(12.0);
            let sel = self.remap_sel;
            ui.label(
                RichText::new(format!(
                    "2. {} acts as:",
                    DS_BUTTONS
                        .iter()
                        .find(|(_, b)| *b == sel)
                        .map_or("", |(n, _)| n)
                ))
                .size(14.0)
                .color(TEXT),
            );
            let cur = self
                .profile
                .remap
                .as_ref()
                .and_then(|r| r.get(&sel))
                .copied();
            let def = mapper::default_for(sel);
            let mut pick: Option<Option<X360Button>> = None; // Some(None) = back to the default
            ui.horizontal_wrapped(|ui| {
                let default_label =
                    def.map_or("(unmapped)".to_string(), |d| format!("(default: {d:?})"));
                if ui.radio(cur.is_none(), default_label).clicked() {
                    pick = Some(None);
                }
                if ui.radio(cur == Some(X360Button::None), "Off").clicked() {
                    pick = Some(Some(X360Button::None));
                }
                for (name, t) in X360_BUTTONS {
                    if ui.radio(cur == Some(t), name).clicked() {
                        pick = Some(Some(t));
                    }
                }
            });
            if let Some(choice) = pick {
                let mut map: BTreeMap<DsButton, X360Button> =
                    self.profile.remap.clone().unwrap_or_default();
                match choice {
                    None => {
                        map.remove(&sel);
                    }
                    Some(t) => {
                        map.insert(sel, t);
                    }
                }
                self.profile.remap = (!map.is_empty()).then_some(map);
                self.apply();
            }
        });
    }

    fn lightbar(&mut self, ui: &mut egui::Ui) {
        card(ui, |ui| {
            ui.set_width(ui.available_width().max(0.0));
            let mut changed = false;
            ui.label(RichText::new("Lightbar shows").size(14.0).color(TEXT));
            ui.horizontal_wrapped(|ui| {
                changed |= ui
                    .radio_value(
                        &mut self.profile.lightbar_mode,
                        LightbarMode::Static,
                        "Static colour",
                    )
                    .changed();
                changed |= ui
                    .radio_value(
                        &mut self.profile.lightbar_mode,
                        LightbarMode::Battery,
                        "Battery level (green / amber / red)",
                    )
                    .changed();
                changed |= ui
                    .radio_value(
                        &mut self.profile.lightbar_mode,
                        LightbarMode::Charging,
                        "Charging status",
                    )
                    .changed();
            });
            if self.profile.lightbar_mode == LightbarMode::Static {
                ui.label(RichText::new("Presets").size(14.0).color(DIM));
                ui.horizontal_wrapped(|ui| {
                    for (name, hex) in PRESETS {
                        let (rect, resp) =
                            ui.allocate_exact_size(Vec2::splat(40.0), Sense::click());
                        let c = parse_colour(Some(hex))
                            .map_or(Color32::WHITE, |(r, g, b)| Color32::from_rgb(r, g, b));
                        ui.painter()
                            .circle(rect.center(), 19.5, c, Stroke::new(1.0, TILE_OFF));
                        if resp.on_hover_text(name).clicked() {
                            self.profile.lightbar = Some(hex.to_string());
                            self.hex_text = hex.to_string();
                            changed = true;
                        }
                    }
                });
                ui.label(RichText::new("Make your own").size(14.0).color(DIM));
                let (mut r, mut g, mut b) =
                    parse_colour(self.profile.lightbar.as_deref()).unwrap_or((0, 0, 0));
                let (swatch, _) =
                    ui.allocate_exact_size(Vec2::new(ui.available_width(), 30.0), Sense::hover());
                ui.painter().rect(
                    swatch,
                    CornerRadius::same(8),
                    if self.profile.lightbar.is_some() {
                        Color32::from_rgb(r, g, b)
                    } else {
                        Color32::TRANSPARENT
                    },
                    Stroke::new(1.0, TILE_OFF),
                    StrokeKind::Inside,
                );
                let mut moved = false;
                moved |= ui
                    .add(egui::Slider::new(&mut r, 0..=255).text("R"))
                    .changed();
                moved |= ui
                    .add(egui::Slider::new(&mut g, 0..=255).text("G"))
                    .changed();
                moved |= ui
                    .add(egui::Slider::new(&mut b, 0..=255).text("B"))
                    .changed();
                if moved {
                    let hex = format!("#{r:02X}{g:02X}{b:02X}");
                    self.profile.lightbar = Some(hex.clone());
                    self.hex_text = hex;
                    changed = true;
                }
                ui.horizontal(|ui| {
                    if ui
                        .add(
                            egui::TextEdit::singleline(&mut self.hex_text)
                                .hint_text("#RRGGBB")
                                .desired_width(120.0),
                        )
                        .changed()
                    {
                        let hex = self.hex_text.trim().to_string();
                        if hex.is_empty() {
                            self.profile.lightbar = None;
                            changed = true;
                        } else if parse_colour(Some(&hex)).is_some() {
                            self.profile.lightbar = Some(hex); // else still typing
                            changed = true;
                        }
                    }
                    if ui.button("Use pad's own colour").clicked() {
                        self.profile.lightbar = None;
                        self.hex_text.clear();
                        changed = true;
                    }
                });
            }
            ui.add_space(8.0);
            ui.label(RichText::new("Player LEDs show").size(14.0).color(TEXT));
            let before = self.profile.player_mode;
            ui.horizontal_wrapped(|ui| {
                changed |= ui
                    .radio_value(
                        &mut self.profile.player_mode,
                        PlayerLedMode::Leave,
                        "Leave as is",
                    )
                    .changed();
                changed |= ui
                    .radio_value(
                        &mut self.profile.player_mode,
                        PlayerLedMode::Player,
                        "Player number",
                    )
                    .changed();
                changed |= ui
                    .radio_value(
                        &mut self.profile.player_mode,
                        PlayerLedMode::Battery,
                        "Battery level (1-5 LEDs)",
                    )
                    .changed();
                changed |= ui
                    .radio_value(
                        &mut self.profile.player_mode,
                        PlayerLedMode::Charging,
                        "Charging (all lit)",
                    )
                    .changed();
            });
            if self.profile.player_mode != before && self.profile.player_number == 0 {
                self.profile.player_number = 1;
            }
            if self.profile.player_mode == PlayerLedMode::Player {
                ui.horizontal_wrapped(|ui| {
                    for n in 1..=4u8 {
                        changed |= ui
                            .radio_value(&mut self.profile.player_number, n, format!("Player {n}"))
                            .changed();
                    }
                });
            }
            if changed {
                self.apply();
            }
        });
    }
}

// ---------- helpers ----------

fn style(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    // the default fonts lack the shape glyphs of the pad diagram; Windows ships a symbol font
    if let Ok(bytes) = std::fs::read(r"C:\Windows\Fonts\seguisym.ttf") {
        fonts.font_data.insert(
            "symbols".into(),
            Arc::new(egui::FontData::from_owned(bytes)),
        );
        fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .push("symbols".into());
    }
    ctx.set_fonts(fonts);
    let mut v = egui::Visuals::light();
    let tint = Color32::from_rgb(0xB9, 0xCB, 0xE6);
    v.panel_fill = TAN;
    v.window_fill = CREAM;
    v.extreme_bg_color = Color32::from_rgb(0xF5, 0xF1, 0xE6);
    for w in [
        &mut v.widgets.noninteractive,
        &mut v.widgets.inactive,
        &mut v.widgets.hovered,
        &mut v.widgets.active,
        &mut v.widgets.open,
    ] {
        w.fg_stroke.color = TEXT;
        w.corner_radius = CornerRadius::same(8);
        w.expansion = 0.0;
    }
    v.widgets.inactive.bg_fill = RAISED;
    v.widgets.inactive.weak_bg_fill = RAISED;
    v.widgets.inactive.bg_stroke = Stroke::new(1.0, OUTLINE);
    v.widgets.hovered.bg_fill = HOVER;
    v.widgets.hovered.weak_bg_fill = HOVER;
    v.widgets.hovered.bg_stroke = Stroke::new(1.0, DIM);
    v.widgets.active.bg_fill = tint;
    v.widgets.active.weak_bg_fill = tint;
    v.widgets.active.bg_stroke = Stroke::new(1.0, ACCENT);
    v.selection.bg_fill = Color32::from_rgb(0x8F, 0xAD, 0xDA);
    v.slider_trailing_fill = true;
    v.selection.stroke = Stroke::new(1.0, ACCENT);
    ctx.style_mut_of(egui::Theme::Light, |s| {
        s.spacing.button_padding = Vec2::new(14.0, 7.0);
        s.spacing.interact_size.y = 30.0;
    });
    ctx.set_theme(egui::ThemePreference::Light); // the system theme must not pick dark visuals
    ctx.set_visuals_of(egui::Theme::Light, v);
}

fn spawn_hide_state(tx: &Sender<Msg>, ctx: &egui::Context) {
    let (tx, ctx) = (tx.clone(), ctx.clone());
    std::thread::spawn(move || {
        let installed = hidhide::is_installed();
        let _ = tx.send(Msg::HideState {
            installed,
            on: installed && hidhide::is_cloak_on(),
        });
        ctx.request_repaint();
    });
}

fn startup_enabled() -> bool {
    use winreg::{enums::HKEY_CURRENT_USER, RegKey};
    RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey(RUN_KEY)
        .and_then(|k| k.get_value::<String, _>(RUN_VALUE))
        .is_ok()
}

// shortcut: registers the exe where it runs now. Fine for the installed app (stable current\ folder); a standalone
// exe breaks start-at-login if rebuilt or moved.
fn set_startup(on: bool) {
    use winreg::{
        enums::{HKEY_CURRENT_USER, KEY_WRITE},
        RegKey,
    };
    let Ok(key) = RegKey::predef(HKEY_CURRENT_USER).open_subkey_with_flags(RUN_KEY, KEY_WRITE)
    else {
        return;
    };
    if on {
        if let Ok(exe) = std::env::current_exe() {
            let _ = key.set_value(RUN_VALUE, &format!("\"{}\" --tray", exe.display()));
        }
    } else {
        let _ = key.delete_value(RUN_VALUE);
    }
}
