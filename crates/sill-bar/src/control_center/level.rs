//! The one level control the Sound and Display modules share: quire's `ModulePanel` headed by
//! the glyph, the title and the percentage, over quire's `Slider` in its `Mac` look: the
//! Control Center's slider, a Regular well (22) under a 20 knob that carries the level's glyph,
//! click to jump, drag the knob, arrows to step (quire design/29 §16). The look is chosen here
//! and nowhere else ([`LOOK`]). The OSD draws the same level through quire's `Osd`, as the
//! bezel's sixteen squares.

use dioxus::prelude::*;
use ds::components::content::level_glyph::vocab::LevelSource;
use ds::components::controls::slider_model::SliderLook;
use ds::prelude::*;
use sill_shell_kit::prelude::*;

use super::modules::use_plate;

/// The control center's level look.
const LOOK: SliderLook = SliderLook::Mac;

/// `glyph` and `title` over the level at `level` (the percentage as the header's value), the
/// knob carrying `level_glyph` (the sun, or the volume's state, the bar item's source);
/// `on_change` hears every new level from a drag or a key. `Disabled` shows the level and takes
/// nothing: quire dims the level and the panel's header with it, so both get it.
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
            title: TextLine::from(&title),
            trailing: rsx! { "{percent}%" },
            plate: use_plate(),
            availability,
            Slider {
                label: title.clone(),
                value: level,
                glyph: level_glyph,
                look: LOOK,
                availability,
                onchange: move |value: Fraction| on_change.call(value),
            }
        }
    }
}
