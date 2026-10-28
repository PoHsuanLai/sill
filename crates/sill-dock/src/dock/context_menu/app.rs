//! An app tile's rows (design/10 §10.3.7).

use sill_model::apps::App;
use sill_model::windows::{Activity, Toplevels, WindowState};

use super::{Checked, DockMenuRow, OptionHeld, menu_windows, sections};
use crate::dock::items::{DockItem, ItemKind};

/// The rows for the app tile `item`. `login` is what the autostart folder says about the app.
pub fn rows(
    item: &DockItem,
    app: Option<&App>,
    toplevels: &Toplevels,
    option: OptionHeld,
    login: Checked,
) -> Vec<DockMenuRow> {
    let windows = menu_windows(item, app, toplevels);
    let running = !windows.is_empty();
    let window_rows = windows.iter().map(|w| DockMenuRow::Window {
        title: w.title.clone(),
        active: checked(w.activity == Activity::Active),
        minimized: w.window == WindowState::Minimized,
    });
    let actions =
        app.into_iter()
            .flat_map(|app| &app.actions)
            .map(|a| DockMenuRow::DesktopAction {
                id: a.id.clone(),
                name: a.name.clone(),
            });
    let pinned = checked(matches!(item.kind, ItemKind::Pinned(_)));
    let keep = item.kind.app_id().map(|_| DockMenuRow::KeepInDock(pinned));
    let at_login = app.map(|app| DockMenuRow::OpenAtLogin {
        state: login,
        entry: app.path.clone(),
    });
    let in_files = item.kind.app_id().map(|_| DockMenuRow::ShowInFiles);
    let options: Vec<DockMenuRow> = keep.into_iter().chain(at_login).chain(in_files).collect();
    let quit = match option {
        OptionHeld::Held => DockMenuRow::ForceQuit,
        OptionHeld::Up => DockMenuRow::Quit,
    };
    let ending: Vec<DockMenuRow> = if running {
        vec![DockMenuRow::ShowAllWindows, DockMenuRow::Hide, quit]
    } else {
        Vec::new()
    };
    sections([window_rows.collect(), actions.collect(), options, ending])
}

fn checked(on: bool) -> Checked {
    if on {
        Checked::Checked
    } else {
        Checked::Unchecked
    }
}
