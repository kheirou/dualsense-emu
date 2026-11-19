# dualsense-device

Rust port of `DualSenseEmu.Device`: HID reader + ViGEm virtual Xbox 360 pad (`service`), HidHide CLI driver (`hidhide`), `status.json` writer (`status_file`). Phase 2 of the all-Rust rewrite; the app still runs the C# code.

- Checked by `cargo test`: `hidhide` and `status_file` replay `tests/golden/*.jsonl` (what the C# returned). Re-record with `dotnet fsi rust/dualsense-device/gen-golden.fsx` after building `src/DualSenseEmu.Device` in Release.
- NOT checked by tests: `service` (needs a DualSense and ViGEmBus). Smoke test with a pad connected: `cargo run -p dualsense-device --example monitor`.
- Difference: when `HidHideCLI --dev-gaming` prints JSON that is not an array, the C# throws; the Rust returns no pads.
- Difference: rumble and `refresh_output` are applied on the next pad report (a flag read by the reader thread), not from the calling thread.
