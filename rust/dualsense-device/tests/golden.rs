//! Replays what the C# Device returned (tests/golden/*.jsonl, made by gen-golden.fsx).

use dualsense_core::state::{ChargeState, DualSenseState};
use dualsense_device::hidhide::{build_chain, cli_path, parse_dualsense_paths};
use dualsense_device::service::{EmuState, Transport};
use dualsense_device::status_file::to_json;
use serde_json::Value;

fn cases(name: &str) -> Vec<Value> {
    let path = format!("{}/tests/golden/{name}.jsonl", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(path)
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}

fn strings(v: &Value) -> Vec<String> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|s| s.as_str().unwrap().to_string())
        .collect()
}

#[test]
fn hidhide() {
    let all = cases("hidhide");
    assert_eq!(all.len(), 40);
    for c in all {
        match c["fn"].as_str().unwrap() {
            "parse" => assert_eq!(
                parse_dualsense_paths(c["json"].as_str().unwrap()),
                strings(&c["out"]),
                "{c}"
            ),
            "cli_path" => assert_eq!(cli_path().to_str().unwrap(), c["out"].as_str().unwrap()),
            "chain" => {
                let got = build_chain(
                    c["hide"].as_bool().unwrap(),
                    &strings(&c["pads"]),
                    c["app"].as_str().unwrap(),
                );
                match c["error"].as_bool().unwrap() {
                    true => assert!(got.is_err(), "{c}"),
                    false => assert_eq!(got.unwrap(), c["out"].as_str().unwrap(), "{c}"),
                }
            }
            other => panic!("unknown fn {other}"),
        }
    }
}

#[test]
fn status_json() {
    let all = cases("status");
    assert_eq!(all.len(), 240);
    for c in all {
        let state = match c["state"].as_str().unwrap() {
            "Stopped" => EmuState::Stopped,
            "Waiting" => EmuState::Waiting,
            "Running" => EmuState::Running,
            "Error" => EmuState::Error,
            _ => EmuState::DriverMissing,
        };
        let transport = match c["transport"].as_str().unwrap() {
            "Usb" => Transport::Usb,
            "Bluetooth" => Transport::Bluetooth,
            _ => Transport::None,
        };
        let ds = c["battery"].as_i64().map(|b| DualSenseState {
            battery_percent: b as i32,
            charge: match c["charge"].as_str().unwrap() {
                "Discharging" => ChargeState::Discharging,
                "Charging" => ChargeState::Charging,
                "Full" => ChargeState::Full,
                _ => ChargeState::Unknown,
            },
            ..DualSenseState::new(128, 128, 128, 128, 0, 0, 0)
        });
        assert_eq!(
            to_json(state, transport, ds.as_ref()),
            c["out"].as_str().unwrap(),
            "{c}"
        );
    }
}
