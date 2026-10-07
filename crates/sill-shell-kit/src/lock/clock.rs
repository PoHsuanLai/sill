//! LockClock: the lock screen's time and date (design/20-SURFACES.md section 1.9;
//! design/04-COMPONENTS.md section 42). The date line, then the time in the display face at
//! `--fs-lock-clock`, heavy, in white over the wallpaper: the reference lock screen's order,
//! where the day sits above a very large time.

use crate::lock::vocab::LockLook;
use dioxus::prelude::*;
use ds::base::word::Word;
use ds::root::common::Common;

/// The time and the date, as the caller words them ("9:41", "Friday 26 September"): the
/// caller owns the clock, the locale and the 12 or 24 hour choice, and re-renders on each new
/// minute. `look: LockLook::Space` sets the date in a pill of the Space gradient.
#[component]
pub fn LockClock(
    #[props(into)] time: String,
    #[props(into)] date: String,
    #[props(default)] look: LockLook,
    #[props(default)] common: Common,
) -> Element {
    let class = common.class("ds-lock-clock");
    let data = common.data_attributes();
    let label = common
        .aria_label
        .clone()
        .unwrap_or_else(|| format!("{date}, {time}"));
    rsx! {
        div {
            class,
            id: common.id.clone(),
            "data-look": look.slug(),
            role: "group",
            "aria-label": "{label}",
            onmounted: move |event| common.mounted(event),
            ..data,
            div { class: "ds-lock-date", "{date}" }
            div { class: "ds-lock-time", "{time}" }
        }
    }
}
