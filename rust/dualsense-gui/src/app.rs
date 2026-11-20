//! The window: header, Dashboard / Sticks / Buttons / Lightbar tabs. Mirrors the Avalonia GUI it replaces.

use crate::padview::{self, PadInput};
use dualsense_core::output::parse_colour;
use dualsense_core::profile::{DsButton, LightbarMode, MapProfile, PlayerLedMode, X360Button};
use dualsense_core::{mapper, state::ChargeState};
use dualsense_device::hidhide;
use dualsense_device::service::{EmuState, EmuStatus, EmulatorService, Transport};
use dualsense_device::status_file;
use eframe::egui::{
    self, Color32, CornerRadius, FontId, Margin, RichText, Sense, Stroke, StrokeKind, Vec2,
};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering::Relaxed};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

const BG: Color32 = Color32::from_rgb(0x0B, 0x0D, 0x14);
const CARD: Color32 = Color32::from_rgb(0x11, 0x14, 0x1C);
const RAISED: Color32 = Color32::from_rgb(0x1A, 0x1E, 0x28);
const HOVER: Color32 = Color32::from_rgb(0x23, 0x28, 0x36);
const BORDER: Color32 = Color32::from_rgb(0x1D, 0x20, 0x29);
const TILE_OFF: Color32 = Color32::from_rgb(0x26, 0x2B, 0x38);
const TEXT: Color32 = Color32::from_rgb(0xEE, 0xF0, 0xF5);
const SOFT: Color32 = Color32::from_rgb(0xA3, 0xA9, 0xB8);
const DIM: Color32 = Color32::from_rgb(0x7A, 0x82, 0x96);
const ACCENT: Color32 = Color32::from_rgb(0x4F, 0xB7, 0xD6);
const AMBER: Color32 = Color32::from_rgb(0xE0, 0xA2, 0x3F);
const GREEN: Color32 = Color32::from_rgb(0x4C, 0xC3, 0x8A);
const RED: Color32 = Color32::from_rgb(0xE5, 0x48, 0x4D);
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
    dirty: bool, // profile changed, not yet saved
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
        egui::Frame::new()
            .fill(BG)
            .inner_margin(Margin::same(24))
            .show(ui, |ui| {
                self.header(ui);
                ui.add_space(16.0);
                ui.horizontal(|ui| {
                    for (tab, name) in [
                        (Tab::Dashboard, "Dashboard"),
                        (Tab::Sticks, "Sticks"),
                        (Tab::Buttons, "Buttons"),
                        (Tab::Lightbar, "Lightbar"),
                    ] {
                        let on = self.tab == tab;
                        let text =
                            RichText::new(name)
                                .size(15.0)
                                .color(if on { TEXT } else { DIM });
                        if ui
                            .add(
                                egui::Button::new(text)
                                    .fill(if on { RAISED } else { Color32::TRANSPARENT })
                                    .stroke(Stroke::NONE)
                                    .min_size(Vec2::new(96.0, 32.0)),
                            )
                            .clicked()
                        {
                            self.tab = tab;
                        }
                    }
                });
                ui.add_space(8.0);
                match self.tab {
                    Tab::Dashboard => self.dashboard(ui, &ctx),
                    Tab::Sticks => self.sticks(ui),
                    Tab::Buttons => self.buttons(ui),
                    Tab::Lightbar => self.lightbar(ui),
                }
            });
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        BG.to_normalized_gamma_f32()
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.quit_status.store(true, Relaxed);
        self.emu.stop();
        if let Some(t) = self.status_thread.take() {
            let _ = t.join();
        }
    }
}

// ---------- header ----------

