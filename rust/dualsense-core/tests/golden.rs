//! Replays what the C# Core returned (tests/golden/*.jsonl, made by gen-golden.fsx and gen-numeric.fsx).

use dualsense_core::mapper::{axis, default_for, map, stick, trigger};
use dualsense_core::output::{build, crc32, parse_colour, resolve, resolve_leds};
use dualsense_core::parser::try_parse;
use dualsense_core::profile::{DsButton, MapProfile, X360Button};
use dualsense_core::state::{ChargeState, DualSenseState};
use serde_json::Value;

fn cases(name: &str) -> Vec<Value> {
    let path = format!("{}/tests/golden/{name}.jsonl", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(path)
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}

fn i(c: &Value, k: &str) -> i64 {
    c[k].as_i64().unwrap_or_else(|| panic!("{k} in {c}"))
}

fn unhex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|n| u8::from_str_radix(&s[n..n + 2], 16).unwrap())
        .collect()
}

fn charge(s: &str) -> ChargeState {
    match s {
        "Discharging" => ChargeState::Discharging,
        "Charging" => ChargeState::Charging,
        "Full" => ChargeState::Full,
        _ => ChargeState::Unknown,
    }
}

fn rgb(v: &Value) -> Option<(u8, u8, u8)> {
    v.as_array().map(|a| {
        (
            a[0].as_u64().unwrap() as u8,
            a[1].as_u64().unwrap() as u8,
            a[2].as_u64().unwrap() as u8,
        )
    })
}

#[test]
fn numeric() {
    let all = cases("numeric");
    assert_eq!(all.len(), 11010);
    for c in all {
        let b = |k: &str| c[k].as_bool().unwrap();
        match c["fn"].as_str().unwrap() {
            "axis" => assert_eq!(
                i64::from(axis(i(&c, "v") as u8, b("invert"))),
                i(&c, "out"),
                "{c}"
            ),
            "trigger" => assert_eq!(
                i64::from(trigger(i(&c, "v") as u8, i(&c, "t") as u8)),
                i(&c, "out"),
                "{c}"
            ),
            "stick" => {
                let (ox, oy) = stick(
                    i(&c, "x") as u8,
                    i(&c, "y") as u8,
                    c["dz"].as_f64().unwrap(),
                    c["curve"].as_f64().unwrap(),
                    b("invert"),
                );
                assert_eq!(
                    (i64::from(ox), i64::from(oy)),
                    (i(&c, "ox"), i(&c, "oy")),
                    "{c}"
                );
            }
            other => panic!("unknown fn {other}"),
        }
    }
}

#[test]
fn parser() {
    let all = cases("parser");
    assert_eq!(all.len(), 9663);
    for c in all {
        let got = try_parse(&unhex(c["report"].as_str().unwrap()));
        assert_eq!(got.is_some(), c["ok"].as_bool().unwrap(), "{c}");
        if let Some(s) = got {
            let want = DualSenseState {
                lx: i(&c, "lx") as u8,
                ly: i(&c, "ly") as u8,
                rx: i(&c, "rx") as u8,
                ry: i(&c, "ry") as u8,
                l2: i(&c, "l2") as u8,
                r2: i(&c, "r2") as u8,
                buttons: i(&c, "buttons") as u32,
                battery_percent: i(&c, "battery") as i32,
                charge: charge(c["charge"].as_str().unwrap()),
            };
            assert_eq!(s, want, "{c}");
        }
    }
}

