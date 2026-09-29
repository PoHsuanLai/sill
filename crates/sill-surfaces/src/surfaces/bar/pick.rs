//! What picking a bar menu item does, as a value: the component turns it into service commands
//! after the menu has closed (design A6 "closes then picks").

use sill_launcher::{AppId, DesktopAction, WindowKey};
use sill_model::cosmic_wl::WlCommand;
use sill_services::{LaunchCommand, LaunchRequest};

/// One bar menu action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum BarPick {
    /// Raise one window of the frontmost app.
    Activate(WindowKey),
    /// Quit the frontmost app: ask each of its windows to close.
    Quit(Vec<WindowKey>),
    /// Open COSMIC Settings.
    Settings,
    /// Open the power menu on this bar's output (the app menu's "Shut Down…").
    PowerMenu,
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
}

/// The commands `pick` sends, in order.
pub(crate) fn commands(pick: &BarPick) -> Vec<Command> {
    match pick {
        BarPick::Activate(key) => vec![Command::Wl(WlCommand::Activate(*key))],
        BarPick::Quit(keys) => keys
            .iter()
            .map(|key| Command::Wl(WlCommand::Close(*key)))
            .collect(),
        BarPick::Settings => vec![Command::Launch(LaunchCommand::Launch(LaunchRequest {
            app: AppId(SETTINGS_APP.to_owned()),
            action: DesktopAction::Default,
            files: Vec::new(),
            uris: Vec::new(),
            token: None,
        }))],
        BarPick::PowerMenu => vec![Command::PowerMenu],
    }
}

#[cfg(test)]
mod tests {
    use super::{BarPick, Command, commands};
    use sill_launcher::WindowKey;
    use sill_model::cosmic_wl::WlCommand;
    use sill_services::LaunchCommand;

    #[test]
    fn each_pick_becomes_its_commands() {
        let cases: Vec<(BarPick, Vec<Command>)> = vec![
            (
                BarPick::Activate(WindowKey(3)),
                vec![Command::Wl(WlCommand::Activate(WindowKey(3)))],
            ),
            (
                BarPick::Quit(vec![WindowKey(1), WindowKey(2)]),
                vec![
                    Command::Wl(WlCommand::Close(WindowKey(1))),
                    Command::Wl(WlCommand::Close(WindowKey(2))),
                ],
            ),
            (BarPick::PowerMenu, vec![Command::PowerMenu]),
        ];
        for (pick, want) in cases {
            assert_eq!(commands(&pick), want, "{pick:?}");
        }
        let settings = commands(&BarPick::Settings);
        assert!(
            matches!(settings.as_slice(), [Command::Launch(LaunchCommand::Launch(r))] if r.app.0 == "com.system76.CosmicSettings"),
            "{settings:?}"
        );
    }
}
