//! A vector gamepad (DualSense or Xbox 360) drawn on a 260x170 canvas: white shaded shell, soft shadow,
//! glossy buttons, sticks whose rubber caps follow the input. Pressed buttons light up.

use eframe::egui::{
    self, epaint::CubicBezierShape, Align2, Color32, CornerRadius, FontId, Pos2, Rect, Sense,
    Shape, Stroke, StrokeKind, Vec2,
};

pub const CANVAS: Vec2 = Vec2::new(260.0, 170.0);
const ACCENT: Color32 = Color32::from_rgb(0x4F, 0xB7, 0xD6);
const SHELL_EDGE: Color32 = Color32::from_rgb(0x9C, 0xA3, 0xB0);
const DARK: Color32 = Color32::from_rgb(0x1B, 0x1E, 0x25);
const KEY: Color32 = Color32::from_rgb(0x2E, 0x32, 0x3B);
const KEY_EDGE: Color32 = Color32::from_rgb(0x4C, 0x52, 0x5F);
const PALE_KEY: Color32 = Color32::from_rgb(0xCF, 0xD3, 0xDB);
const LABEL_OFF: Color32 = Color32::from_rgb(0xB4, 0xBA, 0xC6);
const LABEL_ON: Color32 = Color32::from_rgb(0x12, 0x14, 0x1A);

// the DualSense has a wide body with a black centre panel; the 360 pad has rounder grips
const DS_BODY: &str = "M 38,26 C 38,18 55,17 75,17 L 185,17 C 205,17 222,18 222,26 C 238,72 256,132 240,156 C 228,172 198,168 188,144 L 178,124 L 82,124 L 72,144 C 62,168 32,172 20,156 C 4,132 22,72 38,26 Z";
const XBOX_BODY: &str = "M 44,26 C 44,16 58,14 80,14 L 180,14 C 202,14 216,16 216,26 C 234,64 258,134 240,158 C 226,172 196,164 186,140 L 176,122 L 84,122 L 74,140 C 64,164 34,172 20,158 C 2,134 26,64 44,26 Z";
const DS_PANEL: &str = "M 94,18 L 166,18 C 171,50 170,96 165,123 L 95,123 C 90,96 89,50 94,18 Z";

/// Blends towards white (`t` 0..1).
fn lit(c: Color32, t: f32) -> Color32 {
    let m = |v: u8| (f32::from(v) + (255.0 - f32::from(v)) * t) as u8;
    Color32::from_rgb(m(c.r()), m(c.g()), m(c.b()))
}

