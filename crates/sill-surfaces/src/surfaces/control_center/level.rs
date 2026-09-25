//! The one level control the Sound and Display modules share: quire's `ModulePanel` headed by
//! the glyph, the title and the percentage, over quire's `LevelControl` (Q83) in its default look, the capsule with the glyph inside that
//! follows the level. The look is the user's pick to make later; it is chosen here and nowhere
//! else ([`LOOK`]). The OSD reuses the same control through quire's `Osd`.

use dioxus::prelude::*;
use ds::{
    Availability, Fraction, Icon, LevelControl, LevelGlyph, LevelLook, LevelMode, ModulePanel,
    Muting, Text, Tick,
};

use super::modules::use_plate;

/// The control center's level look.
const LOOK: LevelLook = LevelLook::Capsule;

/// `glyph` and `title` over the level at `level` (the percentage as the header's value);
/// `on_change` hears every new level from a drag or a key. `Disabled` shows the level and takes
/// nothing.
#[component]
pub fn LevelModule(
    glyph: Icon,
    title: String,
    level: Fraction,
    on_change: EventHandler<Fraction>,
    #[props(default)] availability: Availability,
) -> Element {
    let percent = (level.0.min(1000) + 5) / 10;
    rsx! {
        ModulePanel {
            glyph,
            title: Text::from(&title),
            trailing: rsx! { "{percent}%" },
            plate: use_plate(),
            LevelControl {
                label: title.clone(),
                value: level,
                glyph: level_glyph(glyph),
                mode: LevelMode::Interactive,
                look: LOOK,
                tick: Tick::Quiet,
                availability,
                onchange: move |value: Fraction| on_change.call(value),
            }
        }
    }
}

/// quire's level glyph for the module's: the sun for brightness, else the speaker, whose waves
/// follow the level on their own (the Sound module is told no mute).
pub(crate) fn level_glyph(glyph: Icon) -> LevelGlyph {
    match glyph {
        Icon::Sun | Icon::SunMoon | Icon::Brightness => LevelGlyph::Brightness,
        _ => LevelGlyph::Volume(Muting::Audible),
    }
}

#[cfg(test)]
mod tests {
    use super::level_glyph;
    use ds::{Icon, LevelGlyph, Muting};

    #[test]
    fn the_sun_is_brightness_and_the_rest_a_speaker() {
        assert_eq!(level_glyph(Icon::Sun), LevelGlyph::Brightness);
        for icon in [Icon::Volume, Icon::Volume2, Icon::VolumeX] {
            assert_eq!(
                level_glyph(icon),
                LevelGlyph::Volume(Muting::Audible),
                "{icon:?}"
            );
        }
    }
}
