//! A bar title or status item's hit box: it feeds the menu session its presses, releases and
//! pointer entries (design/13 §13.3.2: open on press, click mode on release over the title,
//! hover switch on entry). A text title sits in a ds `MenuBarItem`, which draws the hover and
//! open pill (the macOS polish pass); a status item's `IconButton{Status}` draws its own.

use dioxus::prelude::*;
use ds::{Emphasis, MenuBarItem, RectProbe, Shown};
use shell_host::Point;

use super::io::MenuBar;
use super::menu_track::{MenuIn, Target};
use super::menus::BarMenu;
use super::model::BarInput;
use crate::hooks::use_settings;

/// The pointer's position in the bar, whole logical pixels.
pub(crate) fn pointer_at(event: &PointerEvent) -> Point {
    let at = event.client_coordinates();
    Point {
        x: at.x.round() as i32,
        y: at.y.round() as i32,
    }
}

/// Who draws the hover and open pill (design/13 §13.3.1) behind a slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SlotPill {
    /// A ds `MenuBarItem` around a text title (the app name, `Strong`; the clock, `Plain`).
    Item(Emphasis),
    /// The control inside it: an `IconButton{Status}` is `--f-pill` while `expanded`.
    Control,
}

/// The hit box around a title's ds button. `probe` measures that button (its `mounted`), and
/// its rect travels with every event so the popup can be placed under it.
#[component]
pub(crate) fn TitleSlot(
    menu: BarMenu,
    probe: RectProbe,
    pill: SlotPill,
    children: Element,
) -> Element {
    let bar = use_context::<MenuBar>();
    let settings = use_settings();
    let style = format!("height:{}px", settings().bar.title_hit_height_px.0);
    let open = if bar.is_open(menu) {
        Shown::Visible
    } else {
        Shown::Hidden
    };
    let title = move || probe.rect().map(|rect| (menu, rect));
    let send = move |input: MenuIn| {
        bar.feed(BarInput::Bar {
            input,
            title: title(),
        });
    };
    rsx! {
        div {
            class: "sill-bar-title",
            style,
            onpointerdown: move |event| {
                event.stop_propagation();
                send(MenuIn::PressTitle(menu.id()));
            },
            onpointerup: move |event| {
                event.stop_propagation();
                send(MenuIn::Release(Target::Title(menu.id())));
            },
            onpointerenter: move |event| {
                send(MenuIn::Move(pointer_at(&event), Target::Title(menu.id())));
            },
            match pill {
                SlotPill::Item(emphasis) => rsx! {
                    MenuBarItem { open, emphasis, {children} }
                },
                SlotPill::Control => children,
            }
        }
    }
}
