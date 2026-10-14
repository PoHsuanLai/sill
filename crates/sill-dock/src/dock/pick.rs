//! What the dock asks the services to do, as values: a context menu pick and every machine
//! effect that leaves the dock become [`DockCommand`]s, which the session sends. A launch
//! carries no activation token yet: the session asks shell-host for one first (plan "Launch").

use sill_launcher::{AppId, DesktopAction, WindowKey};
use sill_model::launch::LaunchCommand;
use sill_model::pins::{PinAt, PinEvent};
use sill_model::windows::WlCommand;

/// One context menu action (design/10 §10.3.7).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DockPick {
    /// Raise one of the app's windows.
    Activate(WindowKey),
    /// Run one of the entry's desktop actions.
    Action {
        app: AppId,
        action: String,
    },
    /// Keep in Dock.
    Pin(AppId),
    /// Remove from Dock.
    Unpin(AppId),
    ShowInFiles(AppId),
    /// Minimize every window of the app.
    Hide(Vec<WindowKey>),
    /// Close every window of the app.
    Quit(Vec<WindowKey>),
    /// SIGKILL the app's scopes.
    ForceQuit(AppId),
}

/// Where a command goes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DockCommand {
    Wl(WlCommand),
    Pins(PinEvent),
    /// A launch, sent once an activation token is in hand (`uris` for `%u`/`%U`).
    Launch {
        app: AppId,
        action: DesktopAction,
        uris: Vec<String>,
    },
    Service(LaunchCommand),
}

/// The commands `pick` sends, in order.
pub fn pick_commands(pick: &DockPick) -> Vec<DockCommand> {
    match pick {
        DockPick::Activate(key) => vec![DockCommand::Wl(WlCommand::Activate(*key))],
        DockPick::Action { app, action } => vec![DockCommand::Launch {
            app: app.clone(),
            action: DesktopAction::Named(action.clone()),
            uris: Vec::new(),
        }],
        DockPick::Pin(app) => vec![DockCommand::Pins(PinEvent::Pin {
            app: app.clone(),
            at: PinAt::End,
        })],
        DockPick::Unpin(app) => vec![DockCommand::Pins(PinEvent::Unpin(app.clone()))],
        DockPick::ShowInFiles(app) => {
            vec![DockCommand::Service(LaunchCommand::ShowInFiles(
                app.clone(),
            ))]
        }
        DockPick::Hide(keys) => keys
            .iter()
            .map(|key| DockCommand::Wl(WlCommand::Minimize(*key)))
            .collect(),
        DockPick::Quit(keys) => keys
            .iter()
            .map(|key| DockCommand::Wl(WlCommand::Close(*key)))
            .collect(),
        DockPick::ForceQuit(app) => {
            vec![DockCommand::Service(LaunchCommand::ForceQuit(app.clone()))]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{DockCommand, DockPick, pick_commands};
    use sill_launcher::{AppId, DesktopAction, WindowKey};
    use sill_model::launch::LaunchCommand;
    use sill_model::pins::{PinAt, PinEvent};
    use sill_model::windows::WlCommand;

    #[test]
    fn each_pick_becomes_its_commands() {
        let term = || AppId("term".to_owned());
        let k = WindowKey;
        let cases: Vec<(DockPick, Vec<DockCommand>)> = vec![
            (
                DockPick::Activate(k(3)),
                vec![DockCommand::Wl(WlCommand::Activate(k(3)))],
            ),
            (
                DockPick::Action {
                    app: term(),
                    action: "new-window".to_owned(),
                },
                vec![DockCommand::Launch {
                    app: term(),
                    action: DesktopAction::Named("new-window".to_owned()),
                    uris: vec![],
                }],
            ),
            (
                DockPick::Pin(term()),
                vec![DockCommand::Pins(PinEvent::Pin {
                    app: term(),
                    at: PinAt::End,
                })],
            ),
            (
                DockPick::Unpin(term()),
                vec![DockCommand::Pins(PinEvent::Unpin(term()))],
            ),
            (
                DockPick::Quit(vec![k(1), k(2)]),
                vec![
                    DockCommand::Wl(WlCommand::Close(k(1))),
                    DockCommand::Wl(WlCommand::Close(k(2))),
                ],
            ),
            (
                DockPick::Hide(vec![k(1)]),
                vec![DockCommand::Wl(WlCommand::Minimize(k(1)))],
            ),
            (
                DockPick::ForceQuit(term()),
                vec![DockCommand::Service(LaunchCommand::ForceQuit(term()))],
            ),
        ];
        for (pick, want) in cases {
            assert_eq!(pick_commands(&pick), want, "{pick:?}");
        }
    }
}
