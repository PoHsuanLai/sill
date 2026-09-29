//! The kernel's view of the connected displays: `/sys/class/drm/card<N>-<connector>/`, whose
//! `status` says whether something is plugged in and whose `edid` is what it reported.

use crate::ids::OutputName;

/// One connected connector and its raw EDID (possibly empty: a panel that sends none).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Connector {
    pub name: OutputName,
    pub edid: Vec<u8>,
}

/// Whether a connector is a built-in panel.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Default, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum Placement {
    /// eDP, LVDS or DSI: a laptop's or tablet's own panel, lit by a backlight.
    Internal,
    #[default]
    External,
}

/// The connector name a sysfs entry stands for: `card1-HDMI-A-2` is `HDMI-A-2` (the name the
/// compositor gives the output too). `None` for anything that is not `card<N>-<name>`.
pub fn connector_name(entry: &str) -> Option<OutputName> {
    let rest = entry.strip_prefix("card")?;
    let (index, name) = rest.split_once('-')?;
    (!index.is_empty() && index.chars().all(|c| c.is_ascii_digit()) && !name.is_empty())
        .then(|| OutputName(name.to_owned()))
}

/// Whether `name` is a built-in panel's connector, by its type prefix (`eDP-1`, `LVDS-1`,
/// `DSI-1`), matched up to the dash.
pub fn placement(name: &OutputName) -> Placement {
    let kind = name.0.rsplit_once('-').map_or(name.0.as_str(), |(k, _)| k);
    match kind {
        "eDP" | "LVDS" | "DSI" => Placement::Internal,
        _ => Placement::External,
    }
}

#[cfg(test)]
mod tests {
    use super::{Placement, connector_name, placement};
    use crate::ids::OutputName;

    #[test]
    fn sysfs_entries_name_their_connector() {
        let cases: &[(&str, Option<&str>)] = &[
            ("card1-HDMI-A-2", Some("HDMI-A-2")),
            ("card0-eDP-1", Some("eDP-1")),
            ("card12-DP-3", Some("DP-3")),
            ("card1", None),
            ("renderD128", None),
            ("cardX-DP-1", None),
            ("version", None),
        ];
        for &(entry, want) in cases {
            assert_eq!(
                connector_name(entry),
                want.map(|n| OutputName(n.to_owned())),
                "{entry}"
            );
        }
    }

    #[test]
    fn built_in_panels_are_named_by_their_connector_type() {
        let cases: &[(&str, Placement)] = &[
            ("eDP-1", Placement::Internal),
            ("LVDS-1", Placement::Internal),
            ("DSI-1", Placement::Internal),
            ("HDMI-A-2", Placement::External),
            ("DP-1", Placement::External),
            ("WINIT-0", Placement::External),
            ("eDPX-1", Placement::External),
        ];
        for &(name, want) in cases {
            assert_eq!(placement(&OutputName(name.to_owned())), want, "{name}");
        }
    }
}
