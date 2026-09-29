//! The displays service's state.

use serde::{Deserialize, Serialize};

use super::brightness::{Backlight, BrightnessTarget, percent_of, route};
use super::ddc::DdcDisplay;
use super::output::DisplayOutput;
use crate::ids::{DdcId, OutputName};
use crate::units::Percent;

/// Every display, as the control center, the OSD and the settings read them.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Displays {
    /// The compositor's outputs, described (after any scale sill just set).
    pub outputs: Vec<DisplayOutput>,
    /// The monitors that answer DDC/CI, with their cached controls.
    pub ddc: Vec<DdcDisplay>,
    /// The built-in panels' backlights.
    pub backlights: Vec<Backlight>,
}

impl Displays {
    /// What a brightness change on output `name` drives.
    pub fn target(&self, name: &OutputName) -> BrightnessTarget {
        let placement = self
            .outputs
            .iter()
            .find(|o| &o.name == name)
            .map(|o| o.placement)
            .unwrap_or_else(|| super::drm::placement(name));
        let ddc: Vec<(DdcId, Option<OutputName>)> = self
            .ddc
            .iter()
            .filter(|d| d.controls.brightness.is_some())
            .map(|d| (d.endpoint.id, d.endpoint.connector.clone()))
            .collect();
        route(name, placement, &self.backlights, &ddc)
    }

    /// The brightness `target` is at now (the OSD's level), if it has one.
    pub fn brightness(&self, target: &BrightnessTarget) -> Option<Percent> {
        match target {
            BrightnessTarget::Backlight(name) => self
                .backlights
                .iter()
                .find(|b| &b.name == name)
                .map(|b| percent_of(b.value, b.max)),
            BrightnessTarget::Ddc(id) => self
                .ddc
                .iter()
                .find(|d| d.endpoint.id == *id)
                .and_then(|d| d.controls.brightness)
                .map(|l| percent_of(u32::from(l.value), u32::from(l.max))),
            BrightnessTarget::None => None,
        }
    }
}

/// Whether a view of the brightness opened or closed (`DisplaysCommand::Follow`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Following {
    Start,
    Stop,
}

/// How often the brightness is read again while a view follows it.
pub const FOLLOW_EVERY_MS: u64 = 2000;