impl App {
    fn header(&mut self, ui: &mut egui::Ui) {
        let snap = self.emu.latest();
        let transport = self.emu.transport();
        let icon = self.icon.clone();
        ui.horizontal(|ui| {
            ui.add(
                egui::Image::new(&icon)
                    .fit_to_exact_size(Vec2::splat(40.0))
                    .corner_radius(10),
            );
            ui.add_space(8.0);
            ui.vertical(|ui| {
                ui.label(RichText::new("DualSense Emulator").size(24.0).color(TEXT));
                ui.label(
                    RichText::new("Your DualSense as a virtual Xbox 360 pad")
                        .size(14.0)
                        .color(DIM),
                );
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // right to left: battery pill first (rightmost), then the connection pill
                pill(ui, |ui| {
                    let (rect, _) = ui.allocate_exact_size(Vec2::new(32.0, 14.0), Sense::hover());
                    let p = ui.painter();
                    let body = egui::Rect::from_min_size(rect.min, Vec2::new(28.0, 14.0));
                    p.rect_stroke(
                        body,
                        CornerRadius::same(3),
                        Stroke::new(1.0, DIM),
                        StrokeKind::Inside,
                    );
                    p.rect_filled(
                        egui::Rect::from_min_size(
                            egui::pos2(rect.max.x - 3.0, rect.min.y + 4.0),
                            Vec2::new(3.0, 6.0),
                        ),
                        CornerRadius::same(1),
                        DIM,
                    );
                    let (text, charge) = match snap
                        .as_ref()
                        .map(|s| s.ds)
                        .filter(|d| d.battery_percent >= 0)
                    {
                        Some(d) => {
                            let colour = if d.charge == ChargeState::Charging {
                                ACCENT
                            } else if d.battery_percent < 20 {
                                RED
                            } else if d.battery_percent < 40 {
                                AMBER
                            } else {
                                GREEN
                            };
                            let w = 24.0 * d.battery_percent as f32 / 100.0;
                            p.rect_filled(
                                egui::Rect::from_min_size(
                                    body.min + Vec2::splat(2.0),
                                    Vec2::new(w, 10.0),
                                ),
                                CornerRadius::same(2),
                                colour,
                            );
                            let c = match d.charge {
                                ChargeState::Charging => "Charging",
                                ChargeState::Full => "Full",
                                _ => "",
                            };
                            (format!("{}%", d.battery_percent), c)
                        }
                        None => ("--".to_string(), ""),
                    };
                    ui.label(RichText::new(text).size(14.0).color(TEXT));
                    ui.label(RichText::new(charge).size(14.0).color(SOFT));
                });
                pill(ui, |ui| {
                    let (dot, _) = ui.allocate_exact_size(Vec2::splat(8.0), Sense::hover());
                    ui.painter().circle_filled(
                        dot.center(),
                        4.0,
                        if transport == Transport::None {
                            DIM
                        } else {
                            GREEN
                        },
                    );
                    let t = match transport {
                        Transport::Bluetooth => "Bluetooth",
                        Transport::Usb => "USB cable",
                        Transport::None => "Not connected",
                    };
                    ui.label(RichText::new(t).size(14.0).color(TEXT));
                });
            });
        });
    }
}

fn pill(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::new()
        .fill(CARD)
        .stroke(Stroke::new(1.0, BORDER))
        .corner_radius(8)
        .inner_margin(Margin::symmetric(12, 8))
        .show(ui, |ui| {
            ui.horizontal(add);
        });
}

fn card(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::new()
        .fill(CARD)
        .stroke(Stroke::new(1.0, BORDER))
        .corner_radius(14)
        .inner_margin(Margin::same(20))
        .shadow(egui::Shadow {
            offset: [0, 4],
            blur: 16,
            spread: 0,
            color: Color32::from_black_alpha(90),
        })
        .show(ui, |ui| ui.vertical(add));
}

// ---------- dashboard ----------

