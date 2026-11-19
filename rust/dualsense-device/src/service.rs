//! Reads a DualSense over HID for as long as the service lives (battery, lightbar) and, while `emulating`,
//! mirrors it to a virtual Xbox 360 pad through ViGEmBus.

use dualsense_core::mapper::{map, X360State};
use dualsense_core::output::{build, resolve, resolve_leds, Rgb};
use dualsense_core::parser::try_parse;
use dualsense_core::profile::MapProfile;
use dualsense_core::state::DualSenseState;
use hidapi::{BusType, HidApi, HidDevice};
use std::sync::atomic::{AtomicBool, AtomicU16, Ordering::Relaxed};
use std::sync::{Arc, Mutex, MutexGuard, RwLock};
use std::thread::JoinHandle;
use std::time::Duration;
use vigem_client::{Client, Error as VigemError, TargetId, XButtons, XGamepad, Xbox360Wired};

const SONY_VID: u16 = 0x054C;
const DUALSENSE_PIDS: [u16; 2] = [0x0CE6, 0x0DF2]; // DualSense, DualSense Edge
const USB_INPUT_LENGTH: usize = 64;
const BT_INPUT_LENGTH: usize = 78;
const BT_ENABLE_FEATURE_ID: u8 = 0x05; // reading it switches Bluetooth to the full 0x31 report
const BT_FEATURE_LENGTH: usize = 41;
const READ_TIMEOUT_MS: i32 = 500; // lets stop take effect even if the pad goes quiet

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EmuState {
    Stopped,
    Waiting,
    Running,
    Error,
    DriverMissing,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Transport {
    None,
    Usb,
    Bluetooth,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EmuStatus {
    pub state: EmuState,
    pub message: String,
}

#[derive(Clone, Copy, Debug)]
pub struct Snapshot {
    pub ds: DualSenseState,
    pub x360: X360State,
}

enum RunError {
    DriverMissing,
    /// The pad (HID) failed; the virtual pad can stay.
    Hid(String),
    /// Anything else: the virtual pad is rebuilt on the next try.
    Other(String),
}

type Look = (Option<Rgb>, Option<u8>);

struct Shared {
    profile: RwLock<MapProfile>,
    latest: Mutex<Option<Snapshot>>,
    transport: Mutex<Transport>,
    last_status: Mutex<Option<EmuStatus>>,
    on_status: Box<dyn Fn(&EmuStatus) + Send + Sync>,
    emulating: AtomicBool,
    stop: AtomicBool,
    /// Set by `refresh_output` and by games' rumble: the reader thread re-sends the output report.
    resend: AtomicBool,
    /// Games' rumble, large motor in the high byte, forwarded to the DualSense.
    rumble: AtomicU16,
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

impl Shared {
    fn report(&self, state: EmuState, message: &str) {
        let status = EmuStatus {
            state,
            message: message.to_string(),
        };
        let mut last = lock(&self.last_status);
        if last.as_ref() == Some(&status) {
            return; // the retry loop would otherwise repeat the same message every 2 s
        }
        *last = Some(status.clone());
        drop(last);
        (self.on_status)(&status);
    }
}

pub struct EmulatorService {
    shared: Arc<Shared>,
    task: Option<JoinHandle<()>>,
}

impl EmulatorService {
    pub fn new(
        profile: MapProfile,
        on_status: impl Fn(&EmuStatus) + Send + Sync + 'static,
    ) -> Self {
        let shared = Shared {
            profile: RwLock::new(profile),
            latest: Mutex::new(None),
            transport: Mutex::new(Transport::None),
            last_status: Mutex::new(None),
            on_status: Box::new(on_status),
            emulating: AtomicBool::new(false),
            stop: AtomicBool::new(false),
            resend: AtomicBool::new(false),
            rumble: AtomicU16::new(0),
        };
        Self {
            shared: Arc::new(shared),
            task: None,
        }
    }

    /// Start emulating; also starts the reader thread if it is not running.
    pub fn start(&mut self) {
        self.shared.emulating.store(true, Relaxed);
        if self.is_running() {
            return;
        }
        self.shared.stop.store(false, Relaxed);
        let shared = Arc::clone(&self.shared);
        self.task = Some(std::thread::spawn(move || run(&shared)));
    }

    /// Stop emulating; the pad keeps being read (battery, lightbar). Drop ends the reader.
    pub fn stop(&self) {
        self.shared.emulating.store(false, Relaxed);
    }

    pub fn is_running(&self) -> bool {
        self.task.as_ref().is_some_and(|t| !t.is_finished())
    }

    pub fn is_emulating(&self) -> bool {
        self.shared.emulating.load(Relaxed)
    }

    /// Latest input; a stale frame is harmless.
    pub fn latest(&self) -> Option<Snapshot> {
        *lock(&self.shared.latest)
    }

    /// How the pad is connected right now (None while no pad is read).
    pub fn transport(&self) -> Transport {
        *lock(&self.shared.transport)
    }

    pub fn profile(&self) -> MapProfile {
        self.shared
            .profile
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// Tuning applied to every frame; swap the whole profile to change it live.
    pub fn set_profile(&self, profile: MapProfile) {
        *self
            .shared
            .profile
            .write()
            .unwrap_or_else(|e| e.into_inner()) = profile;
        self.refresh_output();
    }

    /// Re-sends the lightbar colour now (call after changing the lightbar settings).
    pub fn refresh_output(&self) {
        self.shared.resend.store(true, Relaxed);
    }
}

impl Drop for EmulatorService {
    fn drop(&mut self) {
        self.shared.stop.store(true, Relaxed);
        if let Some(t) = self.task.take() {
            let _ = t.join();
        }
    }
}

/// A virtual Xbox 360 pad. Dropping it unplugs the pad.
struct VirtualPad {
    target: Xbox360Wired<Client>,
}

fn vigem(e: VigemError) -> RunError {
    match e {
        VigemError::BusNotFound => RunError::DriverMissing,
        other => RunError::Other(format!("{other:?}")),
    }
}

impl VirtualPad {
    fn create(shared: &Arc<Shared>) -> Result<Self, RunError> {
        let client = Client::connect().map_err(vigem)?; // BusNotFound when the driver is missing
        let mut target = Xbox360Wired::new(client, TargetId::XBOX360_WIRED);
        target.plugin().map_err(vigem)?;
        target.wait_ready().map_err(vigem)?;
        let notification = target.request_notification().map_err(vigem)?;
        let shared = Arc::clone(shared);
        // the thread ends by itself when the target is dropped
        notification.spawn_thread(move |_, n| {
            shared.rumble.store(
                u16::from(n.large_motor) << 8 | u16::from(n.small_motor),
                Relaxed,
            );
            shared.resend.store(true, Relaxed);
        });
        Ok(Self { target })
    }

    fn submit(&mut self, x: &X360State) -> Result<(), RunError> {
        let gamepad = XGamepad {
            buttons: XButtons { raw: x.buttons },
            left_trigger: x.left_trigger,
            right_trigger: x.right_trigger,
            thumb_lx: x.left_x,
            thumb_ly: x.left_y,
            thumb_rx: x.right_x,
            thumb_ry: x.right_y,
        };
        self.target.update(&gamepad).map_err(vigem)
    }

    /// Release everything a game could see as held.
    fn neutral(&mut self) {
        let idle = DualSenseState::new(128, 128, 128, 128, 0, 0, 0);
        let _ = self.submit(&map(&idle, &MapProfile::default())); // pad already gone
    }
}

/// Keep trying until stopped: the pad may be off at login, or drop off Bluetooth mid-game.
fn run(shared: &Arc<Shared>) {
    // Created on first connect and kept across pad drops, so a Bluetooth blip does not unplug the Xbox pad from the game.
    let mut pad: Option<VirtualPad> = None;
    while !shared.stop.load(Relaxed) {
        match run_once(shared, &mut pad) {
            Ok(()) => {}
            Err(RunError::DriverMissing) => {
                pad = None;
                shared.report(EmuState::DriverMissing, "ViGEmBus driver is not installed.");
            }
            Err(RunError::Hid(e)) => shared.report(
                EmuState::Error,
                &format!("Controller error: {e}. Retrying..."),
            ),
            Err(RunError::Other(e)) => {
                pad = None; // not a pad-read problem: rebuild the virtual pad on the next try
                shared.report(
                    EmuState::Error,
                    &format!("Controller error: {e}. Retrying..."),
                );
            }
        }
        for _ in 0..40 {
            if shared.stop.load(Relaxed) {
                break;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    }
    drop(pad);
    shared.report(EmuState::Stopped, "Stopped");
}

fn run_once(shared: &Arc<Shared>, pad: &mut Option<VirtualPad>) -> Result<(), RunError> {
    let hid = |e: hidapi::HidError| RunError::Hid(e.to_string());
    let api = HidApi::new().map_err(hid)?;
    let mut found: Vec<_> = api
        .device_list()
        .filter(|d| d.vendor_id() == SONY_VID && DUALSENSE_PIDS.contains(&d.product_id()))
        .collect();
    // the gamepad collection (usage page 1, usage 5) when the pad exposes several
    found.sort_by_key(|d| (d.usage_page(), d.usage()) != (1, 5));
    let Some(info) = found.first() else {
        let state = if shared.emulating.load(Relaxed) {
            EmuState::Waiting
        } else {
            EmuState::Stopped
        };
        shared.report(state, "Waiting for a DualSense (USB or Bluetooth)...");
        return Ok(());
    };
    let dev = api.open_path(info.path()).map_err(hid)?;
    let bluetooth = matches!(info.bus_type(), BusType::Bluetooth);
    *lock(&shared.transport) = if bluetooth {
        Transport::Bluetooth
    } else {
        Transport::Usb
    };
    let result = (|| {
        if bluetooth {
            let mut feature = [0u8; BT_FEATURE_LENGTH];
            feature[0] = BT_ENABLE_FEATURE_ID;
            dev.get_feature_report(&mut feature).map_err(hid)?;
        }
        read_loop(shared, pad, &dev, bluetooth)
    })();
    if let Some(vp) = pad.as_mut() {
        vp.neutral(); // real pad gone: release everything the game could see as held
    }
    *lock(&shared.latest) = None;
    *lock(&shared.transport) = Transport::None;
    result
}

fn read_loop(
    shared: &Arc<Shared>,
    pad: &mut Option<VirtualPad>,
    dev: &HidDevice,
    bluetooth: bool,
) -> Result<(), RunError> {
    let mut seq = 0u8;
    let mut last_look: Look = (None, None);
    let mut buf = vec![
        0u8;
        if bluetooth {
            BT_INPUT_LENGTH
        } else {
            USB_INPUT_LENGTH
        }
    ];
    let mut send = |last_look: &mut Look| {
        let profile = shared.profile.read().unwrap_or_else(|e| e.into_inner());
        let ds = lock(&shared.latest).map(|s| s.ds);
        *last_look = (
            resolve(&profile, ds.as_ref()),
            resolve_leds(&profile, ds.as_ref()),
        );
        let rumble = shared.rumble.load(Relaxed);
        let report = build(
            bluetooth,
            (rumble >> 8) as u8,
            rumble as u8,
            last_look.0,
            seq & 0x0F,
            last_look.1,
        );
        seq = seq.wrapping_add(1);
        let _ = dev.write(&report); // pad vanished; the read loop notices and reconnects
    };
    shared.resend.store(true, Relaxed); // applies the lightbar colour right away
    while !shared.stop.load(Relaxed) {
        let n = dev
            .read_timeout(&mut buf, READ_TIMEOUT_MS)
            .map_err(|e| RunError::Hid(e.to_string()))?;
        if shared.resend.swap(false, Relaxed) {
            send(&mut last_look);
        }
        let Some(ds) = try_parse(&buf[..n]) else {
            continue;
        };
        let (x, look) = {
            let profile = shared.profile.read().unwrap_or_else(|e| e.into_inner());
            (
                map(&ds, &profile),
                (
                    resolve(&profile, Some(&ds)),
                    resolve_leds(&profile, Some(&ds)),
                ),
            )
        };
        *lock(&shared.latest) = Some(Snapshot { ds, x360: x });
        let mut changed = sync_pad(shared, pad, bluetooth)?;
        changed |= look != last_look; // battery or charge changed the lightbar colour, or the user changed mode / player
        if changed {
            send(&mut last_look);
        }
        if let Some(vp) = pad.as_mut() {
            vp.submit(&x)?;
        }
    }
    Ok(())
}

/// Create the virtual pad when emulation is switched on, release it (and any held input) when switched off.
/// Returns true when the output report must be re-sent.
fn sync_pad(
    shared: &Arc<Shared>,
    pad: &mut Option<VirtualPad>,
    bluetooth: bool,
) -> Result<bool, RunError> {
    if shared.emulating.load(Relaxed) {
        if pad.is_none() {
            *pad = Some(VirtualPad::create(shared)?); // DriverMissing when ViGEmBus is not installed
        }
        let via = if bluetooth { "Bluetooth" } else { "USB" };
        shared.report(
            EmuState::Running,
            &format!("Emulating: games see an Xbox 360 pad ({via})"),
        );
        return Ok(false);
    }
    let mut resend = false;
    if let Some(mut vp) = pad.take() {
        vp.neutral();
        drop(vp);
        shared.rumble.store(0, Relaxed);
        resend = true;
    }
    shared.report(EmuState::Stopped, "Stopped - battery still shown");
    Ok(resend)
}
