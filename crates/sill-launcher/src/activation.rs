//! What activating a result asks the daemon to do. A closed vocabulary: the daemon matches it
//! exhaustively, so a new kind of activation is a compile error at every site that must act.

use serde::{Deserialize, Serialize};

use crate::ids::AppId;

/// The effect of choosing a result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum Activation {
    /// Start (or D-Bus activate) an app, optionally one of its desktop actions.
    Launch { app: AppId, action: DesktopAction },
    /// Activate an open window.
    FocusWindow { window: WindowKey },
    /// Ask an open window to close (the windows provider's Close action). M4 freeze
    /// amendment, FINDINGS F111.
    CloseWindow { window: WindowKey },
    /// Put text on the clipboard (the calculator's result).
    Copy { text: String },
    /// Replace the launcher's query with `text` and keep it open (the calculator's secondary
    /// action: carry on calculating from the result). M4 freeze amendment, FINDINGS F111.
    Insert { text: String },
    /// Open a settings page.
    OpenSettings { target: SettingsTarget },
    /// A session command.
    System(SystemCommand),
    /// Nothing to do (a header row, a stale result).
    Nothing,
}

/// Which entry point of an app to start.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum DesktopAction {
    /// The entry's own `Exec`.
    Default,
    /// `[Desktop Action <id>]`.
    Named(String),
}

/// A window, by the key the `cosmic_wl` thread gave its toplevel handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct WindowKey(pub u64);

/// A settings destination.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum SettingsTarget {
    /// A `cosmic-settings` page id (`cosmic-settings <page>`), when that binary exists.
    Cosmic(String),
    /// A quire settings key path (`appearance.theme`), resolved through the schemas
    /// (design/22-SETTINGS.md §9.3 "deep links").
    Key(String),
}

/// The system provider's commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SystemCommand {
    /// `loginctl lock-session`.
    Lock,
    /// logind `TerminateSession`.
    LogOut,
    Suspend,
    Restart,
    ShutDown,
    /// Re-read settings, apps and pins (the same as `sill reload`).
    Reload,
}

#[cfg(test)]
mod tests {
    use super::{Activation, DesktopAction, SettingsTarget, SystemCommand, WindowKey};
    use crate::ids::AppId;

    /// Every variant's wire form, pinned: the daemon and the launcher UI agree on these.
    #[test]
    fn every_activation_has_a_pinned_wire_form_and_round_trips() {
        let cases: Vec<(Activation, &str)> = vec![
            (
                Activation::Launch {
                    app: AppId("foot".to_owned()),
                    action: DesktopAction::Named("new-window".to_owned()),
                },
                r#"{"kind":"launch","v":{"app":"foot","action":{"kind":"named","v":"new-window"}}}"#,
            ),
            (
                Activation::FocusWindow {
                    window: WindowKey(7),
                },
                r#"{"kind":"focus_window","v":{"window":7}}"#,
            ),
            (
                Activation::CloseWindow {
                    window: WindowKey(7),
                },
                r#"{"kind":"close_window","v":{"window":7}}"#,
            ),
            (
                Activation::Copy {
                    text: "42".to_owned(),
                },
                r#"{"kind":"copy","v":{"text":"42"}}"#,
            ),
            (
                Activation::Insert {
                    text: "42".to_owned(),
                },
                r#"{"kind":"insert","v":{"text":"42"}}"#,
            ),
            (
                Activation::OpenSettings {
                    target: SettingsTarget::Key("appearance.theme".to_owned()),
                },
                r#"{"kind":"open_settings","v":{"target":{"kind":"key","v":"appearance.theme"}}}"#,
            ),
            (
                Activation::System(SystemCommand::ShutDown),
                r#"{"kind":"system","v":"shut_down"}"#,
            ),
            (Activation::Nothing, r#"{"kind":"nothing"}"#),
        ];
        for (activation, wire) in cases {
            let written = serde_json::to_string(&activation).unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(written, wire, "{activation:?}");
            let read: Activation = serde_json::from_str(wire).unwrap_or_else(|e| panic!("{e}"));
            assert_eq!(read, activation, "{wire}");
        }
    }
}
