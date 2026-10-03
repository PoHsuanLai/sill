//! The effects end of the desktop menu: open the popup at a pointer press, listen to what it
//! reports, and carry out what a pick asks for ([`super::pick`]).

use dioxus::prelude::*;
use shell_host::{HostRoot, OutputId, OutputInfo, SurfaceHandle};
use sill_launcher::{AppId, DesktopAction};
use sill_model::launch::{LaunchCommand, LaunchRequest};
use sill_ui_kit::hooks::use_services;
use sill_ui_kit::links::{EditWidgetsLink, WidgetsChange};
use sill_ui_kit::menu::{ContextMenuLink, MenuReport, PopupLook, context_menu_popup};
use sill_ui_kit::services::Services;
use sill_ui_kit::settings::SettingsFeed;
use tokio::sync::mpsc;

use super::geometry::{menu_config, menu_placement};
use super::model::MenuWorld;
use super::pick::{DesktopCommand, commands};
use crate::desktop::arrange::LayerPoint;

/// The document's `data-surface`: what a user stylesheet names the menu by.
const DESKTOP_MENU: &str = "desktop-menu";

/// Where the carried-out asks go: the services (the launch), the Edit Widgets link (its opening
/// and its writer), and the output the menu belongs to.
#[derive(Clone)]
struct Outward {
    services: Services,
    surface: Option<SurfaceHandle>,
    link: Option<EditWidgetsLink>,
    output: OutputId,
}

/// What the menu is opened with, from the layer's latest render.
#[derive(Clone)]
struct Deps {
    world: MenuWorld,
    look: PopupLook,
}

/// The desktop menu of the layer on `output`: the callback opens it with its corner at the
/// point, over `world` and drawn in `look`. Nothing opens without a host (a test render).
pub(in crate::desktop) fn use_desktop_menu(
    output: &OutputInfo,
    world: MenuWorld,
    look: PopupLook,
) -> Callback<LayerPoint> {
    let surface = try_use_context::<SurfaceHandle>();
    let feed = try_use_context::<SettingsFeed>();
    let outward = Outward {
        services: use_services(),
        surface: surface.clone(),
        link: try_use_context::<EditWidgetsLink>(),
        output: output.id,
    };
    let (reports, inbox) = use_hook(|| {
        let (tx, rx) = mpsc::unbounded_channel::<MenuReport<super::model::DesktopPick>>();
        (tx, CopyValue::new(Some(rx)))
    });
    let mut deps = use_hook(|| {
        CopyValue::new(Deps {
            world: world.clone(),
            look,
        })
    });
    deps.set(Deps { world, look });
    use_hook(move || listen(deps, outward, inbox));
    let to_open = reports.clone();
    Callback::new(move |at: LayerPoint| {
        let Some(surface) = surface.clone() else {
            return;
        };
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

/// The task that carries out each pick the popup reports, over the world the layer last drew.
fn listen(
    deps: CopyValue<Deps>,
    outward: Outward,
    inbox: CopyValue<Option<mpsc::UnboundedReceiver<MenuReport<super::model::DesktopPick>>>>,
) {
    let mut inbox = inbox;
    let Some(mut rx) = inbox.write().take() else {
        return;
    };
    spawn(async move {
        while let Some(report) = rx.recv().await {
            if let MenuReport::Picked(pick) = report {
                let world = deps.peek().world.clone();
                for command in commands(pick, &world, outward.output) {
                    carry_out(&outward, command);
                }
            }
        }
    });
}

/// Do one ask. A service that has stopped drops its commands: there is nobody left to tell.
fn carry_out(outward: &Outward, command: DesktopCommand) {
    match command {
        DesktopCommand::OpenPicker(app) => open_picker(outward.clone(), app),
        DesktopCommand::EditWidgets(output) => {
            if let Some(link) = &outward.link {
                link.open(output);
            }
        }
        DesktopCommand::SetView(view) => {
            if let Some(link) = &outward.link {
                link.write(WidgetsChange::View(view));
            }
        }
    }
}

/// Launch `app` through the launch service, once the compositor has given the layer an
/// activation token for it (so the window it opens is let to the front).
fn open_picker(outward: Outward, app: AppId) {
    spawn(async move {
        let token = match &outward.surface {
            Some(surface) => surface.activation_token(Some(app.0.clone())).await,
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
