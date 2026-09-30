//! The power menu's words and button looks, pure.

use ds::{Answers, ButtonRole};
use sill_model::session::SessionAction;

/// A button's label (the launcher's row titles: "Log Out", "Restart", "Shut Down", "Suspend";
/// "Lock" for completeness).
pub fn action_label(action: SessionAction) -> &'static str {
    match action {
        SessionAction::Lock => "Lock",
        SessionAction::LogOut => "Log Out",
        SessionAction::Restart => "Restart",
        SessionAction::PowerOff => "Shut Down",
        SessionAction::Suspend => "Suspend",
    }
}

/// A button's quire look (design/20 §1.8): the focused button answers Return and takes the accent;
/// Restart and Shut Down are destructive (they end every running app); the rest are plain.
pub fn button_look(action: SessionAction, focused: SessionAction) -> (Answers, ButtonRole) {
    match action {
        _ if action == focused => (Answers::Return, ButtonRole::Normal),
        SessionAction::Restart | SessionAction::PowerOff => {
            (Answers::Nothing, ButtonRole::Destructive)
        }
        SessionAction::Lock | SessionAction::LogOut | SessionAction::Suspend => {
            (Answers::Nothing, ButtonRole::Normal)
        }
    }
}

/// The sheet's question for the focused button, as macOS's shut-down dialog asks it.
pub fn prompt(action: SessionAction) -> &'static str {
    match action {
        SessionAction::Lock => "Lock the screen now?",
        SessionAction::LogOut => "Log out now?",
        SessionAction::Restart => "Restart this computer now?",
        SessionAction::PowerOff => "Shut down this computer now?",
        SessionAction::Suspend => "Suspend this computer now?",
    }
}

/// The line under the question: what the focused action does to the running apps.
pub fn consequence(action: SessionAction) -> &'static str {
    match action {
        SessionAction::Lock => "Your apps keep running behind the lock screen.",
        SessionAction::LogOut => "Your apps will quit and the session will end.",
        SessionAction::Restart => "Your apps will quit and the computer will start again.",
        SessionAction::PowerOff => "Your apps will quit and the computer will turn off.",
        SessionAction::Suspend => "The computer will sleep and wake where you left it.",
    }
}

#[cfg(test)]
mod tests {
    use super::{action_label, button_look, consequence, prompt};
    use ds::{Answers, ButtonRole};
    use sill_model::session::SessionAction;

    #[test]
    fn every_button_has_its_label_and_look() {
        use SessionAction::{Lock, LogOut, PowerOff, Restart, Suspend};
        const LABELS: &[(SessionAction, &str)] = &[
            (Lock, "Lock"),
            (LogOut, "Log Out"),
            (Restart, "Restart"),
            (PowerOff, "Shut Down"),
            (Suspend, "Suspend"),
        ];
        for &(action, want) in LABELS {
            assert_eq!(action_label(action), want, "{action:?}");
        }
        let cases = [
            (PowerOff, PowerOff, (Answers::Return, ButtonRole::Normal)),
            (
                Restart,
                PowerOff,
                (Answers::Nothing, ButtonRole::Destructive),
            ),
            (
                PowerOff,
                LogOut,
                (Answers::Nothing, ButtonRole::Destructive),
            ),
            (LogOut, PowerOff, (Answers::Nothing, ButtonRole::Normal)),
            (Suspend, PowerOff, (Answers::Nothing, ButtonRole::Normal)),
        ];
        for &(action, _) in LABELS {
            assert!(prompt(action).ends_with('?'), "{action:?}");
            assert!(consequence(action).ends_with('.'), "{action:?}");
        }
        for (action, focused, want) in cases {
            assert_eq!(button_look(action, focused), want, "{action:?} {focused:?}");
        }
    }
}
