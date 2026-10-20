//! What the dock asks the services to do, as values: a context menu pick and every machine
//! effect that leaves the dock become [`DockCommand`]s, which the session sends. A launch
//! carries no activation token yet: the session asks shell-host for one first (plan "Launch").

use sill_launcher::actions::ActionId;
use sill_launcher::{AppId, DesktopAction, WindowKey};
use sill_model::actions::{Ask, Subject};
use sill_model::pins::PinEvent;
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
    /// An action of the shell, asked of the daemon.
    Do(Ask),
}

/// The commands `pick` sends, in order.
pub fn pick_commands(pick: &DockPick) -> Vec<DockCommand> {
    let on_app = |action, app: &AppId| DockCommand::Do(Ask::on(action, Subject::App(app.clone())));
    let on_windows = |action, keys: &Vec<WindowKey>| {
        DockCommand::Do(Ask::on(action, Subject::Windows(keys.clone())))
    };
    match pick {
        DockPick::Activate(key) => vec![DockCommand::Wl(WlCommand::Activate(*key))],
        DockPick::Action { app, action } => vec![DockCommand::Launch {
            app: app.clone(),
            action: DesktopAction::Named(action.clone()),
            uris: Vec::new(),
        }],
        DockPick::Pin(app) => vec![on_app(ActionId::PinApp, app)],
        DockPick::Unpin(app) => vec![on_app(ActionId::UnpinApp, app)],
        DockPick::ShowInFiles(app) => vec![on_app(ActionId::ShowInFiles, app)],
        DockPick::Hide(keys) => vec![on_windows(ActionId::HideApp, keys)],
        DockPick::Quit(keys) => vec![on_windows(ActionId::QuitApp, keys)],
        DockPick::ForceQuit(app) => vec![on_app(ActionId::ForceQuitApp, app)],
    }
}

#[cfg(test)]
mod tests {
    use super::{DockCommand, DockPick, pick_commands};
    use sill_launcher::actions::ActionId;
    use sill_launcher::{AppId, DesktopAction, WindowKey};
    use sill_model::actions::{Ask, Subject};
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
                vec![DockCommand::Do(Ask::on(
                    ActionId::PinApp,
                    Subject::App(term()),
                ))],
            ),
            (
                DockPick::Unpin(term()),
                vec![DockCommand::Do(Ask::on(
                    ActionId::UnpinApp,
                    Subject::App(term()),
                ))],
            ),
            (
                DockPick::Quit(vec![k(1), k(2)]),
                vec![DockCommand::Do(Ask::on(
                    ActionId::QuitApp,
                    Subject::Windows(vec![k(1), k(2)]),
                ))],
            ),
            (
                DockPick::Hide(vec![k(1)]),
                vec![DockCommand::Do(Ask::on(
                    ActionId::HideApp,
                    Subject::Windows(vec![k(1)]),
                ))],
            ),
            (
                DockPick::ForceQuit(term()),
                vec![DockCommand::Do(Ask::on(
                    ActionId::ForceQuitApp,
                    Subject::App(term()),
                ))],
            ),
        ];
        for (pick, want) in cases {
            assert_eq!(pick_commands(&pick), want, "{pick:?}");
        }
    }
}
