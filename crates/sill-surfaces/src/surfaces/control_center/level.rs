//! The one level control the Sound and Display modules share: quire's `ModulePanel` headed by
//! the glyph, the title and the percentage, over quire's `LevelControl` (Q83) in its default look, the capsule with the glyph inside that
//! follows the level. The look is the user's pick to make later; it is chosen here and nowhere
//! else ([`LOOK`]). The OSD reuses the same control through quire's `Osd`.

use dioxus::prelude::*;
use ds::{
    Availability, Fraction, IconSource, LevelControl, LevelLook, LevelMode, LevelSource,
    ModulePanel, Text, Tick,
};

use super::modules::use_plate;

/// The control center's level look.
const LOOK: LevelLook = LevelLook::Capsule;

/// `glyph` and `title` over the level at `level` (the percentage as the header's value), the
/// capsule carrying `level_glyph` (the sun, or the volume's state, the bar item's source);
/// `on_change` hears every new level from a drag or a key. `Disabled` shows the level and takes
/// nothing.
#[component]
pub fn LevelModule(
    #[props(into)] glyph: IconSource,
    #[props(into)] level_glyph: LevelSource,
    title: String,
    level: Fraction,
    on_change: EventHandler<Fraction>,
    #[props(default)] availability: Availability,
) -> Element {
    let percent = (level.0.min(1000) + 5) / 10;
    rsx! {
        ModulePanel {
            glyph: Some(glyph),
            title: Text::from(&title),
            trailing: rsx! { "{percent}%" },
            plate: use_plate(),
            LevelControl {
                label: title.clone(),
                value: level,
                glyph: level_glyph,
                mode: LevelMode::Interactive,
                look: LOOK,
                tick: Tick::Quiet,
                availability,
                onchange: move |value: Fraction| on_change.call(value),
            }
        }
    }
}
