//! The effects end of the desktop menu: open the popup at a pointer press ([`use_context_menu`]
//! does the plumbing) and open Settings at its Wallpaper page when Change Wallpaper... is picked.

use dioxus::prelude::*;
use shell_host::SurfaceHandle;
use sill_launcher::SettingsLink;
use sill_model::launch::{LaunchCommand, SETTINGS_BUS_NAME, SettingsOpen};
use sill_ui_kit::hooks::use_services;
use sill_ui_kit::menu::{PopupGeom, PopupLook, point_anchor, use_context_menu};
use sill_ui_kit::surface::token_from;

use super::DesktopPoint;
use super::model::{DesktopMenu, DesktopPick};

/// The document's `data-surface`: what a user stylesheet names the menu by.
const DESKTOP_MENU: &str = "desktop-menu";

/// How far the card's corner is from the pointer, on each axis: a pixel, so the button's release
/// at the same place is not over the first entry (a release over an entry after a press outside
/// the menu picks it).
const GEOM: PopupGeom = PopupGeom::AtPoint { clear: 1 };

/// Settings' page for the wallpaper, by the id the schema declares it under.
const WALLPAPER_PAGE: &str = "page:wallpaper";

/// The desktop menu of the wallpaper: the callback opens it with its corner at the point, drawn in
/// `look`. Nothing opens without a host (a test render).
pub(in crate::wallpaper) fn use_desktop_menu(look: PopupLook) -> Callback<DesktopPoint> {
    let surface = try_use_context::<SurfaceHandle>();
    let launch = use_services().launch.control;
    let mut looks = use_hook(|| CopyValue::new(look));
    looks.set(look);
    let menu = use_context_menu::<DesktopMenu>(DESKTOP_MENU, move |pick| match pick {
        DesktopPick::ChangeWallpaper => open_wallpaper_page(surface.clone(), launch.clone()),
    });
    Callback::new(move |at: DesktopPoint| {
        menu.open(DesktopMenu, *looks.peek(), GEOM, point_anchor(at.x, at.y));
    })
}

/// Open Settings at the Wallpaper page, once the compositor has given the wallpaper an activation
/// token for it (so the window it opens is let to the front). No other page stands in for it.
fn open_wallpaper_page(
    surface: Option<SurfaceHandle>,
    launch: sill_services::control::Control<LaunchCommand>,
) {
    spawn(async move {
        let token = token_from(surface.as_ref(), Some(SETTINGS_BUS_NAME.to_owned())).await;
        let open = SettingsOpen::at(SettingsLink(WALLPAPER_PAGE.to_owned())).with_token(token);
        let _gone_is_dropped = launch.send(LaunchCommand::OpenSettings(open));
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
