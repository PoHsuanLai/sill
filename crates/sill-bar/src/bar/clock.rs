//! The clock status item. A click toggles the notification center on this output, as the clock
//! opens Notification Center on macOS 11 on; it no longer opens a date menu. Its pill is quire's
//! `MenuBarItem`, on while the center is open here.

use dioxus::prelude::*;
use ds::prelude::*;
use ds::root::common::Common;
use ds::root::pass_through::ExtraClass;
use shell_host::{OutputInfo, use_watch};
use sill_shell_kit::prelude::*;

use sill_model::clock::format_clock;
use sill_services::ClockService;
use sill_ui_kit::hooks::{use_hours, use_link, use_service};
use sill_ui_kit::links::CenterLink;

/// The clock status item.
#[component]
pub fn ClockItem(output: OutputInfo) -> Element {
    let minute = use_service::<ClockService>()();
    let hours = use_hours();
    // A bar without the daemon's links (the demo, a render test) toggles a center of its own.
    let given = use_link::<CenterLink>();
    let link = use_hook(move || given.unwrap_or_default());
    let shown = use_watch(link.subscribe());
    let open = match shown().open_on(output.id) {
        true => Shown::Visible,
        false => Shown::Hidden,
    };
    rsx! {
        MenuBarItem {
            common: Common {
                aria_label: Some("Notification Center".to_owned()),
                extra_class: ExtraClass::parse("sill-bar-clock").ok(),
                ..Common::default()
            },
            shown: open,
            label: format_clock(minute.0, hours),
            onclick: move |_| link.toggle(output.id),
        }
    }
}
