//! What a detail pane's header holds besides the title: the feature's switch, and the back
//! button's handler as quire's `PaneHeader` takes it.

use dioxus::prelude::*;
use ds::{Availability, Check, Press, Toggle};

/// The feature's switch, named `title`, at `state` (a mixed state reads as on).
#[component]
pub(crate) fn FeatureSwitch(
    title: String,
    state: Check,
    availability: Availability,
    on_toggle: EventHandler<Check>,
) -> Element {
    let value = match state {
        Check::On | Check::Mixed => Check::On,
        Check::Off => Check::Off,
    };
    rsx! {
        Toggle {
            label: title,
            value,
            availability,
            onchange: move |to: Check| on_toggle.call(match to {
                Check::On => Check::On,
                Check::Off | Check::Mixed => Check::Off,
            }),
        }
    }
}

/// A pane's `back` (there is none in a module's own bar dropdown) as the header's handler.
pub(crate) fn back_button(back: Option<EventHandler<()>>) -> Option<EventHandler<Press>> {
    back.map(|back| EventHandler::new(move |_: Press| back.call(())))
}
