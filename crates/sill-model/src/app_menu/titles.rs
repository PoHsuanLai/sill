//! The bar's titles made from an app's menus: the first menu is the app's own (About, Settings,
//! Quit, titled with its name in quire's order) and goes into the bold app-name menu; the others
//! are titles of their own after it.

use crate::tray::{TrayMenuEntry, TrayMenuKind};
use crate::units::Visibility;

use super::ServedMenu;

/// One title after the app name, and the rows of its menu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TitledMenu {
    /// The menu's dbusmenu id: what `AboutToShow` names when it opens.
    pub id: i32,
    pub title: String,
    pub rows: Vec<TrayMenuEntry>,
}

/// What an app's menu makes of the bar.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Titles {
    /// The app's own menu: its dbusmenu id and rows, shown in the app-name menu.
    pub app: Option<(i32, Vec<TrayMenuEntry>)>,
    /// The other menus, in the order the app sent them.
    pub menus: Vec<TitledMenu>,
}

/// `served`'s titles: its first menu is the app's own whatever it holds; a later one the app
/// marked hidden, or with nothing to show, makes no title.
pub fn titles(served: &ServedMenu) -> Titles {
    let mut entries = served.menus.iter();
    let app = entries.next().and_then(|first| match &first.kind {
        TrayMenuKind::Item { children, .. } => Some((first.id, children.clone())),
        TrayMenuKind::Separator => None,
    });
    Titles {
        app,
        menus: entries.filter_map(menu).collect(),
    }
}

fn menu(entry: &TrayMenuEntry) -> Option<TitledMenu> {
    let TrayMenuKind::Item {
        label,
        visibility: Visibility::Shown,
        children,
        ..
    } = &entry.kind
    else {
        return None;
    };
    children.iter().any(shown).then(|| TitledMenu {
        id: entry.id,
        title: label.clone(),
        rows: children.clone(),
    })
}

/// Whether `row` is a row that draws.
pub(crate) fn shown(row: &TrayMenuEntry) -> bool {
    match &row.kind {
        TrayMenuKind::Separator => false,
        TrayMenuKind::Item { visibility, .. } => *visibility == Visibility::Shown,
    }
}
