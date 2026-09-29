//! What one output's indicator draws for a change: the level (the default sink's volume, or the
//! brightness of the display this output drives), the card's level and glyph (quire's
//! `LevelControl` draws the waves or sun itself, from the fill and the mute), and the title above
//! it: the sink's or monitor's real name where the services give one, else "Sound"/"Display".
//! Pure.
//!
//! A monitor's own speakers follow the software volume (`display.hardware_volume`), so the
//! volume shown is the sink's level whichever of the two is playing.

use ds::{Fraction, LevelGlyph, Muting};
use sill_model::ids::OutputName;
use sill_model::units::{Percent, Presence};
use sill_model::volume::{Mute, Volume};
use sill_services::{BrightnessTarget, Displays};

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

/// The level quire's card draws: the fill (empty when muted or unknown) and the glyph
/// `LevelControl` follows it with (a speaker's waves for volume, the sun for brightness).
pub fn card_level(kind: OsdKind, level: Option<Level>) -> ds::OsdLevel {
    ds::OsdLevel {
        value: fill(level),
        glyph: glyph(kind, level),
    }
}

/// The level bar's fill: empty when muted or unknown, full at 100 % and above.
fn fill(level: Option<Level>) -> Fraction {
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

/// The glyph `LevelControl` follows: volume carries its mute (no sink counts as muted, as the
/// old `volume-x` did); brightness is always the sun.
fn glyph(kind: OsdKind, level: Option<Level>) -> LevelGlyph {
    match kind {
        OsdKind::Brightness => LevelGlyph::Brightness,
        OsdKind::Volume => LevelGlyph::Volume(match level {
            Some(Level {
                mute: Mute::Audible,
                ..
            }) => Muting::Audible,
            Some(Level {
                mute: Mute::Muted, ..
            })
            | None => Muting::Muted,
        }),
    }
}

/// The OSD's title: the sink's description for volume ("Sound" when there is none), the output's
/// monitor name for brightness ("Display" when it is a built-in panel or unknown).
pub fn title(
    kind: OsdKind,
    volume: &Volume,
    displays: &Displays,
    output: Option<&OutputName>,
) -> String {
    match kind {
        OsdKind::Volume => match volume.presence {
            Presence::Present if !volume.sink.is_empty() => volume.sink.clone(),
            _ => "Sound".to_owned(),
        },
        OsdKind::Brightness => output
            .and_then(|name| monitor_name(displays, &displays.target(name)))
            .unwrap_or_else(|| "Display".to_owned()),
    }
}

/// The DDC monitor's product name behind a brightness `target`, if it has one.
fn monitor_name(displays: &Displays, target: &BrightnessTarget) -> Option<String> {
    match target {
        BrightnessTarget::Ddc(id) => displays
            .ddc
            .iter()
            .find(|d| d.endpoint.id == *id)
            .map(|d| d.endpoint.product.clone()),
        BrightnessTarget::Backlight(_) | BrightnessTarget::None => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{card_level, level_of, title};
    use crate::surfaces::osd::link::{Level, LevelSource, OsdKind};
    use ds::{Fraction, LevelGlyph, Muting};
    use sill_model::volume::{Mute, Volume};
    use sill_model::{
        ids::OutputName,
        units::{Percent, Presence},
    };
    use sill_services::Displays;
    use sill_services::displays::{Controls, DdcDisplay, Level as DdcLevel};
    use sill_services::env::ddc::DdcId;
    use sill_services::env::ddc::Endpoint;

    fn level(percent: u16, mute: Mute) -> Option<Level> {
        Some(Level {
            percent: Percent(percent),
            mute,
        })
    }

    /// A level as the tables spell it: percent and mute, or none.
    type At = Option<(u16, Mute)>;

    #[test]
    fn the_card_level_follows_the_glyph_and_fills_by_percent() {
        const CASES: &[(OsdKind, At, LevelGlyph, u16)] = &[
            (OsdKind::Volume, None, LevelGlyph::Volume(Muting::Muted), 0),
            (
                OsdKind::Volume,
                Some((0, Mute::Audible)),
                LevelGlyph::Volume(Muting::Audible),
                0,
            ),
            (
                OsdKind::Volume,
                Some((70, Mute::Muted)),
                LevelGlyph::Volume(Muting::Muted),
                0,
            ),
            (
                OsdKind::Volume,
                Some((45, Mute::Audible)),
                LevelGlyph::Volume(Muting::Audible),
                450,
            ),
            (
                OsdKind::Volume,
                Some((130, Mute::Audible)),
                LevelGlyph::Volume(Muting::Audible),
                1000,
            ),
            (
                OsdKind::Brightness,
                Some((0, Mute::Audible)),
                LevelGlyph::Brightness,
                0,
            ),
            (OsdKind::Brightness, None, LevelGlyph::Brightness, 0),
            (
                OsdKind::Brightness,
                Some((49, Mute::Audible)),
                LevelGlyph::Brightness,
                490,
            ),
        ];
        for &(kind, at, glyph, fill) in CASES {
            let at = at.and_then(|(p, m)| level(p, m));
            let card = card_level(kind, at);
            assert_eq!(card.glyph, glyph, "{kind:?} {at:?}");
            assert_eq!(card.value, Fraction(fill), "{kind:?} {at:?}");
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

    #[test]
    fn the_title_is_the_real_name_where_the_services_give_one() {
        let displays = Displays {
            ddc: vec![DdcDisplay {
                endpoint: Endpoint {
                    id: DdcId(5),
                    connector: Some(OutputName("HDMI-A-2".to_owned())),
                    product: "BenQ MA270U".to_owned(),
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
        let panel = OutputName("eDP-1".to_owned());
        let with_sink = Volume {
            presence: Presence::Present,
            sink: "Built-in Audio Analog Stereo".to_owned(),
            ..Volume::default()
        };
        let cases = [
            (
                "a real sink",
                OsdKind::Volume,
                &with_sink,
                None,
                "Built-in Audio Analog Stereo",
            ),
            (
                "no sink",
                OsdKind::Volume,
                &Volume::default(),
                None,
                "Sound",
            ),
            (
                "an empty sink name",
                OsdKind::Volume,
                &Volume {
                    presence: Presence::Present,
                    ..Volume::default()
                },
                None,
                "Sound",
            ),
            (
                "a real monitor",
                OsdKind::Brightness,
                &with_sink,
                Some(&hdmi),
                "BenQ MA270U",
            ),
            (
                "a built-in panel",
                OsdKind::Brightness,
                &with_sink,
                Some(&panel),
                "Display",
            ),
            (
                "no output",
                OsdKind::Brightness,
                &with_sink,
                None,
                "Display",
            ),
        ];
        for (name, kind, volume, output, want) in cases {
            assert_eq!(title(kind, volume, &displays, output), want, "{name}");
        }
    }
}
