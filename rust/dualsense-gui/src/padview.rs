//! A vector gamepad (DualSense or Xbox 360) drawn on a 260x170 canvas: flat body, hairline outline,
//! muted idle buttons, pressed buttons fill with one accent. Only the face buttons keep their real colours.

use eframe::egui::{
    self, epaint::CubicBezierShape, Align2, Color32, CornerRadius, FontId, Pos2, Rect, Sense,
    Shape, Stroke, StrokeKind, Vec2,
};

pub const CANVAS: Vec2 = Vec2::new(260.0, 170.0);
const ACCENT: Color32 = Color32::from_rgb(0x4F, 0xB7, 0xD6);
const EDGE: Color32 = Color32::from_rgb(0x3A, 0x42, 0x56);
const RING: Color32 = Color32::from_rgb(0x7A, 0x82, 0x96);
const BODY: Color32 = Color32::from_rgb(0x1A, 0x1E, 0x28);
const OFF: Color32 = Color32::from_rgb(0x11, 0x14, 0x1C);
const LABEL_OFF: Color32 = Color32::from_rgb(0xA3, 0xA9, 0xB8);
const LABEL_ON: Color32 = Color32::from_rgb(0x0B, 0x0D, 0x14);

/// What to draw: the pad kind and the live input. Sticks are raw 0-255 (centre 128); triggers 0-255.
pub struct PadInput<'a> {
    pub xbox: bool,
    pub pressed: &'a dyn Fn(&str) -> bool,
    pub l2: u8,
    pub r2: u8,
    pub left: (u8, u8),
    pub right: (u8, u8),
}

/// Flattens an SVG-style path made of absolute M / L / C / Z commands into a polygon.
fn path(d: &str) -> Vec<[f32; 2]> {
    let mut pts: Vec<[f32; 2]> = Vec::new();
    let mut nums = Vec::new();
    let mut cmd = ' ';
    let mut cur = [0.0f32; 2];
    let flush = |cmd: char, nums: &mut Vec<f32>, pts: &mut Vec<[f32; 2]>, cur: &mut [f32; 2]| {
        match cmd {
            'M' | 'L' if nums.len() >= 2 => {
                *cur = [nums[0], nums[1]];
                pts.push(*cur);
            }
            'C' if nums.len() >= 6 => {
                let p = |i: usize| Pos2::new(nums[i], nums[i + 1]);
                let bez = CubicBezierShape::from_points_stroke(
                    [Pos2::new(cur[0], cur[1]), p(0), p(2), p(4)],
                    false,
                    Color32::TRANSPARENT,
                    Stroke::NONE,
                );
                pts.extend(
                    bez.flatten(Some(0.2))
                        .into_iter()
                        .skip(1)
                        .map(|q| [q.x, q.y]),
                );
                *cur = [nums[4], nums[5]];
            }
            _ => {}
        }
        nums.clear();
    };
    let mut buf = String::new();
    for ch in d.chars().chain(std::iter::once(' ')) {
        if ch.is_ascii_digit() || ch == '.' || ch == '-' {
            buf.push(ch);
            continue;
        }
        if !buf.is_empty() {
            nums.push(buf.parse().unwrap_or(0.0));
            buf.clear();
        }
        if ch.is_ascii_alphabetic() {
            flush(cmd, &mut nums, &mut pts, &mut cur);
            cmd = ch;
        }
    }
    flush(cmd, &mut nums, &mut pts, &mut cur);
    pts
}

