//! ModuleTile: one control-center module, a glyph disc with its title and status on a tile, and
//! the chevron that opens the module's detail pane (design/30 section 2.10,
//! design/20-SURFACES.md section 1.5, design/13-BEHAVIOUR-menus-windows.md section 13.3.7).
//!
//! Markup: `div.ds-module-tile[role=button][data-span][data-state][data-availability]` with
//! `aria-pressed` (`mixed` while busy), `aria-busy` and `aria-disabled`; parts `.ds-module-disc`,
//! `.ds-module-title`, `.ds-module-status` and `.ds-module-chevron`.
//!
//! The tile is a `div[role=button]` rather than a `button`, because the chevron inside it is a
//! button of its own and a button may not hold another. The chevron keeps its press
//! (`Propagation::Stop`), so a press on it opens the detail and never also toggles the module.
//! Keys are handled here on both renderers: Blitz synthesises no click from Enter, and the
//! chevron prevents the default of the keys it takes, so a browser's synthesised click cannot
//! run the same press twice.

use crate::control_center::module_disc::{Lighting, ModuleDisc};
use crate::control_center::module_tile_kind::TileSpan;
use dioxus::prelude::*;
use ds::base::press::Press;
use ds::base::vocab::{Availability, Check, Shown};
use ds::base::word::Word;
use ds::components::content::icon_source::IconSource;
use ds::components::content::text_runs::{TextLine, text};
use ds::components::controls::press::{ActivationKeys, PressListeners, Propagation};
use ds::focus::click::kept_click;
use ds::root::common::Common;
use ds::style::icon::Icon;
use ds::style::icon::render::{Glyph, IconSize};

/// A control-center module. `onclick` toggles it (a press on the tile, Enter or Space);
/// `value` is whether it is on. A module with a detail pane passes `on_detail`, which draws the
/// chevron and hears a press on it (or Enter or Right on the chevron); `expanded` is the
/// chevron's `aria-expanded`: whether that pane is showing.
///
/// `availability`: `Disabled` dims the tile and drops its presses; `Busy` is a module working
/// towards a state it has not reached (connecting, scanning): the disc turns its ring, the tile
/// takes no press and reads as `aria-pressed="mixed"`.
///
/// `glyph` is an `Icon` (it converts) or any [`IconSource`]: `IconSource::Status` puts a layered
/// status glyph in the disc (the Wi-Fi and Bluetooth modules), which plays its own
/// moments as the state changes.
#[component]
pub fn ModuleTile(
    #[props(into)] glyph: IconSource,
    #[props(into)] title: TextLine,
    status: Option<TextLine>,
    #[props(default)] value: Check,
    #[props(default)] span: TileSpan,
    onclick: EventHandler<Press>,
    #[props(default)] on_detail: Option<EventHandler<Press>>,
    #[props(default)] expanded: Shown,
    #[props(default)] availability: Availability,
    #[props(default)] common: Common,
) -> Element {
    let live = availability == Availability::Enabled;
    let listen = PressListeners::new(onclick);
    let lighting = Lighting::of(value, availability);
    let pressed = match availability {
        Availability::Busy => Check::Mixed,
        Availability::Enabled | Availability::Disabled => value,
    };
    let detail = on_detail.map(|open| chevron_button(&title, open, expanded, availability));
    let class = common.class("ds-module-tile");
    let data = common.data_attributes();
    rsx! {
        div {
            id: common.id.clone(),
            class,
            role: "button",
            tabindex: "0",
            "data-span": span.slug(),
            "data-state": value.slug(),
            "data-availability": availability.slug(),
            "aria-label": common.aria_label.clone(),
            "aria-pressed": pressed.aria(),
            "aria-busy": availability.aria_busy(),
            "aria-disabled": availability.aria_disabled(),
            onmounted: move |event| common.mounted(event),
            onclick: move |event| {
                if live {
                    listen.click(&event);
                }
            },
            onkeydown: move |event| {
                if live {
                    listen.key_down(&event, ActivationKeys::ReturnAndSpace);
                }
            },
            ..data,
            ModuleDisc { glyph, lighting }
            span { class: "ds-module-words",
                span { class: "ds-module-title", {text(&title)} }
                if let Some(status) = status {
                    span { class: "ds-module-status", {text(&status)} }
                }
            }
            {detail}
        }
    }
}

/// Whether a key on the chevron opens the detail: Enter, or Right (into the pane).
fn opens(key: &Key) -> bool {
    matches!(key, Key::Enter | Key::ArrowRight)
}

/// The chevron: its own hit target, named for the module, keeping every press it takes.
fn chevron_button(
    title: &TextLine,
    on_detail: EventHandler<Press>,
    expanded: Shown,
    availability: Availability,
) -> Element {
    let name = format!("{} details", title.plain_text());
    let open = (availability == Availability::Enabled).then_some(on_detail);
    let inert = open.map_or(Availability::Disabled, |_| Availability::Enabled);
    let listen = open.map(|open| PressListeners::new(open).with_propagation(Propagation::Stop));
    rsx! {
        button {
            r#type: "button",
            class: "ds-module-chevron",
            "aria-label": "{name}",
            title: "{name}",
            "aria-expanded": expanded.aria(),
            "aria-disabled": inert.aria_disabled(),
            onclick: move |event| {
                if let Some(listen) = listen {
                    listen.click(&event);
                } else {
                    // An inert chevron still is not the tile: its press toggles nothing.
                    event.stop_propagation();
                    kept_click(&event);
                }
            },
            onkeydown: move |event| {
                if opens(&event.key()) {
                    event.stop_propagation();
                    event.prevent_default();
                    if let Some(open) = open {
                        open.call(Press::primary());
                    }
                }
            },
            Glyph { icon: Icon::ChevronRight, size: IconSize::Compact }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::opens;
    use dioxus::prelude::Key;

    #[test]
    fn the_chevron_opens_on_enter_and_right() {
        let cases = [
            (Key::Enter, true),
            (Key::Character(" ".into()), false),
            (Key::ArrowRight, true),
            (Key::ArrowLeft, false),
            (Key::Tab, false),
        ];
        for (key, chevron) in cases {
            assert_eq!(opens(&key), chevron, "chevron {key:?}");
        }
    }
}
