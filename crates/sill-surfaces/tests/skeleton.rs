//! Behaviour the freeze leaves to its waves, as tests that fail until the stub is filled.

use sill_launcher::AppId;
use sill_services::{AppIndex, DockPins, Toplevels};
use sill_settings::TrashTile;
use sill_surfaces::surfaces::dock::{ItemKind, dock_items};
use sill_surfaces::surfaces::launcher::{LauncherKey, LauncherOut, LauncherState, step};

#[test]
#[ignore = "M3 dock-ui fill"]
fn pins_come_first_then_trash() {
    let pins = DockPins {
        version: 1,
        pinned: vec![AppId("foot".to_owned()), AppId("files".to_owned())],
    };
    let items = dock_items(
        &pins,
        &Toplevels::default(),
        &AppIndex::default(),
        TrashTile::On,
    );
    let kinds: Vec<ItemKind> = items.into_iter().map(|i| i.kind).collect();
    assert_eq!(
        kinds,
        vec![
            ItemKind::Pinned(AppId("foot".to_owned())),
            ItemKind::Pinned(AppId("files".to_owned())),
            ItemKind::Trash
        ]
    );
}

#[test]
#[ignore = "M4 launcher-ui fill"]
fn down_is_clamped_and_enter_activates_the_selection() {
    let (state, _) = step(LauncherState::default(), LauncherKey::Down, 3, 0);
    let (state, _) = step(state, LauncherKey::Down, 3, 0);
    let (state, _) = step(state, LauncherKey::Down, 3, 0);
    assert_eq!(
        state.selected, 2,
        "clamped at the last row, no wrap (06 §20.2)"
    );
    let (_, out) = step(state, LauncherKey::Enter, 3, 0);
    assert_eq!(
        out,
        vec![LauncherOut::Activate {
            row: 2,
            choice: sill_launcher::Choice::Primary
        }]
    );
}
