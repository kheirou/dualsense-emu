//! Builds DualSense output reports (rumble motors, lightbar, player LEDs).

use crate::profile::{LightbarMode, MapProfile, PlayerLedMode};
use crate::state::{ChargeState, DualSenseState};

pub const USB_LENGTH: usize = 63;
pub const BT_LENGTH: usize = 78;
const USB_ID: u8 = 0x02;
const BT_ID: u8 = 0x31;

pub type Rgb = (u8, u8, u8);

/// `rgb` is the lightbar colour, or None to leave it alone.
pub fn build(
    bluetooth: bool,
    left: u8,
    right: u8,
    rgb: Option<Rgb>,
    bt_seq: u8,
    leds: Option<u8>,
) -> Vec<u8> {
    let mut r = vec![0u8; if bluetooth { BT_LENGTH } else { USB_LENGTH }];
    let o = if bluetooth {
        r[0] = BT_ID;
        r[1] = bt_seq << 4;
        r[2] = 0x10;
        3
    } else {
        r[0] = USB_ID;
        1
    }; // index of valid_flag0
    r[o] = 0x01 | 0x02; // compatible vibration + haptics select
    r[o + 2] = right; // small motor
    r[o + 3] = left; // big motor
    if let Some(mask) = leds {
        r[o + 1] |= 0x10; // player indicator control enable
        r[o + 43] = mask & 0x1F;
    }
    if let Some((red, green, blue)) = rgb {
        r[o + 1] |= 0x04; // lightbar control enable
        r[o + 38] = 0x02; // valid_flag2: lightbar setup enable
        r[o + 41] = 0x02; // lightbar setup: fade out the pairing animation
        r[o + 42] = 0x00; // brightness: brightest
        (r[o + 44], r[o + 45], r[o + 46]) = (red, green, blue);
    }
    if bluetooth {
        let crc = crc32(&r[..BT_LENGTH - 4]);
        r[BT_LENGTH - 4..].copy_from_slice(&crc.to_le_bytes());
    }
    r
}

// the five LEDs left to right; player N lights the centre then spreads outwards
const PLAYER_LEDS: [u8; 4] = [0x04, 0x0A, 0x15, 0x1B];

/// Bit mask of the five player LEDs to light now; None = leave them alone.
pub fn resolve_leds(p: &MapProfile, ds: Option<&DualSenseState>) -> Option<u8> {
    match (p.player_mode, ds) {
        (PlayerLedMode::Player, _) if (1..=4).contains(&p.player_number) => {
            Some(PLAYER_LEDS[usize::from(p.player_number) - 1])
        }
        (PlayerLedMode::Battery, Some(s)) if s.battery_percent >= 0 => {
            Some(((1u32 << ((s.battery_percent + 19) / 20).clamp(1, 5)) - 1) as u8)
        }
        (PlayerLedMode::Charging, Some(s)) => Some(
            if matches!(s.charge, ChargeState::Charging | ChargeState::Full) {
                0x1F
            } else {
                0
            },
        ),
        _ => None,
    }
}

const GREEN: Rgb = (0x3D, 0xDC, 0x84);
const AMBER: Rgb = (0xFF, 0xB1, 0x3D);
const RED: Rgb = (0xFF, 0x3B, 0x30);

/// The colour the lightbar should show now; None = leave the pad's own.
pub fn resolve(p: &MapProfile, ds: Option<&DualSenseState>) -> Option<Rgb> {
    let fixed = parse_colour(p.lightbar.as_deref());
    let Some(s) = ds else { return fixed };
    match p.lightbar_mode {
        LightbarMode::Battery if s.battery_percent >= 0 => Some(match s.battery_percent {
            ..=19 => RED,
            20..=39 => AMBER,
            _ => GREEN,
        }),
        LightbarMode::Charging => match s.charge {
            ChargeState::Charging => Some(AMBER),
            ChargeState::Full => Some(GREEN),
            _ => fixed,
        },
        _ => fixed,
    }
}

/// CRC-32 (zlib) over the 0xA2 seed byte then the report body.
pub fn crc32(body: &[u8]) -> u32 {
    let mut c = 0xFFFF_FFFFu32;
    for &b in std::iter::once(&0xA2).chain(body) {
        c ^= u32::from(b);
        for _ in 0..8 {
            c = if c & 1 != 0 {
                (c >> 1) ^ 0xEDB8_8320
            } else {
                c >> 1
            };
        }
    }
    !c
}

/// "#RRGGBB" or "RRGGBB"; anything else is None (lightbar untouched).
pub fn parse_colour(hex: Option<&str>) -> Option<Rgb> {
    let hex = hex?.trim_start_matches('#');
    if hex.encode_utf16().count() != 6 {
        return None;
    }
    // .NET HexNumber also allows leading and trailing white space (0x09-0x0D, 0x20)
    let digits = hex.trim_matches(|c| matches!(c, '\t'..='\r' | ' '));
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let v = u32::from_str_radix(digits, 16).ok()?;
    Some(((v >> 16) as u8, (v >> 8) as u8, v as u8))
}
