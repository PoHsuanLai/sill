//! A detail pane's header: the back button (in the control center; a module's own bar dropdown
//! has none), the title, and the feature's switch.

use dioxus::prelude::*;
use ds::{Availability, Check, Icon, Toggle};
use ds::{Bezel, Button, ControlSize, ImagePosition};
use ds_shell::ModuleState;

/// `title` with its switch at `state`; a back button when `back` is given.
#[component]
pub(crate) fn DetailHead(
    title: String,
    back: Option<EventHandler<()>>,
    state: ModuleState,
    availability: Availability,
    on_toggle: EventHandler<ModuleState>,
) -> Element {
    let back = back.map(|back| {
        rsx! {
            Button {
                bezel: Bezel::Toolbar, size: ControlSize::Large, image: ImagePosition::Only,
                icon: Icon::ChevronLeft,
                label: "Back".to_owned(),
                onclick: move |_| back.call(()),
            }
        }
    });
    let value = match state {
        ModuleState::On | ModuleState::Busy => Check::On,
        ModuleState::Off => Check::Off,
    };
    rsx! {
        div { class: "sill-cc-detail-head",
            {back}
            span { class: "sill-cc-detail-title", "{title}" }
            Toggle {
                label: title.clone(),
                value,
                availability,
                onchange: move |to: Check| on_toggle.call(match to {
                    Check::On => ModuleState::On,
                    Check::Off | Check::Mixed => ModuleState::Off,
                }),
            }
        }
    }
}
