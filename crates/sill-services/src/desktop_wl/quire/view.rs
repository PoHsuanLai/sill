//! The Quire state as the shell's [`DesktopView`]: apps from `quire_desktop_app_v1`, windows
//! joined to their ext toplevels, Spaces in order, and what is active.

use casement_protocols::vocab::ProofKind;
use desktop_client::DesktopState;
use desktop_client::quire::{Quire, QuireApp, QuireSpace, QuireWindow, SpacePlace};
use porter_core::SpaceId;
use sill_model::desktop::{
    ActiveView, AppView, DesktopView, SpacePlaceView, SpaceView, WindowView,
};

use crate::desktop_wl::view::identifier;

/// The view of `state`'s Quire arm. Windows come ranked by their app's recency (the protocol
/// gives apps a rank, not windows), then by key.
pub fn quire_view(state: &DesktopState) -> DesktopView {
    let quire = &state.quire;
    let mut apps: Vec<AppView> = quire.apps().filter_map(app).collect();
    apps.sort_by(|a, b| (a.mru, &a.key).cmp(&(b.mru, &b.key)));
    let mut windows: Vec<(u32, u64, WindowView)> = quire
        .windows()
        .filter_map(|w| window(state, w).map(|view| (rank(quire, w), w.key.get(), view)))
        .collect();
    windows.sort_by_key(|(rank, key, _)| (*rank, *key));
    let mut spaces: Vec<SpaceView> = quire.spaces().filter_map(|s| space(state, s)).collect();
    spaces.sort_by(|a, b| (&a.output, a.position).cmp(&(&b.output, b.position)));
    DesktopView {
        apps,
        windows: windows.into_iter().map(|(_, _, view)| view).collect(),
        spaces,
        active: active(state),
    }
}

/// How the shell names an app: the desktop's proven name, else what the app claims.
fn app_name(app: &QuireApp) -> Option<String> {
    app.id
        .as_ref()
        .map(|id| id.as_str().to_owned())
        .or_else(|| Some(app.claimed_id.clone()).filter(|claimed| !claimed.is_empty()))
}

fn app(app: &QuireApp) -> Option<AppView> {
    Some(AppView {
        key: app_name(app)?,
        proof: app.proof.unwrap_or(ProofKind::Claimed),
        life: app.life,
        attention: app.attention,
        mru: app.mru.unwrap_or(u32::MAX),
        visibility: app.visibility,
    })
}

fn rank(quire: &Quire, window: &QuireWindow) -> u32 {
    window
        .app
        .and_then(|key| quire.app(key))
        .and_then(|app| app.mru)
        .unwrap_or(u32::MAX)
}

/// A window with its ext toplevel; one whose toplevel has not arrived is not shown.
fn window(state: &DesktopState, window: &QuireWindow) -> Option<WindowView> {
    let toplevel = state.toplevels.get(window.key)?;
    Some(WindowView {
        identifier: identifier(toplevel),
        app: window
            .app
            .and_then(|key| state.quire.app(key))
            .and_then(app_name)
            .unwrap_or_default(),
        title: toplevel.title.clone(),
        state: window.state,
        space: place(&state.quire, window.place),
        output: window
            .output
            .as_ref()
            .map(|output| output.as_str().to_owned())
            .unwrap_or_default(),
        emphasis: window.emphasis,
        placement: window.placement,
        role: window.role,
        modality: window.modality,
        geometry: window
            .geometry
            .map(|g| (g.rect.x, g.rect.y, g.rect.w, g.rect.h)),
        responsiveness: window.responsiveness,
        lease: window.lease.state,
        sensitivity: window.sensitivity,
    })
}

/// A Space's id in the shell's type; `None` while the Space has not said it or says one the id
/// grammar refuses.
fn space_id(space: &QuireSpace) -> Option<SpaceId> {
    SpaceId::parse(space.id.as_ref()?.as_str()).ok()
}

fn place(quire: &Quire, place: SpacePlace) -> SpacePlaceView {
    match place {
        SpacePlace::On(key) => quire
            .space(key)
            .and_then(space_id)
            .map_or(SpacePlaceView::Unknown, SpacePlaceView::On),
        SpacePlace::AllSpaces => SpacePlaceView::AllSpaces,
        _ => SpacePlaceView::Unknown,
    }
}

fn space(state: &DesktopState, space: &QuireSpace) -> Option<SpaceView> {
    Some(SpaceView {
        id: space_id(space)?,
        kind: space.kind,
        output: space
            .output
            .as_ref()
            .map(|output| output.as_str().to_owned())
            .unwrap_or_default(),
        position: space.position,
        shown: state.space_shown(space.key),
    })
}

fn active(state: &DesktopState) -> ActiveView {
    let active = state.quire.active();
    ActiveView {
        app: active
            .app
            .and_then(|key| state.quire.app(key))
            .and_then(app_name),
        key_window: active
            .key
            .and_then(|key| state.toplevels.get(key))
            .map(identifier),
    }
}