impl App {
    fn dashboard(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let status = self
            .status
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        let active = matches!(
            status.state,
            EmuState::Running | EmuState::Waiting | EmuState::Error | EmuState::DriverMissing
        ); // it keeps retrying until stopped
        ui.horizontal_top(|ui| {
            ui.allocate_ui(Vec2::new(480.0, 0.0), |ui| {
                ui.set_width(480.0);
                card(ui, |ui| {
                    ui.set_width(440.0);
                    ui.label(
                        RichText::new("Controller management")
                            .size(20.0)
                            .color(TEXT),
                    );
                    ui.add_space(12.0);
                    ui.horizontal(|ui| {
                        if knob(ui, active)
                            .on_hover_text("Click to turn emulation on or off")
                            .clicked()
                        {
                            if status.state == EmuState::Stopped {
                                self.emu.start()
                            } else {
                                self.emu.stop()
                            }
                        }
                        ui.vertical(|ui| {
                            let w = ui.available_width();
                            if mode_button(
                                ui,
                                "Emulate Xbox gamepad",
                                w,
                                if active { Some(ACCENT) } else { None },
                            )
                            .clicked()
                            {
                                self.emu.start();
                            }
                            ui.add_space(8.0);
                            if mode_button(
                                ui,
                                "Passthrough (off)",
                                w,
                                if active { None } else { Some(AMBER) },
                            )
                            .clicked()
                            {
                                self.emu.stop();
                            }
                        });
                    });
                    ui.add_space(16.0);
                    self.status_panel(ui, &status, active);
                    ui.add_space(16.0);
                    self.settings(ui, ctx);
                });
            });
            self.live_card(ui);
        });
    }

