//! The frontmost app's name, the first bar title (ui 13/700): its menu lists the app's windows and Quit.

use dioxus::prelude::*;
use ds::{Button, ButtonVariant, Common, Emphasis, use_rect};
use shell_host::OutputInfo;
use sill_model::ids::OutputName;

use super::content::front_app;
use super::menus::BarMenu;
use super::title::{SlotPill, TitleSlot};
use sill_services::{AppsService, CosmicWlService};
use sill_ui_kit::hooks::{use_service, use_service_part};

/// What the title says with no window on the output.
const NO_APP: &str = "Desktop";

/// The frontmost app's name, the first bar title (ui 13/700): its menu lists the app's windows and Quit.
#[component]
pub fn AppName(output: OutputInfo) -> Element {
    let toplevels = use_service_part::<CosmicWlService, _>(|wl| wl.toplevels.clone());
    let apps = use_service::<AppsService>();
    let probe = use_rect();
    let on = output.name.clone().map(OutputName);
    let name = front_app(&toplevels(), &apps(), on.as_ref())
        .map_or_else(|| NO_APP.to_owned(), |app| app.name);
    rsx! {
        TitleSlot { menu: BarMenu::App, probe, pill: SlotPill::Item(Emphasis::Strong),
            Button {
                common: Common { mounted: Some(EventHandler::new(move |event| probe.on_mounted(event))), ..Common::default() },
                variant: ButtonVariant::Quiet,
                label: name,
                onclick: move |_| {},
            }
        }
    }
}
