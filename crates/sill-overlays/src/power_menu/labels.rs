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

/// The line under the question: what the default action does to the running apps. Each is short
/// enough for one line in the alert's 292 pt text column at 13 pt, so no word is left alone on a
/// second line.
pub fn consequence(action: SessionAction) -> &'static str {
    match action {
        SessionAction::Lock => "Apps keep running behind the lock screen.",
        SessionAction::LogOut => "Apps quit and your session ends.",
        SessionAction::Restart => "Apps quit and the computer restarts.",
        SessionAction::PowerOff => "Apps quit and the computer turns off.",
        SessionAction::Suspend => "The computer sleeps; apps stay open.",
    }
}

/// The alert's words when the session offers no action (no login manager reached, as in a bare
/// container): it says so, and Cancel is the only button.
pub const NONE_OFFERED_TITLE: &str = "No power actions available";
pub const NONE_OFFERED_MESSAGE: &str = "This session cannot shut down or restart the computer.";

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
