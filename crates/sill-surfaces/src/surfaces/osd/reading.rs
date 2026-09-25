//! What one output's indicator draws for a change: the level (the default sink's volume, or the
//! brightness of the display this output drives) and the glyph for it. Pure.
//!
//! A monitor's own speakers follow the software volume (`display.hardware_volume`), so the
//! volume shown is the sink's level whichever of the two is playing.

use ds::{Fraction, Icon};
use sill_services::{Displays, Mute, OutputName, Percent, Presence, Volume};

use crate::surfaces::osd::link::{Level, LevelSource, OsdKind};

/// A change's level on one output, if there is one to show.
pub fn level_of(
    kind: OsdKind,
    source: LevelSource,
    volume: &Volume,
    displays: &Displays,
    output: Option<&OutputName>,
) -> Option<Level> {
    match (source, kind) {
        (LevelSource::Fixed(level), _) => Some(level),
        (LevelSource::Live, OsdKind::Volume) => match volume.presence {
            Presence::Present => Some(Level {
                percent: volume.level,
                mute: volume.mute,
            }),
            Presence::Absent => None,
        },
        (LevelSource::Live, OsdKind::Brightness) => {
            let percent = displays.brightness(&displays.target(output?))?;
            Some(Level {
                percent,
                mute: Mute::Audible,
            })
        }
    }
}

/// The glyph (Lucide `volume*` and `sun`, design/08 §2): muted, silent or no sink is the
/// crossed speaker, then one, two or three waves by thirds (the bar's own thresholds);
/// brightness is always the sun.
pub fn glyph(kind: OsdKind, level: Option<Level>) -> Icon {
    match (kind, level) {
        (OsdKind::Brightness, _) => Icon::Sun,
        (OsdKind::Volume, None) => Icon::VolumeX,
        (OsdKind::Volume, Some(level)) => match (level.mute, level.percent.0) {
            (Mute::Muted, _) | (_, 0) => Icon::VolumeX,
            (_, 1..=33) => Icon::Volume,
            (_, 34..=66) => Icon::Volume1,
            _ => Icon::Volume2,
        },
    }
}

/// The level bar's fill: empty when muted or unknown, full at 100 % and above.
pub fn fill(level: Option<Level>) -> Fraction {
    match level {
        Some(Level {
            mute: Mute::Audible,
            percent: Percent(percent),
        }) => Fraction(percent.min(100) * 10),
        Some(Level {
            mute: Mute::Muted, ..
        })
        | None => Fraction(0),
    }
}

/// The level bar's accessible name.
pub fn label(kind: OsdKind) -> &'static str {
    match kind {
        OsdKind::Volume => "Volume",
        OsdKind::Brightness => "Brightness",
    }
}

#[cfg(test)]
mod tests {
    use super::{fill, glyph, level_of};
    use crate::surfaces::osd::link::{Level, LevelSource, OsdKind};
    use ds::{Fraction, Icon};
    use sill_services::displays::ddc::Endpoint;
    use sill_services::displays::{Controls, DdcDisplay, DdcId, Level as DdcLevel};
    use sill_services::{Displays, Mute, OutputName, Percent, Presence, Volume};

    fn level(percent: u16, mute: Mute) -> Option<Level> {
        Some(Level {
            percent: Percent(percent),
            mute,
        })
    }

    /// A level as the tables spell it: percent and mute, or none.
    type At = Option<(u16, Mute)>;

    #[test]
    fn the_glyph_follows_the_level() {
        const CASES: &[(OsdKind, At, Icon)] = &[
            (OsdKind::Volume, None, Icon::VolumeX),
            (OsdKind::Volume, Some((0, Mute::Audible)), Icon::VolumeX),
            (OsdKind::Volume, Some((70, Mute::Muted)), Icon::VolumeX),
            (OsdKind::Volume, Some((1, Mute::Audible)), Icon::Volume),
            (OsdKind::Volume, Some((33, Mute::Audible)), Icon::Volume),
            (OsdKind::Volume, Some((34, Mute::Audible)), Icon::Volume1),
            (OsdKind::Volume, Some((66, Mute::Audible)), Icon::Volume1),
            (OsdKind::Volume, Some((67, Mute::Audible)), Icon::Volume2),
            (OsdKind::Volume, Some((150, Mute::Audible)), Icon::Volume2),
            (OsdKind::Brightness, Some((0, Mute::Audible)), Icon::Sun),
            (OsdKind::Brightness, None, Icon::Sun),
        ];
        for &(kind, at, want) in CASES {
            let at = at.and_then(|(p, m)| level(p, m));
            assert_eq!(glyph(kind, at), want, "{kind:?} {at:?}");
        }
    }

    #[test]
    fn the_bar_fills_by_level_and_empties_when_muted() {
        const CASES: &[(At, u16)] = &[
            (None, 0),
            (Some((0, Mute::Audible)), 0),
            (Some((45, Mute::Audible)), 450),
            (Some((100, Mute::Audible)), 1000),
            (Some((130, Mute::Audible)), 1000),
            (Some((45, Mute::Muted)), 0),
        ];
        for &(at, want) in CASES {
            let at = at.and_then(|(p, m)| level(p, m));
            assert_eq!(fill(at), Fraction(want), "{at:?}");
        }
    }

    #[test]
    fn a_live_level_comes_from_its_service_and_a_fixed_one_from_the_change() {
        let volume = Volume {
            presence: Presence::Present,
            level: Percent(40),
            mute: Mute::Muted,
            sink: "Headphones".to_owned(),
        };
        let displays = Displays {
            ddc: vec![DdcDisplay {
                endpoint: Endpoint {
                    id: DdcId(5),
                    connector: Some(OutputName("HDMI-A-2".to_owned())),
                    product: "Monitor".to_owned(),
                },
                controls: Controls {
                    brightness: Some(DdcLevel {
                        value: 49,
                        max: 100,
                    }),
                    ..Controls::default()
                },
            }],
            ..Displays::default()
        };
        let hdmi = OutputName("HDMI-A-2".to_owned());
        let other = OutputName("DP-1".to_owned());
        let live = LevelSource::Live;
        let fixed = LevelSource::Fixed(Level {
            percent: Percent(70),
            mute: Mute::Audible,
        });
        let cases = [
            (
                "volume",
                OsdKind::Volume,
                live,
                None,
                level(40, Mute::Muted),
            ),
            ("no sink", OsdKind::Volume, live, None, None),
            (
                "the monitor's brightness",
                OsdKind::Brightness,
                live,
                Some(&hdmi),
                level(49, Mute::Audible),
            ),
            (
                "a display with none",
                OsdKind::Brightness,
                live,
                Some(&other),
                None,
            ),
            ("an unnamed output", OsdKind::Brightness, live, None, None),
            (
                "faked",
                OsdKind::Brightness,
                fixed,
                Some(&other),
                level(70, Mute::Audible),
            ),
        ];
        for (name, kind, source, output, want) in cases {
            let volume = match name {
                "no sink" => Volume::default(),
                _ => volume.clone(),
            };
            assert_eq!(
                level_of(kind, source, &volume, &displays, output),
                want,
                "{name}"
            );
        }
    }
}
