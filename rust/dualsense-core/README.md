# dualsense-core

Rust port of `DualSenseEmu.Core`: report parser, DualSense to Xbox 360 mapper, output reports (rumble, lightbar, player LEDs) and `MapProfile`. Phase 1 of an all-Rust rewrite; the app still runs the C# code.

- Check: `cargo test` replays `tests/golden/*.jsonl`, what the C# returned (25,192 cases).
- Re-record after changing the C#: build `src/DualSenseEmu.Core` in Release, then `dotnet fsi rust/dualsense-core/gen-golden.fsx` and `gen-numeric.fsx`.
- Known gap: profile enum names are case-sensitive here; the C# also accepts e.g. `battery`.
