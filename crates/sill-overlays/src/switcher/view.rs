//! The switcher surface's root: one transparent Osd root over the whole output, holding a key
//! target (focused per opening, so the surface's keys reach the machine: Blitz sends a key to
//! the focused element, else to `<html>`) and, while the machine is `Shown`, quire's
//! `AppSwitcher` centred in it.
//!
//! [`super::io`] follows the [`sill_ui_kit::links::SwitcherLink`] for this output and runs the machine;
//! this file only draws: the tiles from [`super::tiles`] over the opening's apps (a quit tile is left out of the list
//! while its fold plays: quire keeps drawing it until it settles), `selected` the machine's selection as
//! [`super::key_of`], `metrics` [`super::metrics_of`], `output` [`super::output_width`], and
//! `onhover` / `onactivate` as `Hover` / `Click`. Style nothing under `.ds-switcher*`.

use dioxus::prelude::*;
use ds::focus::soon::focus_soon;
use ds::prelude::*;
use shell_host::{OutputInfo, SurfaceHandle};
use sill_shell_kit::prelude::*;
use sill_shell_kit::switcher::app_switcher::AppKey;
use sill_shell_kit::switcher::app_switcher::SwitcherApp as Tile;

use super::icon::SwitcherIcons;
use super::io::{SwitcherCtx, use_switcher};
use super::layer::PANEL_ID;
use super::pointer::At;
use super::session::{Session, Windows};
use super::tiles::{key_of, metrics_of, output_width, tiles};
use super::watch::ChordWatch;
use ds_behaviour::switcher::Phase;
use sill_services::AppsService;
use sill_ui_kit::app_icons::{IconContext, LogicalPx, icon_look, use_icon_resolver};
use sill_ui_kit::hooks::{use_appearance, use_service, use_settings};
use sill_ui_kit::links::SwitcherLink;
use sill_ui_kit::look::use_active_look;
use sill_ui_kit::root::{SurfaceRoot, use_host_scale};

/// The switcher's stylesheet (lint-clean under `Profile::Strict`, `crates/sill/tests/lint.rs`).
pub const SWITCHER_CSS: &str = include_str!("style.css");

pub(super) fn switcher(output: OutputInfo) -> Element {
    let file = use_settings()();
    let environment = use_appearance()();
    let look = use_active_look(&output);
    let appearance = environment.settings.appearance.appearance();
    let resolved = resolve(appearance, look.theme, environment.system);
    let level = resolved.motion;
    let icons = SwitcherIcons {
        cx: IconContext {
            look: icon_look(
                &environment.settings.icons,
                &look,
                appearance.accent,
                resolved.scheme,
            ),
            scale: use_host_scale(),
            resolver: use_icon_resolver(),
        },
        index: use_service::<AppsService>()(),
        side: LogicalPx(file.switcher.icon_size_px.0),
    };
    let link = use_context::<SwitcherLink>();
    let ctx = use_switcher(&link, output.id, file.switcher.params(), level);
    let state = (ctx.state)().phase();
    let session = (ctx.session)();
    let row = match (state, &session) {
        (Phase::Shown { sel }, Some(session)) => Some(row(session, sel, &icons)),
        _ => None,
    };
    let host = try_use_context::<SurfaceHandle>();
    let metrics = metrics_of(&file.switcher);
    let width = output_width(output.logical.width);
    rsx! {
        SurfaceRoot {
            output: output.clone(),
            material: Material::Osd,
            chrome: Some(RootChrome::Transparent),
            extent: RootExtent::Viewport,
            AppStyle { css: SWITCHER_CSS }
            if let Some(session) = &session {
                div {
                    key: "{session.serial.0}",
                    class: "sill-switcher",
                    tabindex: "0",
                    onmounted: move |event: MountedEvent| focus_soon(event.data()),
                    onkeydown: move |event: KeyboardEvent| ctx.key_down(&event),
                    onkeyup: move |event: KeyboardEvent| ctx.key_up(&event),
                    onpointermove: move |event: PointerEvent| ctx.pointer_moved(at_of(&event)),
                    {chord_watch(host.as_ref(), session, ctx)}
                    if let Some((apps, selected)) = row {
                        div { id: PANEL_ID, class: "sill-switcher-panel",
                            AppSwitcher {
                                apps,
                                selected,
                                output: width,
                                metrics,
                                onhover: move |key: AppKey| ctx.hover(&key),
                                onactivate: move |key: AppKey| ctx.click(&key),
                            }
                        }
                    }
                }
            }
        }
    }
}

/// An opening follows the modifiers on a host surface (`use_modifiers` needs one), for the
/// compositor's apps.
fn chord_watch(host: Option<&SurfaceHandle>, session: &Session, ctx: SwitcherCtx) -> Element {
    match (host, session.windows) {
        (Some(_), Windows::Live) => rsx! { ChordWatch { ctx } },
        (None, _) | (_, Windows::MadeUp) => rsx! {},
    }
}

/// The tiles and the selected one's key.
fn row(session: &Session, sel: usize, icons: &SwitcherIcons) -> (Vec<Tile>, AppKey) {
    let listed: Vec<_> = session
        .apps
        .iter()
        .filter(|app| session.shown(&app.app) == Shown::Visible)
        .cloned()
        .collect();
    let row = tiles(&listed, &icons.index, |app| icons.source(app));
    let selected = session
        .apps
        .get(sel.min(session.apps.len().saturating_sub(1)))
        .map_or(AppKey(String::new()), |app| key_of(&app.app));
    (row, selected)
}

/// Where `event` is on the surface.
fn at_of(event: &PointerEvent) -> At {
    let at = event.client_coordinates();
    At { x: at.x, y: at.y }
}
