//! The effects end of the window menu: follow the [`WindowMenuLink`], open the menu's popup at
//! the point an opening names when it is on this output, and write the entry the popup reports
//! back to the link. A dismissal writes nothing. The popup plumbing is
//! [`sill_ui_kit::menu::use_context_menu`].

use dioxus::prelude::*;
use shell_host::{OutputInfo, use_watch};
use sill_model::desktop::WindowEntry;
use sill_ui_kit::links::{MenuSerial, WindowMenuLink};
use sill_ui_kit::menu::{PopupGeom, PopupLook, point_anchor, use_context_menu};

use super::geometry::local_point;
use super::model::WindowMenuModel;

/// The document's `data-surface`: what a user stylesheet names the menu by.
const WINDOW_MENU: &str = "window-menu";

/// The card's corner is on the point the compositor named.
const GEOM: PopupGeom = PopupGeom::AtPoint { clear: 0 };

/// Run the menu of the surface on `output`: `look` is how its popup is drawn (read again at
/// each opening).
pub(super) fn use_window_menu(link: &WindowMenuLink, output: &OutputInfo, look: PopupLook) {
    let latest = use_watch(link.subscribe());
    // A surface respawned later does not replay the opening before it.
    let mut seen =
        use_hook(|| CopyValue::new(link.latest().map_or_else(MenuSerial::default, |o| o.serial)));
    let mut window = use_hook(|| CopyValue::new(None::<String>));
    let mut looks = use_hook(|| CopyValue::new(look));
    looks.set(look);
    let menu = use_context_menu::<WindowMenuModel>(WINDOW_MENU, {
        let link = link.clone();
        move |entry: WindowEntry| {
            if let Some(window) = window.peek().clone() {
                link.pick(window, entry);
            }
        }
    });
    let area = output.logical;
    use_effect(move || {
        let Some(opening) = latest.read().clone() else {
            return;
        };
        if opening.serial <= *seen.peek() {
            return;
        }
        seen.set(opening.serial);
        let Some(local) = local_point(opening.at, area) else {
            return;
        };
        if !menu.is_free() {
            return;
        }
        window.set(Some(opening.window));
        menu.open(
            WindowMenuModel {
                enabled: opening.enabled,
            },
            *looks.peek(),
            GEOM,
            point_anchor(local.0, local.1),
        );
    });
}
