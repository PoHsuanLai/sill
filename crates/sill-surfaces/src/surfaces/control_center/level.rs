//! The one level control the Sound and Display modules share: a glyph, a title and the level,
//! drawn with quire's `Slider` in its default mode for now. quire is designing a level style of
//! its own (a thick capsule with the glyph inside that follows the level, rubber-banding at the
//! ends, swelling on press, after macOS); when it lands, this file switches to it and nothing
//! else changes. The slider is never styled here (coherence rule 1). The OSD is to reuse the
//! same look once it moves under the bar (its follow-up).

use dioxus::prelude::*;
use ds::{Availability, Fraction, Icon};

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
    let _ = (glyph, title, level, on_change, availability);
    todo!("M5 fill: control-center (c)")
}