    fn status_panel(&mut self, ui: &mut egui::Ui, status: &EmuStatus, active: bool) {
        let (colour, word) = match status.state {
            EmuState::Running => (GREEN, "Emulating"),
            EmuState::Waiting => (AMBER, "Waiting"),
            EmuState::Error => (RED, "Error"),
            EmuState::DriverMissing => (RED, "No driver"),
            EmuState::Stopped => (DIM, "Off"),
        };
        egui::Frame::new()
            .fill(RAISED)
            .stroke(Stroke::new(1.0, BORDER))
            .corner_radius(10)
            .inner_margin(Margin::same(16))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.vertical_centered(|ui| {
                    ui.horizontal(|ui| {
                        ui.add_space(ui.available_width() / 2.0 - 70.0);
                        let (dot, _) = ui.allocate_exact_size(Vec2::splat(12.0), Sense::hover());
                        ui.painter().circle_filled(dot.center(), 6.0, colour);
                        ui.label(RichText::new("Status  ").size(18.0).color(SOFT));
                        ui.label(RichText::new(word).size(18.0).color(colour));
                    });
                    let note = self.vigem_note.clone().unwrap_or_else(|| {
                        if status.state == EmuState::Running {
                            "games see an Xbox 360 pad".into()
                        } else {
                            status.message.clone()
                        }
                    });
                    ui.label(RichText::new(note).size(14.0).color(DIM));
                    if status.state == EmuState::DriverMissing
                        && ui.button("Install ViGEmBus driver").clicked()
                    {
                        self.vigem_note =
                            Some("Installing ViGEmBus (approve the prompt)...".into());
                        let (tx, ctx) = (self.tx.clone(), ui.ctx().clone());
                        std::thread::spawn(move || {
                            let _ = tx.send(Msg::VigemInstalled(hidhide::install(
                                hidhide::VIGEM_BUS_PACKAGE,
                            )));
                            ctx.request_repaint();
                        });
                    }
                    ui.add_space(8.0);
                    let label = if active {
                        "Stop emulation"
                    } else {
                        "Start emulation"
                    };
                    let go = egui::Button::new(RichText::new(label).size(15.0).color(BG))
                        .fill(TEXT)
                        .corner_radius(10)
                        .min_size(Vec2::new(0.0, 40.0));
                    if ui.add(go).clicked() {
                        if self.emu.is_emulating() {
                            self.emu.stop()
                        } else {
                            self.emu.start()
                        }
                    }
                });
            });
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
        card(ui, |ui| {
            ui.set_width(ui.available_width().max(0.0));
            ui.horizontal(|ui| {
                ui.label(RichText::new("Live input").size(16.0).color(TEXT));
                ui.label(
                    RichText::new("   Active map translation")
                        .size(13.0)
                        .color(DIM),
                );
            });
            let badge = if emulating { "Translating" } else { "Idle" };
            ui.vertical_centered(|ui| {
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
                ui.label(RichText::new("PS5 DualSense").size(14.0).color(SOFT));
                mono_badge(ui, badge);
                let (line, _) = ui.allocate_exact_size(Vec2::new(1.0, 16.0), Sense::hover());
                ui.painter().rect_filled(line, 0, DIM);
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
                mono_badge(ui, badge);
                ui.label(RichText::new("Xbox 360 pad").size(14.0).color(SOFT));
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

fn mono_badge(ui: &mut egui::Ui, text: &str) {
    egui::Frame::new()
        .fill(RAISED)
        .stroke(Stroke::new(1.0, BORDER))
        .corner_radius(6)
        .inner_margin(Margin::symmetric(8, 4))
        .show(ui, |ui| {
            ui.label(
                RichText::new(text)
                    .font(FontId::monospace(12.0))
                    .color(SOFT),
            );
        });
}

fn mode_button(
    ui: &mut egui::Ui,
    text: &str,
    width: f32,
    outline: Option<Color32>,
) -> egui::Response {
    let b = egui::Button::new(RichText::new(text).size(15.0).color(if outline.is_some() {
        TEXT
    } else {
        SOFT
    }))
    .fill(if outline.is_some() { HOVER } else { RAISED })
    .stroke(Stroke::new(1.0, outline.unwrap_or(BORDER)))
    .corner_radius(10)
    .min_size(Vec2::new(width, 56.0));
    ui.add(b)
}

/// The round on/off dial: the tick points at the end of the arc it is on.
fn knob(ui: &mut egui::Ui, active: bool) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(128.0), Sense::click());
    let (p, c) = (ui.painter(), rect.center());
    let colour = if active { ACCENT } else { AMBER };
    p.circle(c, 64.0, BG, Stroke::new(1.0, BORDER));
    let arc: Vec<egui::Pos2> = (135..=405)
        .step_by(5)
        .map(|d| {
            let a = (d as f32).to_radians();
            c + Vec2::new(a.cos(), a.sin()) * 44.0
        })
        .collect();
    p.add(egui::Shape::line(arc, Stroke::new(6.0, colour)));
    p.circle(c, 32.0, RAISED, Stroke::new(1.0, BORDER));
    let a = (if active { 45.0f32 } else { 135.0 }).to_radians();
    let dir = Vec2::new(a.cos(), a.sin());
    p.line_segment([c + dir * 12.0, c + dir * 24.0], Stroke::new(4.0, colour));
    if resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    resp
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
    ui.painter().circle_filled(
        egui::pos2(x, rect.center().y),
        7.0,
        if *on { TEXT } else { SOFT },
    );
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
    let mut v = egui::Visuals::dark();
    v.panel_fill = BG;
    v.window_fill = BG;
    v.extreme_bg_color = BG;
    v.widgets.noninteractive.fg_stroke.color = TEXT;
    v.widgets.inactive.fg_stroke.color = TEXT;
    v.widgets.inactive.bg_fill = RAISED;
    v.widgets.inactive.weak_bg_fill = RAISED;
    v.widgets.hovered.weak_bg_fill = HOVER;
    v.widgets.hovered.bg_fill = HOVER;
    v.selection.bg_fill = ACCENT.gamma_multiply(0.5);
    v.selection.stroke = Stroke::new(1.0, ACCENT);
    // one look for every widget: rounded, hairline border, accent when pressed, no hover jitter
    v.widgets.inactive.bg_stroke = Stroke::new(1.0, BORDER);
    v.widgets.hovered.bg_stroke = Stroke::new(1.0, TILE_OFF);
    v.widgets.active.bg_fill = ACCENT.gamma_multiply(0.35);
    v.widgets.active.weak_bg_fill = ACCENT.gamma_multiply(0.35);
    v.widgets.active.bg_stroke = Stroke::new(1.0, ACCENT);
    for w in [
        &mut v.widgets.noninteractive,
        &mut v.widgets.inactive,
        &mut v.widgets.hovered,
        &mut v.widgets.active,
        &mut v.widgets.open,
    ] {
        w.corner_radius = CornerRadius::same(8);
        w.expansion = 0.0;
    }
    ctx.style_mut_of(egui::Theme::Dark, |s| {
        s.spacing.button_padding = Vec2::new(14.0, 7.0);
        s.spacing.interact_size.y = 30.0;
    });
    ctx.set_theme(egui::ThemePreference::Dark); // the system theme must not pick light visuals
    ctx.set_visuals_of(egui::Theme::Dark, v);
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