#[test]
fn profile() {
    let all = cases("profile");
    assert_eq!(all.len(), 13);
    for c in all {
        let p = MapProfile::from_json(c["json"].as_str().unwrap());
        assert_eq!(p.left_deadzone, c["left_deadzone"].as_f64().unwrap(), "{c}");
        assert_eq!(
            p.right_deadzone,
            c["right_deadzone"].as_f64().unwrap(),
            "{c}"
        );
        assert_eq!(p.curve, c["curve"].as_f64().unwrap(), "{c}");
        assert_eq!(
            p.invert_left_y,
            c["invert_left_y"].as_bool().unwrap(),
            "{c}"
        );
        assert_eq!(
            p.invert_right_y,
            c["invert_right_y"].as_bool().unwrap(),
            "{c}"
        );
        assert_eq!(p.swap_sticks, c["swap_sticks"].as_bool().unwrap(), "{c}");
        assert_eq!(
            i64::from(p.trigger_threshold),
            i(&c, "trigger_threshold"),
            "{c}"
        );
        assert_eq!(p.lightbar.as_deref(), c["lightbar"].as_str(), "{c}");
        assert_eq!(
            format!("{:?}", p.lightbar_mode),
            c["lightbar_mode"].as_str().unwrap(),
            "{c}"
        );
        assert_eq!(
            format!("{:?}", p.player_mode),
            c["player_mode"].as_str().unwrap(),
            "{c}"
        );
        assert_eq!(i64::from(p.player_number), i(&c, "player_number"), "{c}");
        let mut got: Option<Vec<(String, String)>> = p.remap.map(|r| {
            r.iter()
                .map(|(k, v)| (format!("{k:?}"), format!("{v:?}")))
                .collect()
        });
        let mut want: Option<Vec<(String, String)>> = c["remap"].as_object().map(|o| {
            o.iter()
                .map(|(k, v)| (k.clone(), v.as_str().unwrap().to_string()))
                .collect()
        });
        got.iter_mut().for_each(|v| v.sort());
        want.iter_mut().for_each(|v| v.sort());
        assert_eq!(got, want, "{c}");
    }
}

#[test]
fn map_cases() {
    let all = cases("map");
    assert_eq!(all.len(), 3348);
    for c in all {
        if c.get("default_for").is_some() {
            let bit = i(&c, "default_for") as u32;
            let all_buttons = [
                DsButton::Square,
                DsButton::Cross,
                DsButton::Circle,
                DsButton::Triangle,
                DsButton::L1,
                DsButton::R1,
                DsButton::Create,
                DsButton::Options,
                DsButton::L3,
                DsButton::R3,
                DsButton::Ps,
                DsButton::Touchpad,
                DsButton::Mute,
                DsButton::DpadUp,
                DsButton::DpadDown,
                DsButton::DpadLeft,
                DsButton::DpadRight,
            ];
            let got = all_buttons
                .iter()
                .find(|b| b.bit() == bit)
                .and_then(|b| default_for(*b))
                .map(X360Button::bits);
            assert_eq!(got.map(i64::from), c["out"].as_i64(), "{c}");
            continue;
        }
        let p = MapProfile::from_json(c["profile"].as_str().unwrap());
        let s = DualSenseState::new(
            i(&c, "lx") as u8,
            i(&c, "ly") as u8,
            i(&c, "rx") as u8,
            i(&c, "ry") as u8,
            i(&c, "l2") as u8,
            i(&c, "r2") as u8,
            i(&c, "buttons") as u32,
        );
        let x = map(&s, &p);
        let got = [
            x.buttons as i64,
            x.left_trigger.into(),
            x.right_trigger.into(),
            x.left_x.into(),
            x.left_y.into(),
            x.right_x.into(),
            x.right_y.into(),
        ];
        let want = ["o_buttons", "o_lt", "o_rt", "o_lx", "o_ly", "o_rx", "o_ry"].map(|k| i(&c, k));
        assert_eq!(got, want, "{c}");
    }
}

#[test]
fn output() {
    let all = cases("output");
    assert_eq!(all.len(), 1158);
    for c in all {
        match c["fn"].as_str().unwrap() {
            "build" => {
                let got = build(
                    c["bt"].as_bool().unwrap(),
                    i(&c, "left") as u8,
                    i(&c, "right") as u8,
                    rgb(&c["rgb"]),
                    i(&c, "seq") as u8,
                    c["leds"].as_u64().map(|v| v as u8),
                );
                assert_eq!(got, unhex(c["out"].as_str().unwrap()), "{c}");
            }
            "crc32" => assert_eq!(
                i64::from(crc32(&unhex(c["body"].as_str().unwrap())) as i32),
                i(&c, "out"),
                "{c}"
            ),
            "colour" => assert_eq!(parse_colour(c["hex"].as_str()), rgb(&c["out"]), "{c}"),
            "resolve" => {
                let p = MapProfile::from_json(c["profile"].as_str().unwrap());
                let s = c["battery"].as_i64().map(|b| DualSenseState {
                    battery_percent: b as i32,
                    charge: charge(c["charge"].as_str().unwrap()),
                    ..DualSenseState::new(128, 128, 128, 128, 0, 0, 0)
                });
                assert_eq!(resolve(&p, s.as_ref()), rgb(&c["colour"]), "{c}");
                assert_eq!(
                    resolve_leds(&p, s.as_ref()).map(i64::from),
                    c["leds"].as_i64(),
                    "{c}"
                );
            }
            other => panic!("unknown fn {other}"),
        }
    }
}
