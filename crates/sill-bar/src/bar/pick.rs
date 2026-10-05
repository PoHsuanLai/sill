//! What picking a bar menu item does, as a value: the component turns it into service commands
//! after the menu has closed (design A6 "closes then picks").

use sill_launcher::{AppId, DesktopAction};
use sill_model::app_menu::{AppMenuCommand, MenuApp, OwnItem, own_commands};
use sill_model::cosmic_wl::{Toplevels, WlCommand};
use sill_model::launch::{LaunchCommand, LaunchRequest};
use sill_model::tray::{TrayCommand, TrayItemId};

/// One bar menu action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum BarPick {
    /// A row the shell makes of the frontmost app's windows (`own_commands`): raise one, Hide,
    /// Quit.
    Own { app_id: String, item: OwnItem },
    /// Open COSMIC Settings.
    Settings,
    /// Open the power menu on this bar's output (the app menu's "Shut Down…").
    PowerMenu,
    /// A row of a tray item's dbusmenu, by its dbusmenu id.
    Tray { item: TrayItemId, entry: i32 },
    /// A row of the focused app's own menu, by its dbusmenu id: the person's click.
    AppMenu { app: MenuApp, id: i32 },
}

/// The app COSMIC's settings run as.
const SETTINGS_APP: &str = "com.system76.CosmicSettings";

/// Where a pick goes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Command {
    Wl(WlCommand),
    Launch(LaunchCommand),
    /// Open the power menu (`PowerMenuLink::open` with its default button).
    PowerMenu,
    /// Click a dbusmenu row of a tray item.
    Tray(TrayCommand),
    /// Click a row of the focused app's menu.
    AppMenu(AppMenuCommand),
}

/// The commands `pick` sends, in order.
pub(crate) fn commands(pick: &BarPick, toplevels: &Toplevels) -> Vec<Command> {
    match pick {
        BarPick::Own { app_id, item } => own_commands(*item, app_id, toplevels)
            .into_iter()
            .map(Command::Wl)
            .collect(),
        BarPick::Settings => vec![Command::Launch(LaunchCommand::Launch(LaunchRequest {
            app: AppId(SETTINGS_APP.to_owned()),
            action: DesktopAction::Default,
            files: Vec::new(),
            uris: Vec::new(),
            token: None,
        }))],
        BarPick::PowerMenu => vec![Command::PowerMenu],
        BarPick::Tray { item, entry } => vec![Command::Tray(TrayCommand::MenuPick {
            item: item.clone(),
            entry: *entry,
        })],
        BarPick::AppMenu { app, id } => vec![Command::AppMenu(AppMenuCommand::Pick {
            app: app.clone(),
            id: *id,
        })],
    }
}

#[cfg(test)]
mod tests {
    use super::{BarPick, Command, commands};
    use sill_launcher::WindowKey;
    use sill_model::app_menu::{AppMenuCommand, MenuApp, OwnItem};
    use sill_model::cosmic_wl::{Activity, Sizing, Toplevel, Toplevels, WindowState, WlCommand};
    use sill_model::ids::OutputName;
    use sill_model::launch::LaunchCommand;
    use sill_model::tray::{TrayCommand, TrayItemId};

    fn window(key: u64, app: &str) -> Toplevel {
        Toplevel {
            key: WindowKey(key),
            app_id: app.to_owned(),
            title: String::new(),
            activity: Activity::Inactive,
            window: WindowState::Normal,
            sizing: Sizing::Floating,
            workspaces: Vec::new(),
            outputs: vec![OutputName("DP-1".to_owned())],
        }
    }

    #[test]
    fn each_pick_becomes_its_commands() {
        let toplevels = Toplevels {
            windows: vec![window(1, "foot"), window(2, "foot"), window(3, "files")],
        };
        let own = |item| BarPick::Own {
            app_id: "foot".to_owned(),
            item,
        };
        let cases: Vec<(BarPick, Vec<Command>)> = vec![
            (
                own(OwnItem::Raise(WindowKey(3))),
                vec![Command::Wl(WlCommand::Activate(WindowKey(3)))],
            ),
            (
                own(OwnItem::Quit),
                vec![
                    Command::Wl(WlCommand::Close(WindowKey(1))),
                    Command::Wl(WlCommand::Close(WindowKey(2))),
                ],
            ),
            (
                own(OwnItem::HideOthers),
                vec![Command::Wl(WlCommand::Minimize(WindowKey(3)))],
            ),
            (BarPick::PowerMenu, vec![Command::PowerMenu]),
            (
                BarPick::Tray {
                    item: TrayItemId(":1.7/StatusNotifierItem".to_owned()),
                    entry: 12,
                },
                vec![Command::Tray(TrayCommand::MenuPick {
                    item: TrayItemId(":1.7/StatusNotifierItem".to_owned()),
                    entry: 12,
                })],
            ),
            (
                BarPick::AppMenu {
                    app: MenuApp("test.app".to_owned()),
                    id: 21,
                },
                vec![Command::AppMenu(AppMenuCommand::Pick {
                    app: MenuApp("test.app".to_owned()),
                    id: 21,
                })],
            ),
        ];
        for (pick, want) in cases {
            assert_eq!(commands(&pick, &toplevels), want, "{pick:?}");
        }
        let settings = commands(&BarPick::Settings, &toplevels);
        assert!(
            matches!(settings.as_slice(), [Command::Launch(LaunchCommand::Launch(r))] if r.app.0 == "com.system76.CosmicSettings"),
            "{settings:?}"
        );
    }
}
