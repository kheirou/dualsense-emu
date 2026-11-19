use serde_json::Value;
use x360::{axis, stick, trigger};

#[test]
fn matches_recorded_csharp() {
    let text = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/golden.jsonl"))
        .unwrap();
    let mut n = 0;
    for line in text.lines() {
        let c: Value = serde_json::from_str(line).unwrap();
        let i = |k: &str| c[k].as_i64().unwrap();
        let f = |k: &str| c[k].as_f64().unwrap();
        let b = |k: &str| c[k].as_bool().unwrap();
        match c["fn"].as_str().unwrap() {
            "axis" => assert_eq!(
                i64::from(axis(i("v") as u8, b("invert"))),
                i("out"),
                "{line}"
            ),
            "trigger" => assert_eq!(
                i64::from(trigger(i("v") as u8, i("t") as u8)),
                i("out"),
                "{line}"
            ),
            "stick" => {
                let (ox, oy) = stick(i("x") as u8, i("y") as u8, f("dz"), f("curve"), b("invert"));
                assert_eq!((i64::from(ox), i64::from(oy)), (i("ox"), i("oy")), "{line}");
            }
            other => panic!("unknown fn {other}"),
        }
        n += 1;
    }
    assert_eq!(n, 11010);
}
