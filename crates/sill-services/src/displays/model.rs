//! The displays service's state, its commands and its one reducer.

use serde::{Deserialize, Serialize};

use super::brightness::{Backlight, BrightnessTarget, LevelChange, percent_of, route};
use super::ddc::{DdcDisplay, DdcId, Feature};
use super::output::DisplayOutput;
use super::policy::DisplayPolicy;
use crate::cosmic_wl::OutputName;
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

/// `displays` after `event`: each source replaces its own part.
pub fn reduce_displays(displays: Displays, event: DisplaysEvent) -> Displays {
    match event {
        DisplaysEvent::Outputs(outputs) => Displays {
            outputs,
            ..displays
        },
        DisplaysEvent::Ddc(ddc) => Displays { ddc, ..displays },
        DisplaysEvent::Backlights(backlights) => Displays {
            backlights,
            ..displays
        },
    }
}

#[cfg(test)]
mod tests {
    use super::{Displays, DisplaysEvent, reduce_displays};
    use crate::cosmic_wl::OutputName;
    use crate::displays::brightness::{Backlight, BacklightKind, BacklightName, BrightnessTarget};
    use crate::displays::ddc::{Controls, DdcDisplay, DdcId, Endpoint, Level};
    use crate::units::Percent;

    fn monitor() -> DdcDisplay {
        DdcDisplay {
            endpoint: Endpoint {
                id: DdcId(5),
                connector: Some(OutputName("HDMI-A-2".to_owned())),
                product: "BenQ MA270U".to_owned(),
            },
            controls: Controls {
                brightness: Some(Level {
                    value: 49,
                    max: 100,
                }),
                ..Controls::default()
            },
        }
    }

    fn panel() -> Backlight {
        Backlight {
            name: BacklightName("intel_backlight".to_owned()),
            kind: BacklightKind::Raw,
            value: 9600,
            max: 19200,
        }
    }

    #[test]
    fn each_source_replaces_its_own_part() {
        let with_ddc = reduce_displays(Displays::default(), DisplaysEvent::Ddc(vec![monitor()]));
        let both = reduce_displays(with_ddc.clone(), DisplaysEvent::Backlights(vec![panel()]));
        assert_eq!(both.ddc, with_ddc.ddc);
        assert_eq!(both.backlights, vec![panel()]);
        let cleared = reduce_displays(both, DisplaysEvent::Ddc(Vec::new()));
        assert_eq!(cleared.ddc, vec![]);
        assert_eq!(cleared.backlights, vec![panel()]);
    }

    #[test]
    fn targets_and_levels_follow_the_outputs() {
        let displays = Displays {
            ddc: vec![monitor()],
            backlights: vec![panel()],
            ..Displays::default()
        };
        let name = |n: &str| OutputName(n.to_owned());
        let hdmi = displays.target(&name("HDMI-A-2"));
        let panel_target = displays.target(&name("eDP-1"));
        assert_eq!(hdmi, BrightnessTarget::Ddc(DdcId(5)));
        assert_eq!(
            panel_target,
            BrightnessTarget::Backlight(BacklightName("intel_backlight".to_owned()))
        );
        assert_eq!(displays.target(&name("DP-1")), BrightnessTarget::None);
        assert_eq!(displays.brightness(&hdmi), Some(Percent(49)));
        assert_eq!(displays.brightness(&panel_target), Some(Percent(50)));
        assert_eq!(displays.brightness(&BrightnessTarget::None), None);
    }
}
