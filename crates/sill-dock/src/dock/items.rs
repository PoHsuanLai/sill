//! Which tiles the dock shows, in order (design/10 §10.3.9): `[pinned (layout.toml order)]
//! [running, unpinned, in launch order] | [Trash]`, running windows grouped by `match_app`.
//!
//! "Launch order" is read from the window keys: the `windows` thread numbers toplevels as it
//! first sees them, so an app's smallest key is when it first had a window. A tile's
//! [`TileId`] is a hash of what it stands for, not its position, so a bounce or a pressed tile
//! stays on its app when another app's tile appears before it.

use sill_launcher::AppId;
use sill_model::apps::AppIndex;
use sill_model::pins::DockPins;
use sill_model::windows::{Activity, AppMatch, Toplevel, Toplevels, WindowState, app_of};
use sill_settings::TrashTile;

use crate::dock::machine::{TileApp, TileId};

/// One tile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DockItem {
    pub tile: TileId,
    pub kind: ItemKind,
    pub app: TileApp,
}

/// What a tile stands for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ItemKind {
    Pinned(AppId),
    /// A running app with no pin; `AppId` when `match_app` found an entry, else the raw app id.
    Running(AppId),
    /// A running window whose app id matches no entry.
    Unmatched(String),
    Trash,
}

impl ItemKind {
    /// The desktop entry the tile launches and names, when it has one.
    pub fn app_id(&self) -> Option<&AppId> {
        match self {
            ItemKind::Pinned(id) | ItemKind::Running(id) => Some(id),
            ItemKind::Unmatched(_) | ItemKind::Trash => None,
        }
    }

    /// A stable word for the tile, which its [`TileId`] hashes.
    fn key(&self) -> String {
        match self {
            ItemKind::Pinned(id) | ItemKind::Running(id) => format!("app:{}", id.0),
            ItemKind::Unmatched(raw) => format!("raw:{raw}"),
            ItemKind::Trash => "trash".to_owned(),
        }
    }
}

/// The tiles for `pins`, `toplevels` and `index`, with the Trash tile when `trash` is on.
pub fn dock_items(
    pins: &DockPins,
    toplevels: &Toplevels,
    index: &AppIndex,
    trash: TrashTile,
) -> Vec<DockItem> {
    let groups = running_groups(toplevels, index);
    let pinned = pins.pinned.iter().map(|id| {
        let windows = groups
            .iter()
            .find(|(group, _)| group.app_id() == Some(id))
            .map(|(_, windows)| windows.as_slice())
            .unwrap_or_default();
        item(ItemKind::Pinned(id.clone()), windows)
    });
    let running = groups
        .iter()
        .filter(|(group, _)| group.app_id().is_none_or(|id| !pins.pinned.contains(id)))
        .map(|(group, windows)| item(group.clone(), windows));
    let trash = match trash {
        TrashTile::On => Some(item(ItemKind::Trash, &[])),
        TrashTile::Off => None,
    };
    pinned.chain(running).chain(trash).collect()
}

/// Every running app once, as `Running` or `Unmatched`, with its windows in MRU order; the
/// groups in launch order (smallest window key first).
fn running_groups<'a>(
    toplevels: &'a Toplevels,
    index: &AppIndex,
) -> Vec<(ItemKind, Vec<&'a Toplevel>)> {
    let mut groups: Vec<(ItemKind, Vec<&Toplevel>)> = Vec::new();
    for window in &toplevels.windows {
        let kind = group_of(window, index);
        match groups.iter_mut().find(|(k, _)| *k == kind) {
            Some((_, windows)) => windows.push(window),
            None => groups.push((kind, vec![window])),
        }
    }
    groups.sort_by_key(|(_, windows)| windows.iter().map(|w| w.key).min());
    groups
}

/// The group `window` belongs to.
fn group_of(window: &Toplevel, index: &AppIndex) -> ItemKind {
    match app_of(window, index) {
        AppMatch::Entry(id) => ItemKind::Running(id),
        AppMatch::Unmatched(raw) => ItemKind::Unmatched(raw),
    }
}

/// The windows of `item`, in MRU order (every window of its app).
pub fn item_windows<'a>(
    item: &ItemKind,
    toplevels: &'a Toplevels,
    index: &AppIndex,
) -> Vec<&'a Toplevel> {
    let wanted = match item {
        ItemKind::Pinned(id) => ItemKind::Running(id.clone()),
        other => other.clone(),
    };
    toplevels
        .windows
        .iter()
        .filter(|w| group_of(w, index) == wanted)
        .collect()
}

fn item(kind: ItemKind, windows: &[&Toplevel]) -> DockItem {
    DockItem {
        tile: tile_id(&kind),
        app: tile_app(windows),
        kind,
    }
}

/// What a tile's windows say about its app (design/10 §10.3.6's click conditions).
fn tile_app(windows: &[&Toplevel]) -> TileApp {
    if windows.is_empty() {
        return TileApp::NotRunning;
    }
    if windows.iter().any(|w| w.activity == Activity::Active) {
        return TileApp::Active {
            windows: u32::try_from(windows.len()).unwrap_or(u32::MAX),
        };
    }
    if windows.iter().all(|w| w.window == WindowState::Minimized) {
        return TileApp::AllMinimized;
    }
    TileApp::Inactive
}

/// FNV-1a over the tile's key: the same app is the same tile for the dock's life.
fn tile_id(kind: &ItemKind) -> TileId {
    let hash = kind.key().bytes().fold(0x811c_9dc5_u32, |hash, byte| {
        (hash ^ u32::from(byte)).wrapping_mul(0x0100_0193)
    });
    TileId(hash)
}

