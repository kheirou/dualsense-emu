# x360

Rust port of the numeric core of `X360Mapper` (Axis, Trigger, Stick), exported over a C ABI.
Not wired into the app yet: the C# code is still the one that runs.

- Check: `cargo test` replays `tests/golden.jsonl`, what the C# returned (11,010 cases).
- Re-record after changing the C#: build `src/DualSenseEmu.Core` in Release, then `dotnet fsi rust/x360/gen-golden.fsx`.
