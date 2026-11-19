//! Rust port of DualSenseEmu.Device: HID reader, ViGEm virtual pad, HidHide, status file.
//! `hidhide` and `status_file` are checked against recorded C# output (tests/golden); the HID/ViGEm service needs a pad.

pub mod hidhide;
pub mod service;
pub mod status_file;
