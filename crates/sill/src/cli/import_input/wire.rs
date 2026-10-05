//! cosmic-comp's input settings, as its config file spells them: `input_devices` is a RON map from
//! a device's name to an [`InputConfig`], and `input_default` is one [`InputConfig`]. This
//! re-declares the fields sill reads or writes with the serde attributes cosmic-comp's
//! `cosmic-comp-config/src/input.rs` uses (fields without a default must be present in the file;
//! the rest may be left out). A file that does not parse is ignored by the compositor.

use serde::{Deserialize, Serialize};

/// Whether a device sends events at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeviceState {
    Enabled,
    Disabled,
    DisabledOnExternalMouse,
}

/// libinput's pointer acceleration profiles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AccelProfile {
    Flat,
    Adaptive,
}

/// Pointer acceleration: the profile (`None` leaves it to libinput) and the speed, -1 to 1.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AccelConfig {
    pub profile: Option<AccelProfile>,
    pub speed: f64,
}

/// How a device scrolls (`None` leaves each to libinput's default).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ScrollConfig {
    pub method: Option<String>,
    pub natural_scroll: Option<bool>,
    pub scroll_button: Option<u32>,
    pub scroll_factor: Option<f64>,
}

/// One device's settings (the fields sill knows; cosmic-comp has more).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InputConfig {
    pub state: DeviceState,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub acceleration: Option<AccelConfig>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub scroll_config: Option<ScrollConfig>,
}
