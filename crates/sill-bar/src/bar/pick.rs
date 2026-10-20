//! What picking a bar menu item does, as a value: the component turns it into service commands
//! after the menu has closed (design A6 "closes then picks"). The shell's own actions (Settings,
//! a power action) are not commands of a service here: the bar names the action and the daemon
//! carries it out.

use sill_launcher::SessionAction;
use sill_launcher::actions::ActionId;
use sill_model::actions::Ask;
use sill_model::app_menu::{AppMenuCommand, MenuApp, OwnItem, own_commands};
use sill_model::tray::{TrayCommand, TrayItemId};
use sill_model::windows::{Toplevels, WlCommand};

/// One bar menu action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum BarPick {
    /// A row the shell makes of the frontmost app's windows (`own_commands`): raise one, Hide,
    /// Quit.
    Own { app_id: String, item: OwnItem },
    /// Open Settings.
    Settings,
    /// A power action from the system menu: the daemon carries it out at once, or asks about it
    /// first in the power menu on this bar's output.
    Session(SessionAction),
    /// A row of a tray item's dbusmenu, by its dbusmenu id.
    Tray { item: TrayItemId, entry: i32 },
    /// A row of the focused app's own menu, by its dbusmenu id: the person's click.
    AppMenu { app: MenuApp, id: i32 },
}

/// Where a pick goes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Command {
    Wl(WlCommand),
    /// An action of the shell, asked of the daemon.
    Do(Ask),
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
        BarPick::Settings => vec![Command::Do(Ask::of(ActionId::OpenSettings))],
        BarPick::Session(action) => vec![Command::Do(Ask::of(ActionId::of_session(*action)))],
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
    use sill_launcher::actions::ActionId;
    use sill_launcher::{SessionAction, WindowKey};
    use sill_model::actions::Ask;
    use sill_model::app_menu::{AppMenuCommand, MenuApp, OwnItem};
    use sill_model::ids::OutputName;
    use sill_model::tray::{TrayCommand, TrayItemId};
    use sill_model::windows::{Activity, Sizing, Toplevel, Toplevels, WindowState, WlCommand};

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
            (
                BarPick::Session(SessionAction::PowerOff),
                vec![Command::Do(Ask::of(ActionId::ShutDown))],
            ),
            (
                BarPick::Session(SessionAction::LogOut),
                vec![Command::Do(Ask::of(ActionId::LogOut))],
            ),
            (
                BarPick::Session(SessionAction::Lock),
                vec![Command::Do(Ask::of(ActionId::Lock))],
            ),
            (
                BarPick::Session(SessionAction::Suspend),
                vec![Command::Do(Ask::of(ActionId::Sleep))],
            ),
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
        assert_eq!(
            commands(&BarPick::Settings, &toplevels),
            vec![Command::Do(Ask::of(ActionId::OpenSettings))]
        );
    }
}
