//! The effects end of the desktop menu: open the popup at a pointer press, listen to what it
//! reports, and launch the wallpaper picker when it is picked.

use dioxus::prelude::*;
use shell_host::{HostRoot, SurfaceHandle};
use sill_launcher::{AppId, DesktopAction};
use sill_model::launch::{LaunchCommand, LaunchRequest};
use sill_ui_kit::hooks::use_services;
use sill_ui_kit::menu::{ContextMenuLink, MenuReport, PopupLook, context_menu_popup};
use sill_ui_kit::services::Services;
use sill_ui_kit::settings::SettingsFeed;
use tokio::sync::mpsc;

use super::DesktopPoint;
use super::geometry::{menu_config, menu_placement};
use super::model::DesktopPick;
use super::model::MenuWorld;

/// The document's `data-surface`: what a user stylesheet names the menu by.
const DESKTOP_MENU: &str = "desktop-menu";

/// Whether the wallpaper has its menu's popup up. A second popup may not be created while one holds
/// the grab (xdg-shell: the compositor ends the connection), so a press that would open another
/// waits for the first one to be gone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Popup {
    Up,
    Down,
}

/// Where the carried-out asks go: the services (the launch) and the surface (its activation
/// token).
#[derive(Clone)]
struct Outward {
    services: Services,
    surface: Option<SurfaceHandle>,
}

/// What the menu is opened with, from the wallpaper's latest render.
#[derive(Clone)]
struct Deps {
    world: MenuWorld,
    look: PopupLook,
}

/// The desktop menu of the wallpaper: the callback opens it with its corner at the point, over
/// `world` and drawn in `look`. Nothing opens without a host (a test render) or without an entry.
pub(in crate::wallpaper) fn use_desktop_menu(
    world: MenuWorld,
    look: PopupLook,
) -> Callback<DesktopPoint> {
    let surface = try_use_context::<SurfaceHandle>();
    let feed = try_use_context::<SettingsFeed>();
    let outward = Outward {
        services: use_services(),
        surface: surface.clone(),
    };
    let (reports, inbox) = use_hook(|| {
        let (tx, rx) = mpsc::unbounded_channel::<MenuReport<DesktopPick>>();
        (tx, CopyValue::new(Some(rx)))
    });
    let mut deps = use_hook(|| {
        CopyValue::new(Deps {
            world: world.clone(),
            look,
        })
    });
    deps.set(Deps { world, look });
    let mut popup = use_hook(|| CopyValue::new(Popup::Down));
    use_hook(move || listen(deps, popup, outward, inbox));
    let to_open = reports;
    Callback::new(move |at: DesktopPoint| {
        let Some(surface) = surface.clone() else {
            return;
        };
        if *popup.peek() == Popup::Up || deps.peek().world.picker.is_none() {
            return;
        }
        popup.set(Popup::Up);
        let held = deps.peek().clone();
        let mut root = HostRoot::new(context_menu_popup::<MenuWorld>).context(ContextMenuLink {
            model: held.world,
            look: held.look,
            surface: DESKTOP_MENU,
            reports: to_open.clone(),
        });
        if let Some(feed) = feed.clone() {
            root = root.context(feed);
        }
        surface.open_popup(menu_config(), menu_placement(at), root);
    })
}

/// The task that carries out each pick the popup reports, over the world the wallpaper last drew,
/// and notes when the popup is gone.
fn listen(
    deps: CopyValue<Deps>,
    popup: CopyValue<Popup>,
    outward: Outward,
    inbox: CopyValue<Option<mpsc::UnboundedReceiver<MenuReport<DesktopPick>>>>,
) {
    let mut inbox = inbox;
    let Some(mut rx) = inbox.write().take() else {
        return;
    };
    let mut popup = popup;
    spawn(async move {
        while let Some(report) = rx.recv().await {
            match report {
                MenuReport::Picked(DesktopPick::ChangeWallpaper) => {
                    if let Some(app) = deps.peek().world.picker.clone() {
                        open_picker(outward.clone(), app);
                    }
                }
                MenuReport::Dismissed | MenuReport::Gone => popup.set(Popup::Down),
            }
        }
    });
}

/// Launch `app` through the launch service, once the compositor has given the wallpaper an
/// activation token for it (so the window it opens is let to the front).
fn open_picker(outward: Outward, app: AppId) {
    spawn(async move {
        let token = match &outward.surface {
            Some(surface) => {
                sill_ui_kit::surface::activation_token(surface, Some(app.0.clone())).await
            }
            None => None,
        };
        let _gone_is_dropped =
            outward
                .services
                .launch
                .control
                .send(LaunchCommand::Launch(LaunchRequest {
                    app,
                    action: DesktopAction::Default,
                    files: Vec::new(),
                    uris: Vec::new(),
                    token,
                }));
    });
}

/// A press of the secondary button on the wallpaper opens the desktop's menu with its corner
/// there; any other button does nothing here.
pub(in crate::wallpaper) fn secondary_press(
    event: &Event<MouseData>,
    open: Callback<DesktopPoint>,
) {
    use dioxus::html::input_data::MouseButton;
    use dioxus::html::point_interaction::{InteractionLocation, PointerInteraction};
    if event.trigger_button() == Some(MouseButton::Secondary) {
        let at = event.client_coordinates();
        open.call(DesktopPoint {
            x: at.x.round() as i32,
            y: at.y.round() as i32,
        });
    }
}
