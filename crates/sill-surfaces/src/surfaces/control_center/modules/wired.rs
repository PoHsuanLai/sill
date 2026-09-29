//! The Sound and Display modules wired to their services: what [`super::Module`] and the bar's
//! Sound and Display dropdowns draw.

use dioxus::prelude::*;
use ds::{Availability, Fraction};
use sill_model::displays::{BrightnessTarget, Displays, DisplaysCommand, Following};
use sill_model::ids::OutputName;
use sill_model::units::{Percent, Presence};
use sill_model::volume::{Volume, VolumeCommand};
use sill_services::{DisplaysService, VolumeService};

use super::{DisplayModule, SoundModule};
use crate::hooks::{use_control, use_service};
use crate::status::glyph::volume_glyph;
use crate::status::lines::volume_heading;

/// The Sound module on the default sink.
#[component]
pub(crate) fn WiredSound() -> Element {
    let volume = use_service::<VolumeService>()();
    let control = use_control::<VolumeService>();
    let (level, device, availability) = sound_of(&volume);
    rsx! {
        SoundModule {
            volume: volume_glyph(&volume),
            level,
            device,
            availability,
            on_change: move |to: Fraction| {
                // A stopped service drops it: the slider shows the level it last published.
                let _gone = control.send(VolumeCommand::SetLevel(percent_of(to)));
            },
        }
    }
}

/// The Display module on `output`'s brightness. While it is shown the service follows the
/// monitors (`Following::Start`, then `Stop` when it goes), so a level changed on the monitor or
/// by another program moves the capsule; a drag or click sets exactly the target the capsule
/// shows (`SetBrightness` on `output`).
#[component]
pub(crate) fn WiredDisplay(output: Option<OutputName>) -> Element {
    let displays = use_service::<DisplaysService>()();
    let control = use_control::<DisplaysService>();
    use_hook({
        let control = control.clone();
        // A stopped service drops it: the capsule shows the level it last published.
        move || control.send(DisplaysCommand::Follow(Following::Start)).ok()
    });
    use_drop({
        let control = control.clone();
        move || {
            let _gone = control.send(DisplaysCommand::Follow(Following::Stop));
        }
    });
    let (level, availability) = brightness_of(&displays, output.as_ref());
    rsx! {
        DisplayModule {
            level,
            availability,
            on_change: move |to: Fraction| {
                if let Some(command) = set_brightness(output.as_ref(), to) {
                    // A stopped service drops it: the capsule shows the level it last published.
                    let _gone = control.send(command);
                }
            },
        }
    }
}

/// The command a capsule on `output` sends for `to`: nothing without an output (the capsule is
/// disabled then).
pub(crate) fn set_brightness(output: Option<&OutputName>, to: Fraction) -> Option<DisplaysCommand> {
    output.map(|output| DisplaysCommand::SetBrightness {
        output: output.clone(),
        level: percent_of(to),
    })
}

/// The Sound module's level, device and availability for `volume`.
pub(crate) fn sound_of(volume: &Volume) -> (Fraction, String, Availability) {
    let availability = match volume.presence {
        Presence::Present => Availability::Enabled,
        Presence::Absent => Availability::Disabled,
    };
    (
        fraction_of(volume.level),
        volume_heading(volume),
        availability,
    )
}

/// The Display module's level and availability on `output`: disabled where nothing can change
/// the brightness.
pub(crate) fn brightness_of(
    displays: &Displays,
    output: Option<&OutputName>,
) -> (Fraction, Availability) {
    let target = output.map_or(BrightnessTarget::None, |name| displays.target(name));
    let level = displays
        .brightness(&target)
        .map_or(Fraction(0), fraction_of);
    let availability = match target {
        BrightnessTarget::None => Availability::Disabled,
        BrightnessTarget::Backlight(_) | BrightnessTarget::Ddc(_) => Availability::Enabled,
    };
    (level, availability)
}

/// A percentage (over 100 clamped) as quire's thousandths.
pub(crate) fn fraction_of(percent: Percent) -> Fraction {
    Fraction(percent.0.min(100) * 10)
}

/// Thousandths back to the nearest percentage.
pub(crate) fn percent_of(fraction: Fraction) -> Percent {
    Percent((fraction.0.min(1000) + 5) / 10)
}

#[cfg(test)]
mod tests {
    use super::{brightness_of, fraction_of, percent_of, set_brightness, sound_of};
    use ds::{Availability, Fraction};
    use sill_model::displays::{Displays, DisplaysCommand};
    use sill_model::volume::{Mute, Volume};
    use sill_model::{
        ids::OutputName,
        units::{Percent, Presence},
    };

    #[test]
    fn levels_convert_both_ways() {
        let cases = [(0, 0), (40, 400), (100, 1000), (150, 1000)];
        for (percent, thousandths) in cases {
            assert_eq!(fraction_of(Percent(percent)), Fraction(thousandths));
        }
        assert_eq!(percent_of(Fraction(404)), Percent(40));
        assert_eq!(percent_of(Fraction(405)), Percent(41));
        let speakers = Volume {
            presence: Presence::Present,
            level: Percent(40),
            mute: Mute::Audible,
            sink: "Speakers".to_owned(),
        };
        assert_eq!(
            sound_of(&speakers),
            (Fraction(400), "Speakers".to_owned(), Availability::Enabled)
        );
        assert_eq!(sound_of(&Volume::default()).2, Availability::Disabled);
        assert_eq!(
            brightness_of(&Displays::default(), None),
            (Fraction(0), Availability::Disabled)
        );
        let hdmi = OutputName("HDMI-A-2".to_owned());
        assert_eq!(
            set_brightness(Some(&hdmi), Fraction(625)),
            Some(DisplaysCommand::SetBrightness {
                output: hdmi,
                level: Percent(63),
            })
        );
        assert_eq!(set_brightness(None, Fraction(625)), None);
    }
}
