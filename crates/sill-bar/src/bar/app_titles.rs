//! The focused app's own menus after its name (File, Edit, View, Window, Help): one bar title
//! each, drawn like the app-name title but regular weight, each a [`BarMenu::AppMenu`] of the
//! bar's one menu session so hover switching crosses the app's menus, the tray and the status
//! items alike. The app serves them over D-Bus (`sill_services::AppMenuService`); an app with no
//! menu has none of these titles (design/27 §5.2 rule c).

use dioxus::prelude::*;
use ds::host::measure::use_rect;
use ds::root::common::Common;
use shell_host::OutputInfo;
use sill_model::ids::OutputName;
use sill_services::{AppMenuService, AppsService, WindowsService};
use sill_shell_kit::prelude::*;
use sill_ui_kit::hooks::{use_service, use_service_part};

use super::content::{AppMenuState, front_app};
use super::menus::{AppTitle, BarMenu};
use super::title::{shown_of, use_title_pointer};

/// The titles of the focused app's menus other than its first, which the app-name title holds.
#[component]
pub fn AppMenuTitles(output: OutputInfo) -> Element {
    let toplevels = use_service_part::<WindowsService, _>(|wl| wl.toplevels.clone());
    let apps = use_service::<AppsService>();
    let menus = use_service::<AppMenuService>();
    let on = output.name.clone().map(OutputName);
    let front = front_app(&toplevels(), &apps(), on.as_ref());
    let titles = AppMenuState::of(front.as_ref(), &menus())
        .map(|state| state.titles.menus)
        .unwrap_or_default();
    rsx! {
        for (index , menu) in titles.into_iter().enumerate() {
            AppMenuTitle { key: "{menu.id}", index: u16::try_from(index).unwrap_or(u16::MAX), title: menu.title }
        }
    }
}

/// One title.
#[component]
fn AppMenuTitle(index: u16, title: String) -> Element {
    let menu = BarMenu::AppMenu(AppTitle(index));
    let probe = use_rect();
    let pointer = use_title_pointer(menu, probe);
    rsx! {
        MenuBarItem {
            common: Common { mounted: Some(EventHandler::new(move |event| probe.on_mounted(event))), ..Common::default() },
            shown: shown_of(menu),
            label: title,
            pointer,
            onclick: move |_| {},
        }
    }
}
