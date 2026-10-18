//! The effects end of the window menu: follow the [`WindowMenuLink`], open the menu's popup at
//! the point an opening names when it is on this output, and write the entry the popup reports
//! back to the link. A dismissal writes nothing.

use dioxus::prelude::*;
use shell_host::{HostRoot, OutputInfo, SurfaceHandle, use_watch};
use sill_ui_kit::links::{MenuSerial, WindowMenuLink};
use sill_ui_kit::menu::{ContextMenuLink, MenuReport, PopupLook, context_menu_popup};
use sill_ui_kit::settings::SettingsFeed;
use tokio::sync::mpsc;

use super::geometry::{local_point, menu_config, menu_placement};
use super::model::WindowMenuModel;
use sill_model::desktop::WindowEntry;

/// The document's `data-surface`: what a user stylesheet names the menu by.
const WINDOW_MENU: &str = "window-menu";

/// Whether the surface has its menu's popup up. A second popup may not be created while one
/// holds the grab, so an opening that arrives meanwhile is not shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Popup {
    Up,
    Down,
}

/// Run the menu of the surface on `output`: `look` is how its popup is drawn (read again at
/// each opening).
pub(super) fn use_window_menu(link: &WindowMenuLink, output: &OutputInfo, look: PopupLook) {
    let surface = try_use_context::<SurfaceHandle>();
    let feed = try_use_context::<SettingsFeed>();
    let latest = use_watch(link.subscribe());
    // A surface respawned later does not replay the opening before it.
    let mut seen =
        use_hook(|| CopyValue::new(link.latest().map_or_else(MenuSerial::default, |o| o.serial)));
    let mut popup = use_hook(|| CopyValue::new(Popup::Down));
    let mut window = use_hook(|| CopyValue::new(None::<String>));
    let mut looks = use_hook(|| CopyValue::new(look));
    looks.set(look);
    let (reports, inbox) = use_hook(|| {
        let (tx, rx) = mpsc::unbounded_channel::<MenuReport<WindowEntry>>();
        (tx, CopyValue::new(Some(rx)))
    });
    use_hook({
        let link = link.clone();
        move || listen(link, popup, window, inbox)
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
        let Some(surface) = surface.clone() else {
            return;
        };
        if *popup.peek() == Popup::Up {
            return;
        }
        popup.set(Popup::Up);
        window.set(Some(opening.window));
        let mut root =
            HostRoot::new(context_menu_popup::<WindowMenuModel>).context(ContextMenuLink {
                model: WindowMenuModel {
                    enabled: opening.enabled,
                },
                look: *looks.peek(),
                surface: WINDOW_MENU,
                reports: reports.clone(),
            });
        if let Some(feed) = feed.clone() {
            root = root.context(feed);
        }
        surface.open_popup(menu_config(), menu_placement(local), root);
    });
}

/// The task that writes each pick the popup reports to the link, and notes when the popup is
/// gone.
fn listen(
    link: WindowMenuLink,
    popup: CopyValue<Popup>,
    window: CopyValue<Option<String>>,
    inbox: CopyValue<Option<mpsc::UnboundedReceiver<MenuReport<WindowEntry>>>>,
) {
    let mut inbox = inbox;
    let Some(mut rx) = inbox.write().take() else {
        return;
    };
    let mut popup = popup;
    spawn(async move {
        while let Some(report) = rx.recv().await {
            match report {
                MenuReport::Picked(entry) => {
                    if let Some(window) = window.peek().clone() {
                        link.pick(window, entry);
                    }
                }
                MenuReport::Dismissed | MenuReport::Gone => popup.set(Popup::Down),
            }
        }
    });
}
