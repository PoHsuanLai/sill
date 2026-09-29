//! The context menu popup's surface end: it reports a pick, a dismissal and its own drop to the
//! dock, and closes itself once the menu has closed; the document it draws is
//! the ui-kit `MenuPopup`'s. A popup is a `VirtualDom` of its own, so it provides quire's measurer
//! itself.

use dioxus::prelude::*;
use ds::{Anchor, MenuEntry, Point, Px};
use shell_host::{Size, use_surface};
use tokio::sync::mpsc;

use crate::menu::{MenuPopup, PopupLook};
use crate::root::HostModalityBridge;
use crate::surfaces::dock::menu::DockMenu;
use crate::surfaces::dock::menu_geometry::{menu_point, room};
use crate::surfaces::dock::pick::DockPick;

/// What the popup tells the dock.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PopupReport {
    Picked(DockPick),
    Dismissed,
    /// The popup's document went away (closed by the compositor, or after a report).
    Gone,
}

/// The popup's root context.
#[derive(Debug, Clone)]
pub(crate) struct MenuLink {
    pub entries: Vec<MenuEntry<DockPick>>,
    pub size: Size,
    pub look: PopupLook,
    pub reports: mpsc::UnboundedSender<PopupReport>,
}

/// The popup root.
#[component]
pub(crate) fn DockMenuPopup() -> Element {
    ds_native::measure::provide();
    ds_native::focus::provide();
    let link = use_context::<MenuLink>();
    let surface = use_surface();
    let gone = link.reports.clone();
    use_drop(move || {
        // The dock may be gone first; then nobody is left to tell.
        let _ = gone.send(PopupReport::Gone);
    });
    let mut picked = use_hook(|| CopyValue::new(false));
    let on_pick = {
        let reports = link.reports.clone();
        move |pick: DockPick| {
            picked.set(true);
            let _ = reports.send(PopupReport::Picked(pick));
        }
    };
    // ds `Menu` calls `onpick` and then `onclose`: a close reports a dismissal only when
    // nothing was picked, then closes the popup.
    let on_close = {
        let reports = link.reports.clone();
        let surface = surface.clone();
        move |()| {
            let reports = reports.clone();
            let surface = surface.clone();
            spawn(async move {
                if !picked() {
                    let _ = reports.send(PopupReport::Dismissed);
                }
                surface.close();
            });
        }
    };
    use_context_provider(|| link.look);
    use_context_provider(room);
    let (x, y) = menu_point(link.size);
    rsx! {
        HostModalityBridge {
            MenuPopup {
                model: DockMenu { entries: link.entries.clone() },
                anchor: Anchor::Point(Point { x: Px(x), y: Px(y) }),
                on_pick,
                on_close,
            }
        }
    }
}
