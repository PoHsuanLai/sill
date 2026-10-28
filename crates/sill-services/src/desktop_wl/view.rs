//! The wire state as the shell's [`DesktopView`]: apps grouped and ranked, windows, Spaces and
//! what is active. Grouping and recency are sill's: the protocols give windows, not apps.

use casement_protocols::vocab::{
    AppLifeKind, AppVisibility, Attention, Emphasis, LeaseState, Modality, PlacementMark,
    ProofKind, Responsiveness, SensitivityLevel, Shown, SpaceKind, WindowRole, WindowStateKind,
};
use desktop_client::toplevel::Toplevel as WireToplevel;
use desktop_client::workspace::WorkspaceFlag;
use desktop_client::{DesktopState, ToplevelKey};
use sill_model::desktop::{
    ActiveView, AppView, DesktopView, SpacePlaceView, SpaceView, WindowView,
};
use sill_model::windows::{Activity, Sizing, Toplevel, Toplevels, WindowState};

use super::ids::space_id;
use super::workspaces::{group_of, ordered};

/// The view of `state`, windows in the recency order of `recency` (the projected
/// [`Toplevels`], most recently activated first): each window is its [`Toplevel`] in the
/// desktop's vocabulary, so the wire's flags are read once, in `toplevels`.
pub fn desktop_view(state: &DesktopState, recency: &Toplevels) -> DesktopView {
    let windows: Vec<WindowView> = recency.windows.iter().map(window).collect();
    DesktopView {
        apps: apps(&windows),
        active: active(&windows),
        windows,
        spaces: spaces(state),
    }
}

/// The name a window is quoted by: the compositor's identifier, else a name made from the key
/// (a compositor without the toplevel list gives none).
pub fn identifier(toplevel: &WireToplevel) -> String {
    toplevel
        .identifier
        .as_ref()
        .map_or_else(|| made_up(toplevel.key), |id| id.as_str().to_owned())
}

fn made_up(key: ToplevelKey) -> String {
    format!("toplevel-{}", key.get())
}

fn window(toplevel: &Toplevel) -> WindowView {
    WindowView {
        identifier: toplevel.identifier.clone(),
        app: toplevel.app_id.clone(),
        title: toplevel.title.clone(),
        state: window_state(toplevel),
        space: SpacePlaceView::Unknown,
        output: toplevel
            .outputs
            .first()
            .map(|o| o.0.clone())
            .unwrap_or_default(),
        emphasis: emphasis(toplevel.activity),
        placement: placement(toplevel.sizing),
        role: WindowRole::Document,
        modality: Modality::None,
        geometry: None,
        responsiveness: Responsiveness::Responding,
        lease: LeaseState::Free,
        sensitivity: SensitivityLevel::Normal,
    }
}

/// The casement vocabulary's one word for minimised, full screen, filled or floating.
fn window_state(toplevel: &Toplevel) -> WindowStateKind {
    match (toplevel.window, toplevel.sizing) {
        (WindowState::Minimized, _) => WindowStateKind::Minimised,
        (WindowState::Normal, Sizing::Fullscreen) => WindowStateKind::FullScreen,
        (WindowState::Normal, Sizing::Maximized) => WindowStateKind::Filled,
        (WindowState::Normal, Sizing::Floating) => WindowStateKind::Floating,
    }
}

/// Only the key window is ever `Key`: the standard protocols know no main window.
fn emphasis(activity: Activity) -> Emphasis {
    match activity {
        Activity::Active => Emphasis::Key,
        Activity::Inactive => Emphasis::Inactive,
    }
}

fn placement(sizing: Sizing) -> PlacementMark {
    match sizing {
        Sizing::Fullscreen => PlacementMark::FullScreen,
        Sizing::Maximized => PlacementMark::Fill,
        Sizing::Floating => PlacementMark::Free,
    }
}

/// One app per distinct, non-empty app id, ranked by its most recent window.
fn apps(windows: &[WindowView]) -> Vec<AppView> {
    let mut keys: Vec<&str> = Vec::new();
    for window in windows.iter().filter(|w| !w.app.trim().is_empty()) {
        if !keys.contains(&window.app.as_str()) {
            keys.push(&window.app);
        }
    }
    keys.into_iter()
        .zip(0u32..)
        .map(|(key, mru)| AppView {
            key: key.to_owned(),
            proof: ProofKind::Claimed,
            life: AppLifeKind::Running,
            attention: Attention::None,
            mru,
            visibility: AppVisibility::Shown,
        })
        .collect()
}

fn active(windows: &[WindowView]) -> ActiveView {
    let key = windows.iter().find(|w| w.emphasis == Emphasis::Key);
    ActiveView {
        app: key.map(|w| w.app.clone()),
        key_window: key.map(|w| w.identifier.clone()),
    }
}

/// One Space per workspace the compositor lists, in each group's order.
fn spaces(state: &DesktopState) -> Vec<SpaceView> {
    state
        .workspaces
        .groups()
        .map(|g| Some(g.key))
        .chain(std::iter::once(None))
        .flat_map(|group| {
            ordered(state, move |w| w.group == group)
                .into_iter()
                .zip(0u32..)
                .map(|(workspace, position)| SpaceView {
                    id: space_id(workspace),
                    kind: SpaceKind::Desktop,
                    output: group_of(state, workspace)
                        .and_then(|g| g.outputs.first())
                        .map(|o| o.as_str().to_owned())
                        .unwrap_or_default(),
                    position,
                    shown: if workspace.flags.contains(&WorkspaceFlag::Active) {
                        Shown::Shown
                    } else {
                        Shown::Hidden
                    },
                })
                .collect::<Vec<_>>()
        })
        .collect()
}
