//! The clock status item; its menu is the date.

use dioxus::prelude::*;
use ds::{Button, ButtonVariant, Emphasis, use_rect};
use shell_host::OutputInfo;

use super::menus::BarMenu;
use super::title::{SlotPill, TitleSlot};
use crate::hooks::use_clock;

/// The clock status item; its menu is the date.
#[component]
pub fn ClockItem(output: OutputInfo) -> Element {
    // Every bar item is told its output (the frozen signature); this one reads the same on all.
    let _ = &output;
    let clock = use_clock()();
    let probe = use_rect();
    rsx! {
        TitleSlot { menu: BarMenu::Clock, probe, pill: SlotPill::Item(Emphasis::Plain),
            div { class: "sill-bar-clock",
                Button {
                    variant: ButtonVariant::Quiet,
                    label: clock.text,
                    onclick: move |_| {},
                    mounted: move |event| probe.on_mounted(event),
                }
            }
        }
    }
}
