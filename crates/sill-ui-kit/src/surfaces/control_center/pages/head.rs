//! A detail pane's header: the back button (in the control center; a module's own bar dropdown
//! has none), the title, and the feature's switch.

use dioxus::prelude::*;
use ds::{Availability, Icon, IconButton, IconButtonVariant, ModuleState, Switch, Toggle};

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
            IconButton {
                variant: IconButtonVariant::Tool,
                icon: Icon::ChevronLeft,
                label: "Back".to_owned(),
                onclick: move |_| back.call(()),
            }
        }
    });
    let value = match state {
        ModuleState::On | ModuleState::Busy => Switch::On,
        ModuleState::Off => Switch::Off,
    };
    rsx! {
        div { class: "sill-cc-detail-head",
            {back}
            span { class: "sill-cc-detail-title", "{title}" }
            Toggle {
                label: title.clone(),
                value,
                availability,
                onchange: move |to: Switch| on_toggle.call(match to {
                    Switch::On => ModuleState::On,
                    Switch::Off => ModuleState::Off,
                }),
            }
        }
    }
}