#[cfg(test)]
mod tests {
    use super::{DockItem, ItemKind, dock_items};
    use crate::dock::machine::TileApp;
    use sill_launcher::{AppId, WindowKey};
    use sill_model::apps::{App, AppIcon, AppIndex, DbusActivation, Listing, Terminal};
    use sill_model::ids::OutputName;
    use sill_model::pins::DockPins;
    use sill_model::windows::{Activity, Sizing, Toplevel, Toplevels, WindowState};
    use sill_settings::TrashTile;
    use std::path::PathBuf;

    fn app(id: &str) -> App {
        App {
            id: AppId(id.to_owned()),
            name: id.to_owned(),
            generic_name: String::new(),
            comment: String::new(),
            exec: id.to_owned(),
            icon: AppIcon::None,
            startup_wm_class: None,
            flatpak: None,
            bus_names: Vec::new(),
            intents_app: None,
            renamed_from: Vec::new(),
            terminal: Terminal::No,
            activation: DbusActivation::Exec,
            actions: Vec::new(),
            mime_types: Vec::new(),
            categories: Vec::new(),
            keyboard_profile: None,
            keywords: Vec::new(),
            listing: Listing::Shown,
            path: PathBuf::from(format!("/usr/share/applications/{id}.desktop")),
        }
    }

    fn index() -> AppIndex {
        AppIndex::new(vec![app("org.gnome.Nautilus"), app("foot"), app("firefox")])
    }

    fn window(key: u64, app_id: &str, activity: Activity, state: WindowState) -> Toplevel {
        Toplevel {
            identifier: String::new(),
            key: WindowKey(key),
            app_id: app_id.to_owned(),
            title: format!("{app_id} {key}"),
            activity,
            window: state,
            sizing: Sizing::Floating,
            workspaces: Vec::new(),
            outputs: vec![OutputName("DP-1".to_owned())],
        }
    }

    fn pins(ids: &[&str]) -> DockPins {
        DockPins {
            pinned: ids.iter().map(|id| AppId((*id).to_owned())).collect(),
        }
    }

    /// `(kind word, app state)` per tile, for readable tables.
    fn shape(items: &[DockItem]) -> Vec<(String, TileApp)> {
        items
            .iter()
            .map(|item| {
                let word = match &item.kind {
                    ItemKind::Pinned(id) => format!("pin {}", id.0),
                    ItemKind::Running(id) => format!("run {}", id.0),
                    ItemKind::Unmatched(raw) => format!("raw {raw}"),
                    ItemKind::Trash => "trash".to_owned(),
                };
                (word, item.app)
            })
            .collect()
    }

    #[test]
    fn pins_then_running_grouped_by_match_app_then_trash() {
        use Activity::{Active, Inactive};
        use WindowState::{Minimized, Normal};
        // MRU order: the active terminal first; firefox opened first (key 1).
        let toplevels = Toplevels {
            windows: vec![
                window(5, "foot", Active, Normal),
                window(1, "firefox", Inactive, Normal),
                window(3, "org.example.Unknown", Inactive, Normal),
                window(4, "foot", Inactive, Normal),
                window(2, "Firefox", Inactive, Minimized),
            ],
        };
        let items = dock_items(
            &pins(&["org.gnome.Nautilus", "foot"]),
            &toplevels,
            &index(),
            TrashTile::On,
        );
        assert_eq!(
            shape(&items),
            vec![
                ("pin org.gnome.Nautilus".to_owned(), TileApp::NotRunning),
                ("pin foot".to_owned(), TileApp::Active { windows: 2 }),
                ("run firefox".to_owned(), TileApp::Inactive),
                ("raw org.example.Unknown".to_owned(), TileApp::Inactive),
                ("trash".to_owned(), TileApp::NotRunning),
            ]
        );
    }

    #[test]
    fn a_minimized_app_and_no_trash() {
        let toplevels = Toplevels {
            windows: vec![window(
                1,
                "firefox",
                Activity::Inactive,
                WindowState::Minimized,
            )],
        };
        let items = dock_items(&pins(&[]), &toplevels, &index(), TrashTile::Off);
        assert_eq!(
            shape(&items),
            vec![("run firefox".to_owned(), TileApp::AllMinimized)]
        );
        assert!(dock_items(&pins(&[]), &Toplevels::default(), &index(), TrashTile::Off).is_empty());
    }

    #[test]
    fn a_tile_keeps_its_id_when_another_tile_appears_before_it() {
        let term = window(9, "foot", Activity::Inactive, WindowState::Normal);
        let before = dock_items(
            &pins(&[]),
            &Toplevels {
                windows: vec![term.clone()],
            },
            &index(),
            TrashTile::On,
        );
        let after = dock_items(
            &pins(&[]),
            &Toplevels {
                windows: vec![
                    term,
                    window(1, "firefox", Activity::Inactive, WindowState::Normal),
                ],
            },
            &index(),
            TrashTile::On,
        );
        assert_eq!(before[0].kind, after[1].kind, "firefox came first");
        assert_eq!(before[0].tile, after[1].tile);
        assert_ne!(after[0].tile, after[1].tile);
        let pinned = dock_items(
            &pins(&["foot"]),
            &Toplevels::default(),
            &index(),
            TrashTile::Off,
        );
        assert_eq!(
            pinned[0].tile, before[0].tile,
            "pinned or running, one tile"
        );
    }
}
