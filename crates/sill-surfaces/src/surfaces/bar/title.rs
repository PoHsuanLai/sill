//! A bar title or status item's hit box: it feeds the menu session its presses, releases and
//! pointer entries (design/13 §13.3.2: open on press, click mode on release over the title,
//! hover switch on entry), and shows the open-title pill while its menu is open.

use dioxus::prelude::*;
use ds::RectProbe;
use shell_host::Point;

use super::menu_track::{MenuIn, Target};
use super::menus::BarMenu;
use super::popup::MenuSide;
use super::session::{BarInput, MenuBar};
use crate::hooks::use_settings;

/// The pointer's position in the bar, whole logical pixels.
pub(crate) fn pointer_at(event: &PointerEvent) -> Point {
    let at = event.client_coordinates();
    Point {
        x: at.x.round() as i32,
        y: at.y.round() as i32,
    }
}

/// Who draws the open-title pill (design/13 §13.3.1) while a slot's menu is open.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SlotPill {
    /// The slot's own background: a text title (the app name, the clock).
    Slot,
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
    let bar_settings = settings().bar;
    let open = match (pill, bar.is_open(menu)) {
        (SlotPill::Slot, true) => "true",
        (SlotPill::Slot | SlotPill::Control, _) => "false",
    };
    let pad = match menu.side() {
        MenuSide::Title => bar_settings.title_padding_px.0,
        MenuSide::Status => 0,
    };
    let style = format!(
        "height:{}px;padding:0 {pad}px",
        bar_settings.title_hit_height_px.0
    );
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
            "data-open": open,
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
            {children}
        }
    }
}
