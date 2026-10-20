//! What each power action is called and asked as, in the Mac's words: the one place the
//! launcher's System rows, the bar's system menu and the power menu read them from.

use sill_launcher::actions::ActionId;
use sill_launcher::{Confirm, SessionAction};

use crate::actions::def;

/// The action's name: a launcher row's title and a power-menu button's label.
pub fn title(action: SessionAction) -> &'static str {
    def(ActionId::of_session(action)).label
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

/// The power menu's question about the action, in macOS's words.
pub fn prompt(action: SessionAction) -> &'static str {
    match action {
        SessionAction::Lock => "Are you sure you want to lock the screen now?",
        SessionAction::LogOut => "Are you sure you want to quit all apps and log out now?",
        SessionAction::Restart => "Are you sure you want to restart your computer now?",
        SessionAction::PowerOff => "Are you sure you want to shut down your computer now?",
        SessionAction::Suspend => "Are you sure you want to put your computer to sleep now?",
    }
}

/// What the notice says when `app` kept the session from ending: the action's name, said as
/// cancelled, and why.
pub fn held(action: SessionAction, app: &str) -> (String, String) {
    (
        format!("{} Cancelled", title(action)),
        format!("{app} is still open. Finish what it is asking, then try again."),
    )
}

/// How long the power menu waits before it carries out an action that asks first, as macOS's
/// shut-down dialog does.
pub const COUNTDOWN_SECONDS: u32 = 60;

/// The line under the question while the power menu counts down to an action that asks first:
/// what happens if the person does nothing, and in how long. `None` for an action that never
/// asks (Lock, Sleep), whose line is its [`consequence`].
pub fn countdown(action: SessionAction, seconds: u32) -> Option<String> {
    let unit = if seconds == 1 { "second" } else { "seconds" };
    let what = match action.confirm() {
        Confirm::Now => return None,
        Confirm::Ask => match action {
            SessionAction::LogOut => "you will be logged out",
            SessionAction::Restart => "your computer will restart",
            SessionAction::PowerOff | SessionAction::Lock | SessionAction::Suspend => {
                "your computer will shut down"
            }
        },
    };
    Some(format!(
        "If you do nothing, {what} automatically in {seconds} {unit}."
    ))
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
    use super::{aliases, consequence, held, menu_title, prompt, title};
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

    #[test]
    fn a_held_session_names_the_action_and_the_app() {
        let (summary, body) = held(SessionAction::LogOut, "Zen");
        assert_eq!(summary, "Log Out Cancelled");
        assert!(body.starts_with("Zen is still open."), "{body}");
    }
}
