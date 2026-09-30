//! The power menu's words, pure.

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

/// The alert's question for the default action, as macOS's shut-down dialog asks it.
pub fn prompt(action: SessionAction) -> &'static str {
    match action {
        SessionAction::Lock => "Lock the screen now?",
        SessionAction::LogOut => "Log out now?",
        SessionAction::Restart => "Restart this computer now?",
        SessionAction::PowerOff => "Shut down this computer now?",
        SessionAction::Suspend => "Suspend this computer now?",
    }
}

/// The line under the question: what the default action does to the running apps.
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
    use super::{action_label, consequence, prompt};
    use sill_model::session::SessionAction;

    #[test]
    fn every_action_has_its_label_question_and_consequence() {
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
        for &(action, _) in LABELS {
            assert!(prompt(action).ends_with('?'), "{action:?}");
            assert!(consequence(action).ends_with('.'), "{action:?}");
        }
    }
}
