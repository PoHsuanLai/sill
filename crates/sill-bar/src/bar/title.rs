//! A bar title or status item's hit box: it feeds the menu session its presses, releases and
//! pointer entries (design/13 §13.3.2: open on press, click mode on release over the title,
//! hover switch on entry). Its child is a quire `MenuBarItem`, a title or a status glyph, which
//! draws the hover and open pill itself.

use dioxus::prelude::*;
use ds::{RectProbe, Shown};
use shell_host::Point;

use super::io::MenuBar;
use super::menus::BarMenu;
use super::model::BarInput;
use super::track::{Event, Target, point};
use sill_ui_kit::hooks::use_settings;

/// The pointer's position in the bar, whole logical pixels.
pub(crate) fn pointer_at(event: &PointerEvent) -> Point {
    let at = event.client_coordinates();
    Point {
        x: at.x.round() as i32,
        y: at.y.round() as i32,
    }
}

/// Whether `menu` is the bar's open one: its item's `shown`. Read where the item is drawn.
pub(crate) fn shown_of(menu: BarMenu) -> Shown {
    if use_context::<MenuBar>().is_open(menu) {
        Shown::Visible
    } else {
        Shown::Hidden
    }
}

/// The hit box around a title's ds button. `probe` measures that button (its `mounted`), and
/// its rect travels with every event so the popup can be placed under it.
#[component]
pub(crate) fn TitleSlot(menu: BarMenu, probe: RectProbe, children: Element) -> Element {
    let bar = use_context::<MenuBar>();
    let settings = use_settings();
    let style = format!("height:{}px", settings().bar.title_hit_height_px.0);
    let title = move || probe.rect().map(|rect| (menu, rect));
    let send = move |input: Event| {
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
                send(Event::PressTitle(menu.id()));
            },
            onpointerup: move |event| {
                event.stop_propagation();
                send(Event::Release(Target::Title(menu.id())));
            },
            onpointerenter: move |event| {
                send(Event::Move(point(pointer_at(&event)), Target::Title(menu.id())));
            },
            {children}
        }
    }
}