pub fn show(ui: &mut egui::Ui, scale: f32, input: &PadInput) {
    let (rect, _) = ui.allocate_exact_size(CANVAS * scale, Sense::hover());
    let painter = ui.painter_at(rect);
    let at = |x: f32, y: f32| rect.min + Vec2::new(x, y) * scale;
    let poly = |d: &str| -> Vec<Pos2> { path(d).into_iter().map(|[x, y]| at(x, y)).collect() };
    let xbox = input.xbox;

    painter.add(Shape::convex_polygon(
        poly("M 40,24 C 40,18 55,18 75,18 L 185,18 C 205,18 220,18 220,24 C 236,70 252,128 234,150 C 220,166 196,162 186,142 L 176,122 L 84,122 L 74,142 C 64,162 40,166 26,150 C 8,128 24,70 40,24 Z"),
        BODY,
        Stroke::new(1.0, EDGE),
    ));

    // shoulders: curved trigger caps on the outer corners (fill with pull), bumper keys just inside them
    let cap = |geom: &str, pull: u8, label: &str, lx: f32| {
        let pts = poly(geom);
        painter.add(Shape::convex_polygon(
            pts.clone(),
            OFF,
            Stroke::new(1.0, RING),
        ));
        painter.add(Shape::convex_polygon(
            pts,
            ACCENT.gamma_multiply(f32::from(pull) / 255.0),
            Stroke::NONE,
        ));
        painter.text(
            at(lx, 6.0),
            Align2::LEFT_TOP,
            label,
            FontId::proportional(6.0 * scale),
            LABEL_OFF,
        );
    };
    cap(
        "M 30,27 C 30,12 40,4 54,4 L 60,4 C 63,4 64,6 64,9 L 64,16 C 52,16 42,20 37,28 Z",
        input.l2,
        if xbox { "LT" } else { "L2" },
        41.0,
    );
    cap("M 230,27 C 230,12 220,4 206,4 L 200,4 C 197,4 196,6 196,9 L 196,16 C 208,16 218,20 223,28 Z", input.r2, if xbox { "RT" } else { "R2" }, 199.0);

    let part = |key: &str,
                x: f32,
                y: f32,
                w: f32,
                h: f32,
                r: f32,
                label: Option<&str>,
                font: f32,
                colour: Option<Color32>| {
        let r_ = Rect::from_min_size(at(x, y), Vec2::new(w, h) * scale);
        let on = (input.pressed)(key);
        let edge = colour.map_or(EDGE, |c| c.gamma_multiply(0.6));
        let fill = if on { colour.unwrap_or(ACCENT) } else { OFF };
        painter.rect(
            r_,
            CornerRadius::same((r * scale) as u8),
            fill,
            Stroke::new(1.0, edge),
            StrokeKind::Inside,
        );
        if let Some(l) = label {
            let ink = if on {
                LABEL_ON
            } else {
                colour.unwrap_or(LABEL_OFF)
            };
            painter.text(
                r_.center(),
                Align2::CENTER_CENTER,
                l,
                FontId::proportional(font * scale),
                ink,
            );
        }
        ui.interact(r_, ui.scope_id().with((xbox, key)), Sense::hover())
            .on_hover_text(spaced(key));
    };
    part(
        if xbox { "LeftShoulder" } else { "L1" },
        68.0,
        7.0,
        24.0,
        12.0,
        4.0,
        Some(if xbox { "LB" } else { "L1" }),
        7.0,
        None,
    );
    part(
        if xbox { "RightShoulder" } else { "R1" },
        168.0,
        7.0,
        24.0,
        12.0,
        4.0,
        Some(if xbox { "RB" } else { "R1" }),
        7.0,
        None,
    );

    let dpad = |cx: f32, cy: f32| {
        part(
            "DpadUp",
            cx - 7.0,
            cy - 21.0,
            14.0,
            14.0,
            3.0,
            Some("▲"),
            7.0,
            None,
        );
        part(
            "DpadDown",
            cx - 7.0,
            cy + 7.0,
            14.0,
            14.0,
            3.0,
            Some("▼"),
            7.0,
            None,
        );
        part(
            "DpadLeft",
            cx - 21.0,
            cy - 7.0,
            14.0,
            14.0,
            3.0,
            Some("◀"),
            7.0,
            None,
        );
        part(
            "DpadRight",
            cx + 7.0,
            cy - 7.0,
            14.0,
            14.0,
            3.0,
            Some("▶"),
            7.0,
            None,
        );
    };
    let (green, red, amber, blue, violet) = (
        Color32::from_rgb(0x4C, 0xC3, 0x8A),
        Color32::from_rgb(0xE5, 0x48, 0x4D),
        Color32::from_rgb(0xE0, 0xA2, 0x3F),
        Color32::from_rgb(0x4F, 0xB7, 0xD6),
        Color32::from_rgb(0xC9, 0x7B, 0xD6),
    );
    let (left_c, right_c) = if xbox {
        dpad(98.0, 104.0);
        part("Back", 106.0, 52.0, 14.0, 9.0, 4.0, Some("◀"), 5.0, None);
        part("Start", 140.0, 52.0, 14.0, 9.0, 4.0, Some("▶"), 5.0, None);
        part(
            "Guide",
            120.0,
            29.0,
            20.0,
            20.0,
            10.0,
            Some("✕"),
            10.0,
            None,
        );
        part(
            "Y",
            189.0,
            42.0,
            18.0,
            18.0,
            9.0,
            Some("Y"),
            10.0,
            Some(amber),
        );
        part(
            "B",
            207.0,
            60.0,
            18.0,
            18.0,
            9.0,
            Some("B"),
            10.0,
            Some(red),
        );
        part(
            "A",
            189.0,
            78.0,
            18.0,
            18.0,
            9.0,
            Some("A"),
            10.0,
            Some(green),
        );
        part(
            "X",
            171.0,
            60.0,
            18.0,
            18.0,
            9.0,
            Some("X"),
            10.0,
            Some(blue),
        );
        ((62.0, 56.0), (162.0, 104.0))
    } else {
        dpad(62.0, 66.0);
        part("Touchpad", 100.0, 28.0, 60.0, 26.0, 6.0, None, 0.0, None);
        part("Create", 84.0, 32.0, 8.0, 12.0, 4.0, None, 0.0, None);
        part("Options", 168.0, 32.0, 8.0, 12.0, 4.0, None, 0.0, None);
        part("Ps", 123.0, 66.0, 14.0, 14.0, 7.0, Some("PS"), 5.0, None);
        part("Mute", 124.0, 86.0, 12.0, 5.0, 2.0, None, 0.0, None);
        part(
            "Triangle",
            189.0,
            42.0,
            18.0,
            18.0,
            9.0,
            Some("△"),
            10.0,
            Some(green),
        );
        part(
            "Circle",
            207.0,
            60.0,
            18.0,
            18.0,
            9.0,
            Some("○"),
            10.0,
            Some(red),
        );
        part(
            "Cross",
            189.0,
            78.0,
            18.0,
            18.0,
            9.0,
            Some("✕"),
            10.0,
            Some(Color32::from_rgb(0x5B, 0x8D, 0xEF)),
        );
        part(
            "Square",
            171.0,
            60.0,
            18.0,
            18.0,
            9.0,
            Some("□"),
            10.0,
            Some(violet),
        );
        ((98.0, 104.0), (162.0, 104.0))
    };

    // sticks: hairline rings with crosshair lines (fill on L3/R3); the accent dot follows the stick
    let line = RING.gamma_multiply(0.33);
    for (key, (cx, cy), (sx, sy)) in [
        (if xbox { "LeftThumb" } else { "L3" }, left_c, input.left),
        (if xbox { "RightThumb" } else { "R3" }, right_c, input.right),
    ] {
        part(key, cx - 19.0, cy - 19.0, 38.0, 38.0, 19.0, None, 0.0, None);
        painter.circle_stroke(at(cx, cy), 12.0 * scale, Stroke::new(0.6, line));
        painter.line_segment(
            [at(cx - 16.0, cy), at(cx + 16.0, cy)],
            Stroke::new(0.5, line),
        );
        painter.line_segment(
            [at(cx, cy - 16.0), at(cx, cy + 16.0)],
            Stroke::new(0.5, line),
        );
        // +-12px of travel inside the 38px well
        let dx = (f32::from(sx) - 128.0) / 128.0 * 12.0;
        let dy = (f32::from(sy) - 128.0) / 128.0 * 12.0;
        painter.circle_filled(at(cx + dx, cy + dy), 4.0 * scale, ACCENT);
    }
}

/// "LeftThumb" -> "Left Thumb"
pub fn spaced(key: &str) -> String {
    let mut out = String::new();
    for (i, c) in key.chars().enumerate() {
        if i > 0 && c.is_uppercase() && key.chars().nth(i - 1).is_some_and(char::is_lowercase) {
            out.push(' ');
        }
        out.push(c);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spaced_splits_camel_case() {
        assert_eq!(spaced("LeftThumb"), "Left Thumb");
        assert_eq!(spaced("L1"), "L1");
    }

    #[test]
    fn path_flattens_lines_and_curves() {
        assert_eq!(
            path("M 0,0 L 10,0 L 10,10 Z"),
            vec![[0.0, 0.0], [10.0, 0.0], [10.0, 10.0]]
        );
        let curve = path("M 0,0 C 0,10 10,10 10,0");
        assert!(curve.len() > 3);
        assert_eq!(curve.last(), Some(&[10.0, 0.0]));
    }
}
