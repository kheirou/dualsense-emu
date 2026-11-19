//! Parses DualSense input reports (USB 0x01, Bluetooth 0x31).

use crate::state::{ds, ChargeState, DualSenseState};

pub const USB_REPORT_ID: u8 = 0x01;
pub const BT_REPORT_ID: u8 = 0x31;
const USB_MIN_LENGTH: usize = 11;
const BT_MIN_LENGTH: usize = 12;
/// Battery/charge byte, relative to the first axis byte (hid-playstation layout).
const STATUS_OFFSET: usize = 52;

pub fn try_parse(report: &[u8]) -> Option<DualSenseState> {
    let o = match *report.first()? {
        USB_REPORT_ID if report.len() >= USB_MIN_LENGTH => 1,
        BT_REPORT_ID if report.len() >= BT_MIN_LENGTH => 2, // extra seq/tag byte
        _ => return None,
    };
    let (b0, b1, b2) = (report[o + 7], report[o + 8], report[o + 9]);
    let mut btn = ds::NONE;
    for (byte, bit, flag) in [
        (b0, 0x10, ds::SQUARE),
        (b0, 0x20, ds::CROSS),
        (b0, 0x40, ds::CIRCLE),
        (b0, 0x80, ds::TRIANGLE),
        (b1, 0x01, ds::L1),
        (b1, 0x02, ds::R1),
        (b1, 0x10, ds::CREATE),
        (b1, 0x20, ds::OPTIONS),
        (b1, 0x40, ds::L3),
        (b1, 0x80, ds::R3),
        (b2, 0x01, ds::PS),
        (b2, 0x02, ds::TOUCHPAD),
        (b2, 0x04, ds::MUTE),
    ] {
        if byte & bit != 0 {
            btn |= flag;
        }
    }
    // d-pad hat: 0=N,1=NE,2=E,3=SE,4=S,5=SW,6=W,7=NW, 8=released
    let hat = b0 & 0x0F;
    if matches!(hat, 7 | 0 | 1) {
        btn |= ds::DPAD_UP;
    }
    if matches!(hat, 3..=5) {
        btn |= ds::DPAD_DOWN;
    }
    if matches!(hat, 5..=7) {
        btn |= ds::DPAD_LEFT;
    }
    if matches!(hat, 1..=3) {
        btn |= ds::DPAD_RIGHT;
    }
    let (battery_percent, charge) = match report.get(o + STATUS_OFFSET) {
        Some(&s) => parse_battery(s),
        None => (-1, ChargeState::Unknown),
    };
    Some(DualSenseState {
        lx: report[o],
        ly: report[o + 1],
        rx: report[o + 2],
        ry: report[o + 3],
        l2: report[o + 4],
        r2: report[o + 5],
        buttons: btn,
        battery_percent,
        charge,
    })
}

/// Low nibble = level 0-10, high nibble = state (0 discharging, 1 charging, 2 full, others = error).
pub fn parse_battery(status: u8) -> (i32, ChargeState) {
    let level = i32::from(status & 0x0F);
    match status >> 4 {
        0 => ((level * 10 + 5).min(100), ChargeState::Discharging),
        1 => ((level * 10 + 5).min(100), ChargeState::Charging),
        2 => (100, ChargeState::Full),
        _ => (-1, ChargeState::Unknown),
    }
}
