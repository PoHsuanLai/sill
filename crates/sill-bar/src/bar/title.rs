//! A bar title or status item's pointer: its quire `MenuBarItem`, a title or a status glyph, hands
//! the menu session its presses, releases and pointer entries (design/13 §13.3.2: open on press,
//! click mode on release over the title, hover switch on entry) and draws the hover and open pill
//! itself.

use dioxus::prelude::*;
use ds::{RectProbe, Shown};
use ds_shell::BarPointer;
use shell_host::Point;

use super::io::MenuBar;
use super::menus::BarMenu;
use super::model::BarInput;
use super::track::{Event, Target, point};

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

/// What a title's ds button hands to the menu session: its presses, releases and pointer entries.
/// `probe` measures that button (its `mounted`), and its rect travels with every event so the
/// popup can be placed under it.
pub(crate) fn use_title_pointer(menu: BarMenu, probe: RectProbe) -> BarPointer {
    let bar = use_context::<MenuBar>();
    let title = move || probe.rect().map(|rect| (menu, rect));
    let send = move |input: Event| {
        bar.feed(BarInput::Bar {
            input,
            title: title(),
        });
    };
    BarPointer {
        down: Some(EventHandler::new(move |event: PointerEvent| {
            event.stop_propagation();
            send(Event::PressTitle(menu.id()));
        })),
        up: Some(EventHandler::new(move |event: PointerEvent| {
            event.stop_propagation();
            send(Event::Release(Target::Title(menu.id())));
        })),
        enter: Some(EventHandler::new(move |event: PointerEvent| {
            send(Event::Move(
                point(pointer_at(&event)),
                Target::Title(menu.id()),
            ));
        })),
    }
}
