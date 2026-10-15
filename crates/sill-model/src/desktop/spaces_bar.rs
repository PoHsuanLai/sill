//! What Mission Control's Spaces bar shows and does, from the overview's state and the desktop's
//! view: when the bar is up, which Spaces it lists on an output (named as the Mac names them) and
//! which is marked, and the commands a click on a Space or on the plus sends. Pure.

use casement_protocols::vocab::{OverviewMode, OverviewState, Shown, SpaceKind, Toggle};
use porter_core::SpaceId;

use super::{DesktopCommand, DesktopView, SpacePlaceView};
use crate::apps::AppIndex;
use sill_launcher::AppId;

/// Whether the bar is up.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BarShow {
    /// Not on screen.
    #[default]
    Hide,
    /// On screen, or sliding in or out with Mission Control.
    Show,
}

/// The bar for an overview in `mode` at `state`: up from the moment Mission Control starts to
/// open (a gesture counts) until it starts to close; App Exposé and Show Desktop have no bar.
pub fn bar_show(mode: OverviewMode, state: OverviewState) -> BarShow {
    match (mode, state) {
        (
            OverviewMode::MissionControl,
            OverviewState::Entering | OverviewState::Open | OverviewState::Tracking,
        ) => BarShow::Show,
        _ => BarShow::Hide,
    }
}

/// One Space in the bar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BarSpace {
    /// Its id, as a click on it quotes it.
    pub id: SpaceId,
    /// Its name: "Desktop 3", or the apps of a full-screen or Split View Space.
    pub label: String,
    /// Whether it is the Space on show.
    pub current: bool,
}

/// The Spaces on the output called `output` (`None` for an unnamed output), left to right. A
/// Space the desktop does not place on an output is listed on every output.
pub fn bar_spaces(view: &DesktopView, output: Option<&str>, apps: &AppIndex) -> Vec<BarSpace> {
    let mut here: Vec<_> = view
        .spaces
        .iter()
        .filter(|space| space.output.is_empty() || Some(space.output.as_str()) == output)
        .collect();
    here.sort_by_key(|space| space.position);
    let mut desktops = 0_u32;
    here.into_iter()
        .map(|space| {
            let label = match space.kind {
                SpaceKind::Desktop => {
                    desktops += 1;
                    format!("Desktop {desktops}")
                }
                kind => app_label(view, &space.id, kind, apps),
            };
            BarSpace {
                id: space.id.clone(),
                label,
                current: space.shown == Shown::Shown,
            }
        })
        .collect()
}

/// The apps with a window on the Space, as the Mac names a full-screen or Split View Space
/// ("Safari", "Mail & Notes"); the kind's own word when no window is known.
fn app_label(view: &DesktopView, id: &SpaceId, kind: SpaceKind, apps: &AppIndex) -> String {
    let mut names: Vec<String> = Vec::new();
    for window in &view.windows {
        if window.space != SpacePlaceView::On(id.clone()) {
            continue;
        }
        let name = apps
            .get(&AppId(window.app.clone()))
            .map_or_else(|| window.app.clone(), |app| app.name.clone());
        if !names.contains(&name) {
            names.push(name);
        }
    }
    if names.is_empty() {
        return match kind {
            SpaceKind::SplitView => "Split View",
            SpaceKind::Agent => "Agent",
            SpaceKind::Desktop | SpaceKind::FullScreen => "Full Screen",
        }
        .to_owned();
    }
    names.join(" & ")
}

/// What the person did in the bar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BarPick {
    /// Clicked a Space.
    Switch(SpaceId),
    /// Clicked the plus on the output with this name.
    Add(String),
}

/// The commands a pick sends, in order. A Space switched to ends Mission Control, as on the Mac;
/// a new Space stays in it.
pub fn commands_of(pick: &BarPick) -> Vec<DesktopCommand> {
    match pick {
        BarPick::Switch(id) => vec![
            DesktopCommand::SwitchSpace(id.clone()),
            DesktopCommand::Overview {
                mode: OverviewMode::MissionControl,
                toggle: Toggle::Off,
            },
        ],
        BarPick::Add(output) => vec![DesktopCommand::CreateSpace(output.clone())],
    }
}
