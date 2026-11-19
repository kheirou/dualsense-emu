//! DualSense to Xbox 360 translation.

use crate::profile::{DsButton, MapProfile, X360Button};
use crate::state::DualSenseState;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct X360State {
    pub buttons: u16,
    pub left_trigger: u8,
    pub right_trigger: u8,
    pub left_x: i16,
    pub left_y: i16,
    pub right_x: i16,
    pub right_y: i16,
}

const DEFAULT: [(DsButton, X360Button); 15] = [
    (DsButton::Cross, X360Button::A),
    (DsButton::Circle, X360Button::B),
    (DsButton::Square, X360Button::X),
    (DsButton::Triangle, X360Button::Y),
    (DsButton::L1, X360Button::LeftShoulder),
    (DsButton::R1, X360Button::RightShoulder),
    (DsButton::Create, X360Button::Back),
    (DsButton::Options, X360Button::Start),
    (DsButton::L3, X360Button::LeftThumb),
    (DsButton::R3, X360Button::RightThumb),
    (DsButton::Ps, X360Button::Guide),
    (DsButton::DpadUp, X360Button::DpadUp),
    (DsButton::DpadDown, X360Button::DpadDown),
    (DsButton::DpadLeft, X360Button::DpadLeft),
    (DsButton::DpadRight, X360Button::DpadRight),
];

/// The built-in target of a DualSense button, or None (Touchpad, Mute).
pub fn default_for(b: DsButton) -> Option<X360Button> {
    DEFAULT.iter().find(|d| d.0 == b).map(|d| d.1)
}

pub fn map(s: &DualSenseState, p: &MapProfile) -> X360State {
    let mut b = 0u16;
    for (from, to) in DEFAULT {
        if s.buttons & from.bit() == 0 {
            continue;
        }
        b |= p
            .remap
            .as_ref()
            .and_then(|r| r.get(&from))
            .copied()
            .unwrap_or(to)
            .bits();
    }
    if let Some(remap) = &p.remap {
        for (from, to) in remap {
            if s.buttons & from.bit() != 0 && default_for(*from).is_none() {
                b |= to.bits(); // Touchpad, Mute
            }
        }
    }
    let swap = p.swap_sticks;
    let ((lx, ly), (rx, ry)) = if swap {
        ((s.rx, s.ry), (s.lx, s.ly))
    } else {
        ((s.lx, s.ly), (s.rx, s.ry))
    };
    let (ldz, rdz) = if swap {
        (p.right_deadzone, p.left_deadzone)
    } else {
        (p.left_deadzone, p.right_deadzone)
    };
    let (linv, rinv) = if swap {
        (p.invert_right_y, p.invert_left_y)
    } else {
        (p.invert_left_y, p.invert_right_y)
    };
    let (left_x, left_y) = stick(lx, ly, ldz, p.curve, linv);
    let (right_x, right_y) = stick(rx, ry, rdz, p.curve, rinv);
    X360State {
        buttons: b,
        left_trigger: trigger(s.l2, p.trigger_threshold),
        right_trigger: trigger(s.r2, p.trigger_threshold),
        left_x,
        left_y,
        right_x,
        right_y,
    }
}

/// 0-255 (centre 128) to -32768..32767. DualSense Y is up=0, Xbox Y is up=+.
pub fn axis(v: u8, invert: bool) -> i16 {
    let mut x = if v == 255 {
        32767
    } else {
        (i32::from(v) - 128) * 256
    };
    if invert {
        x = if x == -32768 { 32767 } else { -x };
    }
    x.clamp(-32768, 32767) as i16
}

/// Reads 0 at or below `threshold`; the rest is rescaled to the full 0-255 range.
pub fn trigger(v: u8, threshold: u8) -> u8 {
    if v <= threshold {
        return 0;
    }
    let scaled = (i32::from(v) - i32::from(threshold)) * 255 / (255 - i32::from(threshold));
    scaled.min(255) as u8
}

/// Radial deadzone then curve. With deadzone 0 and curve 1 this equals `axis`.
pub fn stick(x: u8, y: u8, deadzone: f64, curve: f64, invert_y: bool) -> (i16, i16) {
    let vx = f64::from(axis(x, false)) / 32768.0;
    let vy = f64::from(axis(y, !invert_y)) / 32768.0; // Xbox Y is up = +
    let mag = (vx * vx + vy * vy).sqrt();
    if mag <= deadzone || mag == 0.0 {
        return (0, 0);
    }
    let dz = deadzone.clamp(0.0, 0.9);
    let scaled = ((mag - dz) / (1.0 - dz)).powf(curve.max(0.1));
    let f = scaled / mag * 32768.0;
    (
        (vx * f).clamp(-32768.0, 32767.0) as i16,
        (vy * f).clamp(-32768.0, 32767.0) as i16,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{ds, x360};

    #[test]
    fn edge_values() {
        assert_eq!(axis(0, false), -32768);
        assert_eq!(axis(128, false), 0);
        assert_eq!(axis(255, false), 32767);
        assert_eq!(trigger(255, 255), 0); // no divide by zero
        assert_eq!(stick(128, 128, 0.0, 1.0, false), (0, 0));
    }

    #[test]
    fn cross_and_circle_map_to_a_and_b() {
        let s = DualSenseState::new(128, 128, 128, 128, 0, 0, ds::CROSS | ds::CIRCLE);
        assert_eq!(map(&s, &MapProfile::default()).buttons, x360::A | x360::B);
    }
}
