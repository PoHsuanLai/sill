//! What activating a result asks the daemon to do. A closed vocabulary: the daemon matches it
//! exhaustively, so a new kind of activation is a compile error at every site that must act.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::companion::CompanionAct;
use crate::ids::{AppId, ClipKey};
use crate::query::Generation;

/// The effect of choosing a result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum Activation {
    /// Start (or D-Bus activate) an app, optionally one of its desktop actions.
    Launch { app: AppId, action: DesktopAction },
    /// Activate an open window.
    FocusWindow { window: WindowKey },
    /// Ask an open window to close (the windows provider's Close action).
    CloseWindow { window: WindowKey },
    /// Put text on the clipboard (the calculator's result).
    Copy { text: String },
    /// Replace the launcher's query with `text` and keep it open (the calculator's secondary
    /// action: carry on calculating from the result).
    Insert { text: String },
    /// Open a settings page.
    OpenSettings { target: SettingsTarget },
    /// A session command.
    System(SystemCommand),
    /// Open a file, its folder, or an address with the default handler (`xdg-open`, with the
    /// surface's activation token).
    Open { target: OpenTarget },
    /// Make a clipboard history entry the clipboard again (the clipboard service offers it as the
    /// selection; the person pastes).
    SetClipboard { entry: ClipKey },
    /// Drop a clipboard history entry (its row's "Forget" action; the launcher stays open).
    ForgetClipboard { entry: ClipKey },
    /// Perform an action of an app through intentd (a thing's primary action, from the things
    /// provider).
    Intent(Box<docket_core::CallRequest>),
    /// Something for the companion: ask, press a card's button, steer a run, undo, settle a
    /// memory, switch the panel's mode.
    Companion(CompanionAct),
    /// Nothing to do (a header row, a stale result).
    Nothing,
}

/// What [`Activation::Open`] opens.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum OpenTarget {
    /// The file or folder itself, in its default app.
    File(PathBuf),
    /// A file in the viewer, with the search results it was chosen from, so ← and → walk them.
    Viewer(ViewerOpen),
    /// A file the viewer plays with no window of its own (audio).
    Play(PathBuf),
    /// "Reveal in Files": the folder holding this path, the path selected where the file
    /// manager can (`org.freedesktop.FileManager1.ShowItems`), else the folder opened.
    Reveal(PathBuf),
    /// An address (the web fallback's search).
    Url(String),
}

/// A file chosen in the launcher to be opened in the viewer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ViewerOpen {
    /// The file.
    pub file: PathBuf,
    /// The search the results came from.
    pub search: Generation,
    /// The files of the results it was chosen from, in the order the launcher shows them, with
    /// `file` among them; empty when the row came from no list.
    pub results: Vec<PathBuf>,
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

/// A window, by the key the windows thread gave its toplevel handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct WindowKey(pub u64);

/// A settings destination.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum SettingsTarget {
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
    use super::{
        Activation, DesktopAction, OpenTarget, SettingsTarget, SystemCommand, ViewerOpen, WindowKey,
    };
    use crate::ids::{AppId, ClipKey};
    use crate::query::Generation;
    use std::path::PathBuf;

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
            (
                Activation::Open {
                    target: OpenTarget::File(PathBuf::from("/home/p/a b.pdf")),
                },
                r#"{"kind":"open","v":{"target":{"kind":"file","v":"/home/p/a b.pdf"}}}"#,
            ),
            (
                Activation::Open {
                    target: OpenTarget::Viewer(ViewerOpen {
                        file: PathBuf::from("/home/p/b.pdf"),
                        search: Generation(41),
                        results: vec![
                            PathBuf::from("/home/p/a.pdf"),
                            PathBuf::from("/home/p/b.pdf"),
                        ],
                    }),
                },
                r#"{"kind":"open","v":{"target":{"kind":"viewer","v":{"file":"/home/p/b.pdf","search":41,"results":["/home/p/a.pdf","/home/p/b.pdf"]}}}}"#,
            ),
            (
                Activation::Open {
                    target: OpenTarget::Play(PathBuf::from("/home/p/a.flac")),
                },
                r#"{"kind":"open","v":{"target":{"kind":"play","v":"/home/p/a.flac"}}}"#,
            ),
            (
                Activation::Open {
                    target: OpenTarget::Reveal(PathBuf::from("/home/p/a.pdf")),
                },
                r#"{"kind":"open","v":{"target":{"kind":"reveal","v":"/home/p/a.pdf"}}}"#,
            ),
            (
                Activation::Open {
                    target: OpenTarget::Url("https://duckduckgo.com/?q=a%20b".to_owned()),
                },
                r#"{"kind":"open","v":{"target":{"kind":"url","v":"https://duckduckgo.com/?q=a%20b"}}}"#,
            ),
            (
                Activation::SetClipboard { entry: ClipKey(9) },
                r#"{"kind":"set_clipboard","v":{"entry":9}}"#,
            ),
            (
                Activation::ForgetClipboard { entry: ClipKey(9) },
                r#"{"kind":"forget_clipboard","v":{"entry":9}}"#,
            ),
            (
                Activation::Intent(Box::new(docket_core::CallRequest {
                    action: docket_core::ActionRef {
                        app: prov::AppName::parse("org.quire.Mail")
                            .unwrap_or_else(|e| panic!("{e}")),
                        name: prov::ActionName::parse("mail.message.open")
                            .unwrap_or_else(|e| panic!("{e}")),
                    },
                    target: docket_core::TargetValue::Nothing,
                    args: docket_core::Args::new(),
                    origin: docket_core::Origin::Launcher,
                })),
                r#"{"kind":"intent","v":{"action":{"app":"org.quire.Mail","name":"mail.message.open"},"target":{"kind":"nothing"},"args":{},"origin":"launcher"}}"#,
            ),
            (
                Activation::Companion(crate::CompanionAct::Mode(crate::LauncherMode::Search)),
                r#"{"kind":"companion","v":{"kind":"mode","v":"search"}}"#,
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
