//! The names of the shell's actions: every thing a person can ask of the desktop by name, from
//! a menu, a corner, a launcher row, a chord or the command line. A closed enum, so a new
//! action is a compile error at every table that must say what it is; what each one is called,
//! asks and does is `sill_model::actions`'s, and the one place it is carried out is the daemon's.
//!
//! `ActionId::ALL` (from `Word`) lists them all.
//!
//! This is not [`crate::ActionId`] (that names an action of one launcher row, such as Reveal).

use ds_core::word::Word;
use serde::{Deserialize, Serialize};

use crate::session::SessionAction;

/// One action of the shell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Word)]
#[serde(rename_all = "kebab-case")]
pub enum ActionId {
    /// Lock the screen.
    Lock,
    LogOut,
    Restart,
    ShutDown,
    Sleep,
    /// The launcher panel.
    Launcher,
    /// The launcher panel, on emoji and symbols alone.
    EmojiAndSymbols,
    /// Switch to the next input source (input method).
    NextInputSource,
    PowerMenu,
    /// The Force Quit Applications panel.
    ForceQuitPanel,
    NotificationCenter,
    ControlCenter,
    ScreenshotArea,
    ScreenshotWindow,
    ScreenshotScreen,
    ShowDesktop,
    MissionControl,
    NextSpace,
    PreviousSpace,
    /// Bring forward the front app's next window.
    NextWindow,
    /// The app switcher: arm it, or move its selection on.
    SwitcherNext,
    /// The app switcher: move its selection back.
    SwitcherPrev,
    /// The app switcher: switch to the selection and close it.
    SwitcherCommit,
    /// The app switcher: close it and switch to nothing.
    SwitcherCancel,
    /// Open Settings, at a place when one is given.
    OpenSettings,
    /// Start an app.
    OpenApp,
    ShowInFiles,
    /// Hide an app's windows.
    HideApp,
    /// Close an app's windows.
    QuitApp,
    /// End an app the shell started, at once.
    ForceQuitApp,
    /// Keep an app in the dock.
    PinApp,
    UnpinApp,
    ToggleDockHide,
    /// Turn Do Not Disturb on or off.
    ToggleFocus,
    /// Re-read settings, apps and pins.
    Reload,
}

impl ActionId {
    /// The action that carries out the power action `action`.
    pub fn of_session(action: SessionAction) -> ActionId {
        match action {
            SessionAction::Lock => ActionId::Lock,
            SessionAction::LogOut => ActionId::LogOut,
            SessionAction::Restart => ActionId::Restart,
            SessionAction::PowerOff => ActionId::ShutDown,
            SessionAction::Suspend => ActionId::Sleep,
        }
    }
}

#[cfg(test)]
mod tests {
    use ds_core::word::Word;

    use super::ActionId;
    use crate::session::SessionAction;

    #[test]
    fn every_session_action_has_its_action_and_the_list_has_no_repeat() {
        let named: Vec<ActionId> = SessionAction::ALL
            .into_iter()
            .map(ActionId::of_session)
            .collect();
        assert_eq!(named.len(), SessionAction::ALL.len());
        for (at, id) in named.iter().enumerate() {
            assert!(!named[..at].contains(id), "{id:?} is named twice");
            assert!(ActionId::ALL.contains(id), "{id:?}");
        }
    }
}
