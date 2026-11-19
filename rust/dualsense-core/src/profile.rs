//! User tuning for the DualSense to Xbox translation. Defaults reproduce the plain 1:1 mapping.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum LightbarMode {
    #[default]
    Static,
    Battery,
    Charging,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlayerLedMode {
    #[default]
    Leave,
    Player,
    Battery,
    Charging,
}

/// One DualSense button, named as in `profile.json`. Order matches the bit positions of `state::ds`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum DsButton {
    Square,
    Cross,
    Circle,
    Triangle,
    L1,
    R1,
    Create,
    Options,
    L3,
    R3,
    Ps,
    Touchpad,
    Mute,
    DpadUp,
    DpadDown,
    DpadLeft,
    DpadRight,
}

impl DsButton {
    pub fn bit(self) -> u32 {
        1 << (self as u32)
    }
}

/// One Xbox 360 button, or `None` to disable the source button.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum X360Button {
    None,
    DpadUp,
    DpadDown,
    DpadLeft,
    DpadRight,
    Start,
    Back,
    LeftThumb,
    RightThumb,
    LeftShoulder,
    RightShoulder,
    Guide,
    A,
    B,
    X,
    Y,
}

impl X360Button {
    pub fn bits(self) -> u16 {
        use crate::state::x360 as x;
        match self {
            Self::None => x::NONE,
            Self::DpadUp => x::DPAD_UP,
            Self::DpadDown => x::DPAD_DOWN,
            Self::DpadLeft => x::DPAD_LEFT,
            Self::DpadRight => x::DPAD_RIGHT,
            Self::Start => x::START,
            Self::Back => x::BACK,
            Self::LeftThumb => x::LEFT_THUMB,
            Self::RightThumb => x::RIGHT_THUMB,
            Self::LeftShoulder => x::LEFT_SHOULDER,
            Self::RightShoulder => x::RIGHT_SHOULDER,
            Self::Guide => x::GUIDE,
            Self::A => x::A,
            Self::B => x::B,
            Self::X => x::X,
            Self::Y => x::Y,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase", default)]
pub struct MapProfile {
    /// Radial deadzone, 0-0.5 of stick travel.
    pub left_deadzone: f64,
    pub right_deadzone: f64,
    /// Response curve exponent: 1 = linear, above 1 = finer control near centre.
    pub curve: f64,
    pub invert_left_y: bool,
    pub invert_right_y: bool,
    pub swap_sticks: bool,
    /// Triggers read 0 below this (0-254); the rest is rescaled to the full range.
    pub trigger_threshold: u8,
    /// Lightbar colour as "#RRGGBB"; None leaves the pad's own colour.
    pub lightbar: Option<String>,
    pub lightbar_mode: LightbarMode,
    pub player_mode: PlayerLedMode,
    /// Which player (1-4) the LEDs show in Player mode.
    pub player_number: u8,
    /// Overrides of the default button map; a button mapped to `None` is disabled.
    pub remap: Option<BTreeMap<DsButton, X360Button>>,
}

impl Default for MapProfile {
    fn default() -> Self {
        Self {
            left_deadzone: 0.0,
            right_deadzone: 0.0,
            curve: 1.0,
            invert_left_y: false,
            invert_right_y: false,
            swap_sticks: false,
            trigger_threshold: 0,
            lightbar: None,
            lightbar_mode: LightbarMode::Static,
            player_mode: PlayerLedMode::Leave,
            player_number: 0,
            remap: None,
        }
    }
}

impl MapProfile {
    /// Unreadable JSON gives the defaults: a hand-edited typo must not stop the pad working.
    // shortcut: enum names are case-sensitive here, the C# also accepts "battery"; add serde aliases if a user hits it.
    pub fn from_json(text: &str) -> Self {
        serde_json::from_str(text).unwrap_or_default()
    }

    pub fn default_path() -> PathBuf {
        let appdata = std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_default();
        appdata.join("DualSenseEmu").join("profile.json")
    }

    /// A missing or unreadable file gives the defaults.
    pub fn load(path: &Path) -> Self {
        std::fs::read_to_string(path)
            .map(|t| Self::from_json(&t))
            .unwrap_or_default()
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(
            path,
            serde_json::to_string_pretty(self).map_err(std::io::Error::other)?,
        )
    }
}
