//! Rust port of DualSenseEmu.Core. `tests/golden/*.jsonl` holds what the C# returned; `cargo test` replays it.

pub mod mapper;
pub mod output;
pub mod parser;
pub mod profile;
pub mod state;
