//! Raw DualSense state. Buttons are bit sets (`ds::*`, `x360::*`), as in the C# flags enums.

pub mod ds {
    pub const NONE: u32 = 0;
    pub const SQUARE: u32 = 1 << 0;
    pub const CROSS: u32 = 1 << 1;
    pub const CIRCLE: u32 = 1 << 2;
    pub const TRIANGLE: u32 = 1 << 3;
    pub const L1: u32 = 1 << 4;
    pub const R1: u32 = 1 << 5;
    pub const CREATE: u32 = 1 << 6;
    pub const OPTIONS: u32 = 1 << 7;
    pub const L3: u32 = 1 << 8;
    pub const R3: u32 = 1 << 9;
    pub const PS: u32 = 1 << 10;
    pub const TOUCHPAD: u32 = 1 << 11;
    pub const MUTE: u32 = 1 << 12;
    pub const DPAD_UP: u32 = 1 << 13;
    pub const DPAD_DOWN: u32 = 1 << 14;
    pub const DPAD_LEFT: u32 = 1 << 15;
    pub const DPAD_RIGHT: u32 = 1 << 16;
}

pub mod x360 {
    pub const NONE: u16 = 0;
    pub const DPAD_UP: u16 = 0x0001;
    pub const DPAD_DOWN: u16 = 0x0002;
    pub const DPAD_LEFT: u16 = 0x0004;
    pub const DPAD_RIGHT: u16 = 0x0008;
    pub const START: u16 = 0x0010;
    pub const BACK: u16 = 0x0020;
    pub const LEFT_THUMB: u16 = 0x0040;
    pub const RIGHT_THUMB: u16 = 0x0080;
    pub const LEFT_SHOULDER: u16 = 0x0100;
    pub const RIGHT_SHOULDER: u16 = 0x0200;
    pub const GUIDE: u16 = 0x0400;
    pub const A: u16 = 0x1000;
    pub const B: u16 = 0x2000;
    pub const X: u16 = 0x4000;
    pub const Y: u16 = 0x8000;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChargeState {
    Unknown,
    Discharging,
    Charging,
    Full,
}

/// Sticks/triggers 0-255 (sticks centre at 128, up = 0).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DualSenseState {
    pub lx: u8,
    pub ly: u8,
    pub rx: u8,
    pub ry: u8,
    pub l2: u8,
    pub r2: u8,
    pub buttons: u32,
    /// -1 when unknown.
    pub battery_percent: i32,
    pub charge: ChargeState,
}

impl DualSenseState {
    pub fn new(lx: u8, ly: u8, rx: u8, ry: u8, l2: u8, r2: u8, buttons: u32) -> Self {
        Self {
            lx,
            ly,
            rx,
            ry,
            l2,
            r2,
            buttons,
            battery_percent: -1,
            charge: ChargeState::Unknown,
        }
    }
}
