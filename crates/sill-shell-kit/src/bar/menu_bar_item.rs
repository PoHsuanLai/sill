//! MenuBarItem: an item of the menu bar, `NSStatusItem`'s button or a menu's title
//! (design/30 section 2.10, design/13 section 13.3.1). One component for the app name and the
//! clock (a title) and the Wi-Fi, volume and battery items (a glyph).
//!
//! Markup: `button.ds-menu-bar-item[data-image][data-emphasis][data-availability]` holding
//! `.ds-menu-bar-item-icon` and/or `.ds-menu-bar-item-title`; `aria-expanded` says whether the
//! item's menu is showing, `aria-pressed` and `data-state` only for a toggled item, and
//! `data-pressed` while a press is under way. The title is the shell scale's bar size and weight
//! (`--fs-shell-bar` 13, `--fw-shell-bar` 500; the app name is `Emphasis::Strong`, 700). The pill
//! is `--shell-bar-item` (22) high with the 5 px `--r-shell-bar-item` radius: `--surface` under
//! the pointer, `--surface-2` while pressed or while its menu is open, with no transition (a bar
//! menu switches in the same frame). A glyph item is as wide as the bar's status slot.

use crate::bar::pointer::BarPointer;
use dioxus::prelude::*;
use ds::base::press::Press;
use ds::base::vocab::{Availability, Check, Emphasis, Shown};
use ds::base::word::Word;
use ds::components::content::icon_source::IconSource;
use ds::components::content::icon_view::IconView;
use ds::components::content::text_runs::{TextLine, text};
use ds::components::controls::button_model::ImagePosition;
use ds::components::controls::press::{ActivationKeys, PressListeners, disabled, use_pressing};
use ds::root::common::Common;
use ds::style::icon::render::IconSize;

/// One bar item. `label` is the title, and the item's accessible name when it is drawn as a glyph
/// only; `icon` puts a glyph before it (an `Icon`, or a layered status glyph), and
/// `ImagePosition::Only` draws the glyph alone. `shown` says whether the item's menu is up
/// (`aria-expanded`). `value` makes it a toggle. `onclick` hears the press; `pointer` hears the
/// raw pointer events, which a bar menu's session reads (it opens on the press, so it passes an
/// `onclick` that does nothing). `title` is the hover hint.
#[component]
pub fn MenuBarItem(
    #[props(into)] label: TextLine,
    #[props(default)] icon: Option<IconSource>,
    #[props(default)] image: ImagePosition,
    #[props(default)] emphasis: Emphasis,
    #[props(default)] shown: Shown,
    #[props(default)] value: Option<Check>,
    #[props(default)] availability: Availability,
    #[props(default)] title: Option<String>,
    onclick: EventHandler<Press>,
    #[props(default)] pointer: BarPointer,
    #[props(default)] common: Common,
) -> Element {
    let class = common.class("ds-menu-bar-item");
    let data = common.data_attributes();
    let spoken = match image {
        ImagePosition::Only => Some(label.plain_text()),
        ImagePosition::Leading => None,
    };
    let aria_label = common.aria_label.clone().or(spoken);
    let listen = PressListeners::new(onclick);
    let pressing = use_pressing();
    let live = availability == Availability::Enabled;
    rsx! {
        button {
            r#type: "button",
            id: common.id.clone(),
            class,
            "data-image": image.slug(),
            "data-emphasis": emphasis.slug(),
            "data-availability": availability.slug(),
            "data-state": value.map(|state| state.slug()),
            title,
            "aria-label": aria_label,
            "aria-pressed": value.map(Check::aria),
            "aria-expanded": shown.aria(),
            "aria-disabled": availability.aria_disabled(),
            "aria-busy": availability.aria_busy(),
            disabled: disabled(availability),
            "data-pressed": if live { pressing.attr() } else { None },
            onpointerdown: {
                let pointer = pointer.clone();
                move |event| pointer.pressed(event)
            },
            onpointerup: {
                let pointer = pointer.clone();
                move |event| pointer.released(event)
            },
            onpointerenter: move |event| pointer.entered(event),
            onmousedown: move |event| pressing.pointer_down(&event),
            onmouseleave: move |_| pressing.released(),
            onkeydown: move |event| {
                if live {
                    pressing.key_down(&event, ActivationKeys::ReturnAndSpace);
                    listen.key_down(&event, ActivationKeys::ReturnAndSpace);
                }
            },
            onkeyup: move |_| pressing.released(),
            onblur: move |_| pressing.released(),
            onclick: move |event| {
                if live {
                    listen.click(&event);
                }
            },
            oncontextmenu: move |event| {
                if live {
                    listen.context_menu(&event);
                }
            },
            onmouseup: move |event| {
                pressing.released();
                if live {
                    listen.mouse_up(&event);
                }
            },
            onmounted: move |event| common.mounted(event),
            ..data,
            if let Some(source) = icon {
                span { class: "ds-menu-bar-item-icon",
                    IconView { source, size: IconSize::Base }
                }
            }
            if image == ImagePosition::Leading {
                span { class: "ds-menu-bar-item-title", {text(&label)} }
            }
        }
    }
}
