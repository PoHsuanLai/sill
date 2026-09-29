//! The clock status item. A click toggles the notification center on this output, as the clock
//! opens Notification Center on macOS 11 on; it no longer opens a date menu. Its pill is quire's
//! `MenuBarItem`, on while the center is open here.

use dioxus::prelude::*;
use ds::{Button, ButtonVariant, Emphasis, MenuBarItem, Switch};
use shell_host::{OutputInfo, use_watch};

use crate::hooks::{use_service, use_settings};
use crate::links::CenterLink;
use sill_services::ClockService;

/// The clock status item.
#[component]
pub fn ClockItem(output: OutputInfo) -> Element {
    let clock = use_service::<ClockService>()();
    let settings = use_settings();
    // A bar without the daemon's links (the demo, a render test) toggles a center of its own.
    let given = try_use_context::<CenterLink>();
    let link = use_hook(move || given.unwrap_or_default());
    let shown = use_watch(link.subscribe());
    let open = match shown().open_on(output.id) {
        true => Switch::On,
        false => Switch::Off,
    };
    let style = format!("height:{}px", settings().bar.title_hit_height_px.0);
    rsx! {
        div { class: "sill-bar-title", style,
            MenuBarItem { open, emphasis: Emphasis::Plain,
                div { class: "sill-bar-clock",
                    Button {
                        variant: ButtonVariant::Quiet,
                        label: clock.text,
                        aria_label: "Notification Center".to_owned(),
                        onclick: move |_| link.toggle(output.id),
                    }
                }
            }
        }
    }
}
