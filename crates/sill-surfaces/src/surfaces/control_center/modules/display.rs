//! The Display module: a `SectionHeader` and a brightness quire `Slider` (the `Sun` glyph) for
//! the bar's output, through the displays service: the level from `Displays::target` and
//! `Displays::brightness`, a change as `DisplaysCommand::Brightness { at, change:
//! LevelChange::Set(..) }`. Disabled when the target is `BrightnessTarget::None`. Reads
//! `use_displays`.

use dioxus::prelude::*;
use sill_services::OutputName;

/// The Display module for the bar on `output`.
#[component]
pub fn DisplayModule(output: Option<OutputName>) -> Element {
    let _ = output;
    todo!("M5 fill: control-center (c)")
}
