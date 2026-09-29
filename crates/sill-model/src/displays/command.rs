//! What the displays service hears and what it can be told.

use serde::{Deserialize, Serialize};

use super::Following;
use super::brightness::{Backlight, LevelChange};
use super::ddc::{DdcDisplay, Feature};
use super::output::DisplayOutput;
use super::policy::DisplayPolicy;
use crate::ids::{DdcId, OutputName};
use crate::units::Percent;

/// What the displays service can be told. Nothing happens until the first `Policy` arrives:
/// the service only acts for a running daemon that has read its settings, never merely because
/// `Services::spawn` ran (a test, a headless render).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum DisplaysCommand {
    /// The `display.*` settings, at start and on every change.
    Policy(DisplayPolicy),
    /// The compositor's outputs changed: re-describe them, apply a first-sight scale, re-probe DDC.
    Rescan,
    /// A brightness key or slider. `at` is the output under the pointer (else the focused one),
    /// used when `display.brightness_keys_target` is `pointer_output`.
    Brightness {
        at: Option<OutputName>,
        change: LevelChange,
    },
    /// The control center's brightness capsule on `output`: that output's own target is set to
    /// `level`, whatever `display.brightness_keys_target` says and whether or not the outputs
    /// pass has listed `output` yet (the capsule shows `Displays::target(output)`, so this is
    /// exactly the monitor it shows).
    SetBrightness { output: OutputName, level: Percent },
    /// A view of the brightness opened (`Start`) or closed (`Stop`). While one is open, the
    /// monitors' brightness is read again every [`FOLLOW_EVERY_MS`], so a change made on the
    /// monitor itself or by another program shows (a read is skipped while sill's own write is
    /// pending).
    Follow(Following),
    /// Set one DDC feature to a raw value (the control center's contrast, input, ... sliders).
    SetFeature {
        display: DdcId,
        feature: Feature,
        value: u16,
    },
}

/// One change the service observed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum DisplaysEvent {
    Outputs(Vec<DisplayOutput>),
    Ddc(Vec<DdcDisplay>),
    Backlights(Vec<Backlight>),
}
