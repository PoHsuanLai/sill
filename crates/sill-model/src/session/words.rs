//! What each power action is called and asked as, in the Mac's words: the one place the
//! launcher's System rows, the bar's system menu and the power menu read them from.

use sill_launcher::{Confirm, SessionAction};

/// The action's name: a launcher row's title and a power-menu button's label.
pub fn title(action: SessionAction) -> &'static str {
    match action {
        SessionAction::Lock => "Lock Screen",
        SessionAction::LogOut => "Log Out",
        SessionAction::Restart => "Restart",
        SessionAction::PowerOff => "Shut Down",
        SessionAction::Suspend => "Sleep",
    }
}

/// The action as a menu row: its name, with an ellipsis when choosing it asks first (the Mac's
/// rule for a command that opens a dialog).
pub fn menu_title(action: SessionAction) -> String {
    match action.confirm() {
        Confirm::Ask => format!("{}…", title(action)),
        Confirm::Now => title(action).to_owned(),
    }
}

/// Other words a person may type for the action: the names other systems give it, in one
/// word and in two.
pub fn aliases(action: SessionAction) -> &'static [&'static str] {
    match action {
        SessionAction::Lock => &["Lock"],
        SessionAction::LogOut => &["Logout", "Log Off", "Sign Out"],
        SessionAction::Restart => &["Reboot"],
        SessionAction::PowerOff => &["Shutdown", "Power Off", "Turn Off"],
        SessionAction::Suspend => &["Suspend"],
    }
}

/// The power menu's question about the action, as macOS's shut-down dialog asks it.
pub fn prompt(action: SessionAction) -> &'static str {
    match action {
        SessionAction::Lock => "Lock the screen now?",
        SessionAction::LogOut => "Log out now?",
        SessionAction::Restart => "Restart this computer now?",
        SessionAction::PowerOff => "Shut down this computer now?",
        SessionAction::Suspend => "Put this computer to sleep now?",
    }
}

/// The line under the question: what the action does to the running apps. Each is short
/// enough for one line in the alert's 292 pt text column at 13 pt.
pub fn consequence(action: SessionAction) -> &'static str {
    match action {
        SessionAction::Lock => "Apps keep running behind the lock screen.",
        SessionAction::LogOut => "Apps quit and your session ends.",
        SessionAction::Restart => "Apps quit and the computer restarts.",
        SessionAction::PowerOff => "Apps quit and the computer turns off.",
        SessionAction::Suspend => "The computer sleeps; apps stay open.",
    }
}

#[cfg(test)]
mod tests {
    use super::{aliases, consequence, menu_title, prompt, title};
    use sill_launcher::SessionAction;

    #[test]
    fn every_action_has_its_words() {
        use SessionAction::{Lock, LogOut, PowerOff, Restart, Suspend};
        const WORDS: &[(SessionAction, &str, &str)] = &[
            (Lock, "Lock Screen", "Lock Screen"),
            (LogOut, "Log Out", "Log Out…"),
            (Restart, "Restart", "Restart…"),
            (PowerOff, "Shut Down", "Shut Down…"),
            (Suspend, "Sleep", "Sleep"),
        ];
        for &(action, name, row) in WORDS {
            assert_eq!(title(action), name, "{action:?}");
            assert_eq!(menu_title(action), row, "{action:?}");
            assert!(prompt(action).ends_with('?'), "{action:?}");
            assert!(consequence(action).ends_with('.'), "{action:?}");
            assert!(!aliases(action).is_empty(), "{action:?}");
        }
    }
}
