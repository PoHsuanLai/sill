//! The switcher surface's root: one transparent Osd root over the whole output, holding a key
//! target (focused per opening, so the surface's keys reach the machine: Blitz sends a key to
//! the focused element, else to `<html>`) and, while the machine is `Shown`, quire's
//! `AppSwitcher` centred in it.
//!
//! [`super::drive`] follows the [`super::SwitcherLink`] for this output and runs the machine;
//! this file only draws: the tiles from [`super::tiles`] over the opening's apps (a quit tile
//! `TilePresence::Leaving` until its fold settles), `selected` the machine's selection as
//! [`super::key_of`], `metrics` [`super::metrics_of`], `output` [`super::output_width`], and
//! `onhover` / `onactivate` as `Hover` / `Click`. Style nothing under `.ds-switcher*`.

use dioxus::prelude::*;
use ds::{AppKey, BlurState, Ds, Material, RootChrome, RootExtent, SwitcherApp as Tile};
use shell_host::{OutputInfo, SurfaceHandle};
use sill_model::ids::OutputName;

use super::drive::{SwitcherCtx, use_switcher};
use super::icon::SwitcherIcons;
use super::layer::PANEL_ID;
use super::link::SwitcherLink;
use super::machine::{Switcher, SwitcherParams};
use super::session::{Session, Windows};
use super::tiles::{key_of, metrics_of, output_width, tiles};
use super::watch::ChordWatch;
use crate::app_icons::{IconContext, LogicalPx, icon_look, use_icon_resolver};
use crate::hooks::{use_appearance, use_ds_scale, use_service, use_service_part, use_settings};
use crate::surfaces::launcher::panel::active_look;
use sill_services::{AppsService, CosmicWlService};

/// The switcher's stylesheet (lint-clean under `Profile::Strict`, `tests/switcher_lint.rs`).
pub(crate) const SWITCHER_CSS: &str = include_str!("../../style/switcher.css");

pub(super) fn switcher_panel(output: OutputInfo) -> Element {
    let ds_scale = use_ds_scale();
    let file = use_settings()();
    let environment = use_appearance()();
    let on = output.name.clone().map(OutputName);
    let look = active_look(
        &use_service_part::<CosmicWlService, _>(|wl| wl.workspaces.clone())(),
        on.as_ref(),
        &file.spaces,
    );
    let appearance = environment.settings.appearance.appearance();
    let resolved = ds::resolve(appearance, look.theme, environment.system);
    let level = resolved.motion;
    let icons = SwitcherIcons {
        cx: IconContext {
            look: icon_look(
                &environment.settings.icons,
                &look,
                appearance.accent,
                resolved.scheme,
            ),
            scale: ds_scale.unwrap_or(ds::Scale::ONE),
            resolver: use_icon_resolver(),
        },
        index: use_service::<AppsService>()(),
        side: LogicalPx(file.switcher.icon_size_px.0),
    };
    let link = use_context::<SwitcherLink>();
    let ctx = use_switcher(&link, output.id, SwitcherParams::new(&file.switcher), level);
    let state = (ctx.state)();
    let session = (ctx.session)();
    let row = match (state, &session) {
        (Switcher::Shown { sel }, Some(session)) => Some(row(session, sel, &icons)),
        _ => None,
    };
    let host = try_use_context::<SurfaceHandle>();
    let metrics = metrics_of(&file.switcher);
    let width = output_width(output.logical.width);
    rsx! {
        Ds {
            scale: ds_scale,
            appearance,
            system: environment.system,
            look,
            material: Material::Osd,
            chrome: Some(RootChrome::Transparent),
            extent: RootExtent::Viewport,
            blur: BlurState::Available,
            tint_alpha: Some(environment.tint_alpha()),
            stack: Some(environment.material_stack()),
            style { {SWITCHER_CSS} }
            if let Some(session) = &session {
                div {
                    key: "{session.serial.0}",
                    class: "sill-switcher",
                    tabindex: "0",
                    onmounted: move |event: MountedEvent| ds::focus_soon(event.data()),
                    onkeydown: move |event: KeyboardEvent| ctx.key_down(&event),
                    onkeyup: move |event: KeyboardEvent| ctx.key_up(&event),
                    {chord_watch(host.as_ref(), session, ctx)}
                    if let Some((apps, selected)) = row {
                        div { id: PANEL_ID, class: "sill-switcher-panel",
                            ds::AppSwitcher {
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
    let mut row = tiles(&session.apps, &icons.index, |app| icons.source(app));
    for (tile, app) in row.iter_mut().zip(&session.apps) {
        tile.presence = session.presence(&app.app);
    }
    let selected = session
        .apps
        .get(sel.min(session.apps.len().saturating_sub(1)))
        .map_or(AppKey(String::new()), |app| key_of(&app.app));
    (row, selected)
}
