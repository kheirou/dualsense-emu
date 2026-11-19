//! Current pad status as JSON in %APPDATA%\DualSenseEmu\status.json, for status bars (YASB) to read.

use crate::service::{EmuState, Transport};
use dualsense_core::state::{ChargeState, DualSenseState};
use serde::Serialize;
use std::path::{Path, PathBuf};

pub fn default_path() -> PathBuf {
    let appdata = std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_default();
    appdata.join("DualSenseEmu").join("status.json")
}

#[derive(Serialize)]
struct Status {
    state: &'static str,
    transport: &'static str,
    battery: Option<i32>,
    charge: Option<&'static str>,
}

pub fn to_json(state: EmuState, transport: Transport, ds: Option<&DualSenseState>) -> String {
    let status = Status {
        state: match state {
            EmuState::Stopped => "stopped",
            EmuState::Waiting => "waiting",
            EmuState::Running => "running",
            EmuState::Error => "error",
            EmuState::DriverMissing => "drivermissing",
        },
        transport: match transport {
            Transport::None => "none",
            Transport::Usb => "usb",
            Transport::Bluetooth => "bluetooth",
        },
        battery: ds.map(|d| d.battery_percent).filter(|&b| b >= 0),
        charge: ds.map(|d| match d.charge {
            ChargeState::Unknown => "unknown",
            ChargeState::Discharging => "discharging",
            ChargeState::Charging => "charging",
            ChargeState::Full => "full",
        }),
    };
    serde_json::to_string(&status).expect("plain struct serializes")
}

/// Write via temp file + rename so a reader never sees half a file.
pub fn write(json: &str, path: Option<&Path>) {
    let path = path.map(Path::to_path_buf).unwrap_or_else(default_path);
    let tmp = path.with_extension("json.tmp");
    let result = path
        .parent()
        .map_or(Ok(()), std::fs::create_dir_all)
        .and_then(|()| std::fs::write(&tmp, json))
        .and_then(|()| std::fs::rename(&tmp, &path));
    // a reader has it open: the next change rewrites it
    drop(result);
}
