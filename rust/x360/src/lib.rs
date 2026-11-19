//! Port of `X360Mapper.Axis`, `Trigger` and `Stick` (src/DualSenseEmu.Core/X360Mapper.cs).
//! `tests/golden.jsonl` holds what the C# returned; `cargo test` replays it.

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

// C ABI for P/Invoke. Stick packs X in the low 16 bits and Y in the high 16 bits.
#[no_mangle]
pub extern "C" fn x360_axis(v: u8, invert: bool) -> i16 {
    axis(v, invert)
}

#[no_mangle]
pub extern "C" fn x360_trigger(v: u8, threshold: u8) -> u8 {
    trigger(v, threshold)
}

#[no_mangle]
pub extern "C" fn x360_stick(x: u8, y: u8, deadzone: f64, curve: f64, invert_y: bool) -> u32 {
    let (ox, oy) = stick(x, y, deadzone, curve, invert_y);
    u32::from(ox as u16) | (u32::from(oy as u16) << 16)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edge_values() {
        assert_eq!(axis(0, false), -32768);
        assert_eq!(axis(128, false), 0);
        assert_eq!(axis(255, false), 32767);
        assert_eq!(trigger(255, 255), 0); // no divide by zero
        assert_eq!(stick(128, 128, 0.0, 1.0, false), (0, 0));
    }

    #[test]
    fn pack_roundtrip() {
        let p = x360_stick(255, 0, 0.0, 1.0, false);
        assert_eq!(((p & 0xFFFF) as u16) as i16, 32767);
        assert_eq!(((p >> 16) as u16) as i16, 32767);
    }
}