/// Scales the channels (`k` < 1 darkens).
fn shade(c: Color32, k: f32) -> Color32 {
    let m = |v: u8| (f32::from(v) * k) as u8;
    Color32::from_rgb(m(c.r()), m(c.g()), m(c.b()))
}

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

    // shell: soft shadow, rim, face, then light from above (smoothly darker towards the grips).
    // The outline is concave at the grip notch, so it is filled as a fan from a point inside it.
    let body = poly(if xbox { XBOX_BODY } else { DS_BODY });
    let mid = (body.iter().fold(Vec2::ZERO, |a, p| a + p.to_vec2()) / body.len() as f32).to_pos2();
    let moved = |k: f32, dy: f32| -> Vec<Pos2> {
        body.iter()
            .map(|p| mid + (*p - mid) * k + Vec2::new(0.0, dy * scale))
            .collect()
    };
    let fan = |pts: &[Pos2], c: Color32, clip: Rect| {
        let mut m = egui::Mesh::default();
        m.colored_vertex(at(130.0, 64.0), c);
        for p in pts {
            m.colored_vertex(*p, c);
        }
        let n = pts.len() as u32;
        for i in 1..n {
            m.add_triangle(0, i, i + 1);
        }
        m.add_triangle(0, n, 1);
        painter.with_clip_rect(clip).add(m);
    };
    let fill = |pts: Vec<Pos2>, c: Color32, s: Stroke| {
        painter.add(Shape::convex_polygon(pts, c, s));
    };
    for (d, a) in [(7.0, 10u8), (5.0, 14), (3.0, 18)] {
        fan(&moved(1.0, d), Color32::from_black_alpha(a), rect);
    }
    fan(&body, SHELL_EDGE, rect);
    let face = moved(0.978, -0.8);
    // one opaque mesh, lit from above: vertex colour follows height, so there are no seams
    let lit_by_y = |y: f32| {
        let t = ((y - rect.min.y) / (CANVAS.y * scale)).clamp(0.0, 1.0);
        let (top, low) = (
            Color32::from_rgb(0xF1, 0xF3, 0xF6),
            Color32::from_rgb(0xB4, 0xBA, 0xC4),
        );
        let m = |a: u8, b: u8| (f32::from(a) + (f32::from(b) - f32::from(a)) * t) as u8;
        Color32::from_rgb(
            m(top.r(), low.r()),
            m(top.g(), low.g()),
            m(top.b(), low.b()),
        )
    };
    let mut m = egui::Mesh::default();
    let centre = at(130.0, 64.0);
    m.colored_vertex(centre, lit_by_y(centre.y));
    for p in &face {
        m.colored_vertex(*p, lit_by_y(p.y));
    }
    let n = face.len() as u32;
    for i in 1..n {
        m.add_triangle(0, i, i + 1);
    }
    m.add_triangle(0, n, 1);
    painter.add(m);
    if !xbox {
        fill(poly(DS_PANEL), DARK, Stroke::new(1.0, KEY));
        // the light bar: a U of light under the touchpad
        let u = vec![
            at(97.0, 30.0),
            at(97.0, 57.0),
            at(163.0, 57.0),
            at(163.0, 30.0),
        ];
        painter.add(Shape::line(
            u.clone(),
            Stroke::new(4.0 * scale, ACCENT.gamma_multiply(0.18)),
        ));
        painter.add(Shape::line(u, Stroke::new(1.4 * scale, ACCENT)));
    }

    // shoulders: dark glossy trigger caps on the outer corners (fill with pull), bumper keys just inside
    let cap = |geom: &str, pull: u8, label: &str, lx: f32| {
        let pts = poly(geom);
        painter.add(Shape::convex_polygon(
            pts.clone(),
            KEY,
            Stroke::new(1.0, KEY_EDGE),
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

    // a key: coloured Xbox face keys are solid; everything else is a dark key with a coloured symbol
    let part = |key: &str,
                x: f32,
                y: f32,
                w: f32,
                h: f32,
                r: f32,
                label: Option<&str>,
                font: f32,
                colour: Option<Color32>,
                base: Color32| {
        let r_ = Rect::from_min_size(at(x, y), Vec2::new(w, h) * scale);
        let on = (input.pressed)(key);
        let solid = xbox && colour.is_some();
        let fill = if on {
            colour.map_or(ACCENT, |c| lit(c, 0.3))
        } else if solid {
            colour.unwrap_or(base)
        } else {
            base
        };
        let edge = colour.map_or(KEY_EDGE, |c| shade(c, if solid { 0.6 } else { 0.7 }));
        let rr = CornerRadius::same((r * scale) as u8);
        if on {
            painter.rect_stroke(
                r_.expand(1.5 * scale),
                CornerRadius::same(((r + 1.5) * scale) as u8),
                Stroke::new(
                    2.0 * scale,
                    Color32::from_rgba_unmultiplied(fill.r(), fill.g(), fill.b(), 110),
                ),
                StrokeKind::Outside,
            );
        }
        painter.rect(r_, rr, fill, Stroke::new(1.0, edge), StrokeKind::Inside);
        let gloss = Color32::from_white_alpha(if base == PALE_KEY { 90 } else { 46 });
        if r * 2.0 >= w {
            painter.circle_filled(
                r_.center() + Vec2::new(-0.2 * w, -0.24 * h) * scale,
                0.17 * w * scale,
                gloss,
            );
        } else {
            let inset = Vec2::new(r * 0.6 * scale, 1.3 * scale);
            painter.line_segment(
                [
                    r_.left_top() + inset,
                    r_.right_top() + Vec2::new(-inset.x, inset.y),
                ],
                Stroke::new(1.0, gloss),
            );
        }
        if let Some(l) = label {
            let pale = solid || base == PALE_KEY;
            let ink = if on || solid {
                LABEL_ON
            } else {
                colour.unwrap_or(if pale { LABEL_ON } else { LABEL_OFF })
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
    let bumper = |key: &str, x: f32, label: &str| {
        part(key, x, 7.0, 24.0, 12.0, 4.0, Some(label), 7.0, None, KEY);
    };
    bumper(
        if xbox { "LeftShoulder" } else { "L1" },
        68.0,
        if xbox { "LB" } else { "L1" },
    );
    bumper(
        if xbox { "RightShoulder" } else { "R1" },
        168.0,
        if xbox { "RB" } else { "R1" },
    );

    let dpad = |cx: f32, cy: f32| {
        let base = if xbox { PALE_KEY } else { KEY };
        for (key, dx, dy, arrow) in [
            ("DpadUp", -7.0, -21.0, "▲"),
            ("DpadDown", -7.0, 7.0, "▼"),
            ("DpadLeft", -21.0, -7.0, "◀"),
            ("DpadRight", 7.0, -7.0, "▶"),
        ] {
            part(
                key,
                cx + dx,
                cy + dy,
                14.0,
                14.0,
                3.0,
                Some(arrow),
                7.0,
                None,
                base,
            );
        }
    };
    let (green, red, amber, blue, violet) = (
        Color32::from_rgb(0x4C, 0xC3, 0x8A),
        Color32::from_rgb(0xE5, 0x48, 0x4D),
        Color32::from_rgb(0xE8, 0xA8, 0x2E),
        Color32::from_rgb(0x4F, 0x9F, 0xE0),
        Color32::from_rgb(0xC9, 0x7B, 0xD6),
    );
    let (left_c, right_c) = if xbox {
        dpad(98.0, 104.0);
        // the guide sits in a ring lit green
        painter.circle_stroke(
            at(130.0, 39.0),
            13.0 * scale,
            Stroke::new(
                2.4 * scale,
                Color32::from_rgb(0x59, 0xB3, 0x3C).gamma_multiply(0.55),
            ),
        );
        part(
            "Back",
            106.0,
            52.0,
            14.0,
            9.0,
            4.0,
            Some("◀"),
            5.0,
            None,
            KEY,
        );
        part(
            "Start",
            140.0,
            52.0,
            14.0,
            9.0,
            4.0,
            Some("▶"),
            5.0,
            None,
            KEY,
        );
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
            PALE_KEY,
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
            KEY,
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
            KEY,
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
            KEY,
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
            KEY,
        );
        ((62.0, 56.0), (162.0, 104.0))
    } else {
        dpad(62.0, 66.0);
        // touchpad: dark glass with a sheen
        part(
            "Touchpad",
            100.0,
            28.0,
            60.0,
            26.0,
            6.0,
            None,
            0.0,
            None,
            Color32::from_rgb(0x26, 0x2A, 0x33),
        );
        part("Create", 86.0, 32.0, 6.0, 12.0, 3.0, None, 0.0, None, KEY);
        part("Options", 168.0, 32.0, 6.0, 12.0, 3.0, None, 0.0, None, KEY);
        part(
            "Ps",
            123.0,
            66.0,
            14.0,
            14.0,
            7.0,
            Some("PS"),
            5.0,
            None,
            KEY,
        );
        part("Mute", 124.0, 86.0, 12.0, 5.0, 2.0, None, 0.0, None, KEY);
        part(
            "Triangle",
            193.0,
            42.0,
            18.0,
            18.0,
            9.0,
            Some("△"),
            10.0,
            Some(green),
            KEY,
        );
        part(
            "Circle",
            211.0,
            60.0,
            18.0,
            18.0,
            9.0,
            Some("○"),
            10.0,
            Some(red),
            KEY,
        );
        part(
            "Cross",
            193.0,
            78.0,
            18.0,
            18.0,
            9.0,
            Some("✕"),
            10.0,
            Some(Color32::from_rgb(0x5B, 0x8D, 0xEF)),
            KEY,
        );
        part(
            "Square",
            175.0,
            60.0,
            18.0,
            18.0,
            9.0,
            Some("□"),
            10.0,
            Some(violet),
            KEY,
        );
        ((98.0, 104.0), (162.0, 104.0))
    };

    // sticks: a recessed well and a rubber cap that follows the input; the rim lights on L3/R3
    for (key, (cx, cy), (sx, sy)) in [
        (if xbox { "LeftThumb" } else { "L3" }, left_c, input.left),
        (if xbox { "RightThumb" } else { "R3" }, right_c, input.right),
    ] {
        let well = at(cx, cy);
        let pressed = (input.pressed)(key);
        painter.circle_filled(well, 21.5 * scale, Color32::from_black_alpha(46));
        painter.circle(
            well,
            19.0 * scale,
            shade(KEY, 0.8),
            Stroke::new(
                if pressed { 2.0 } else { 1.0 },
                if pressed { ACCENT } else { KEY_EDGE },
            ),
        );
        // +-9px of travel
        let dx = (f32::from(sx) - 128.0) / 128.0 * 9.0;
        let dy = (f32::from(sy) - 128.0) / 128.0 * 9.0;
        let cap = at(cx + dx, cy + dy);
        painter.circle_filled(
            cap + Vec2::new(0.0, 1.5 * scale),
            12.5 * scale,
            Color32::from_black_alpha(70),
        );
        painter.circle(cap, 12.0 * scale, DARK, Stroke::new(1.0, KEY_EDGE));
        for r in [9.0, 6.0] {
            painter.circle_stroke(
                cap,
                r * scale,
                Stroke::new(0.7, Color32::from_white_alpha(26)),
            );
        }
        painter.circle_filled(
            cap + Vec2::new(-3.5, -4.5) * scale,
            2.6 * scale,
            Color32::from_white_alpha(30),
        );
        ui.interact(
            Rect::from_center_size(well, Vec2::splat(38.0 * scale)),
            ui.scope_id().with((xbox, key)),
            Sense::hover(),
        )
        .on_hover_text(spaced(key));
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
