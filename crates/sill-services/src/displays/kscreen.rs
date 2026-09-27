//! KWin's view of its outputs, read-only, for when `cosmic-randr` has no answer (KWin has no
//! `zwlr_output_manager_v1`, FINDINGS G51 and "Displays on KWin"). `kscreen-doctor --json`
//! prints every output KScreen knows; [`parse_outputs`] turns the connected ones into the same
//! [`Head`]s `cosmic-randr` gives, so the brightness routing, the OSD and the control center see
//! the same outputs on both compositors.
//!
//! Read-only by construction: [`HeadReader`] has no way to set anything, and the only command
//! [`KscreenDoctor`] ever runs is `kscreen-doctor --json`. A pass from these heads applies no
//! scale policy (`scale_pass`), so sill never changes KDE's own scaling.

use serde::Deserialize;

use super::edid::{Millimetres, Mode, Pixels};
use super::randr::{Enablement, Head};
use super::scale::DisplayScale;
use super::sink::run_tool;
use crate::cosmic_wl::OutputName;
use crate::error::ServiceError;

/// Somewhere a compositor's outputs can be read, and nothing else.
pub trait HeadReader {
    /// Every output the compositor reports now.
    fn heads(&mut self) -> Result<Vec<Head>, ServiceError>;
}

/// `kscreen-doctor --json`, on `$PATH`, bounded like `cosmic-randr` (2 s).
#[derive(Debug, Clone, Default)]
pub struct KscreenDoctor;

impl HeadReader for KscreenDoctor {
    fn heads(&mut self) -> Result<Vec<Head>, ServiceError> {
        run_tool("kscreen-doctor", &["--json"]).and_then(|text| parse_outputs(&text))
    }
}

/// A JSON boolean, as a value with a name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(from = "bool")]
enum Flag {
    On,
    #[default]
    Off,
}

impl From<bool> for Flag {
    fn from(value: bool) -> Flag {
        match value {
            true => Flag::On,
            false => Flag::Off,
        }
    }
}

#[derive(Debug, Deserialize)]
struct Config {
    #[serde(default)]
    outputs: Vec<Output>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Output {
    name: String,
    #[serde(default)]
    connected: Flag,
    #[serde(default)]
    enabled: Flag,
    #[serde(default)]
    current_mode_id: String,
    #[serde(default)]
    modes: Vec<WireMode>,
    #[serde(default)]
    scale: f64,
    #[serde(default, rename = "sizeMM")]
    size_mm: Size,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WireMode {
    id: String,
    #[serde(default)]
    refresh_rate: f64,
    #[serde(default)]
    size: Size,
}

#[derive(Debug, Default, Deserialize)]
struct Size {
    #[serde(default)]
    width: u32,
    #[serde(default)]
    height: u32,
}

/// The connected outputs in `json`, in KScreen's order. KScreen names no make or model, so a
/// head's model is its connector name (the EDID in sysfs names the product, `output::describe`).
pub fn parse_outputs(json: &str) -> Result<Vec<Head>, ServiceError> {
    let config: Config = serde_json::from_str(json)
        .map_err(|error| ServiceError::Launch(format!("kscreen-doctor --json: {error}")))?;
    Ok(config
        .outputs
        .iter()
        .filter(|o| o.connected == Flag::On)
        .map(head)
        .collect())
}

fn head(output: &Output) -> Head {
    Head {
        name: OutputName(output.name.clone()),
        enablement: match output.enabled {
            Flag::On => Enablement::Enabled,
            Flag::Off => Enablement::Disabled,
        },
        make: String::new(),
        model: output.name.clone(),
        physical: match (output.size_mm.width, output.size_mm.height) {
            (0, _) | (_, 0) => None,
            (width, height) => Some(Millimetres { width, height }),
        },
        current: output
            .modes
            .iter()
            .find(|m| m.id == output.current_mode_id)
            .map(|m| Mode {
                pixels: Pixels {
                    width: m.size.width,
                    height: m.size.height,
                },
                refresh_mhz: thousandths(m.refresh_rate),
            }),
        scale: hundredths(output.scale).map_or(DisplayScale::ONE, DisplayScale),
    }
}

/// `59.997` Hz as `59997` mHz; 0 for anything out of range.
fn thousandths(hz: f64) -> u32 {
    let mhz = (hz * 1000.0).round();
    match mhz {
        m if (0.0..=f64::from(u32::MAX)).contains(&m) => m as u32,
        _ => 0,
    }
}

/// `1.5` as `150`; `None` for 0, a negative or an absurd scale.
fn hundredths(scale: f64) -> Option<u16> {
    let cents = (scale * 100.0).round();
    (1.0..=f64::from(u16::MAX))
        .contains(&cents)
        .then_some(cents as u16)
}

#[cfg(test)]
mod tests;
