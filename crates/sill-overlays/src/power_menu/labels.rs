//! The power menu's words and button variants, pure.

use ds::{ButtonSize, ButtonVariant};
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

/// A button's quire variant (design/20 §1.8 `Button{Primary, Secondary, Danger}`): the focused
/// button `Primary`; Restart and Shut Down `Danger` (they end every running app); the rest
/// `Secondary`.
pub fn button_variant(action: SessionAction, focused: SessionAction) -> ButtonVariant {
    match action {
        _ if action == focused => ButtonVariant::Primary,
        SessionAction::Restart | SessionAction::PowerOff => ButtonVariant::Danger,
        SessionAction::Lock | SessionAction::LogOut | SessionAction::Suspend => {
            ButtonVariant::Secondary
        }
    }
}

/// A button's quire size: `Danger` is Mini-sized at rest, so it takes Primary's box to stand
/// level with the rest (quire's `ButtonSize::Regular`); the others keep their own.
pub fn button_size(variant: ButtonVariant) -> Option<ButtonSize> {
    match variant {
        ButtonVariant::Danger => Some(ButtonSize::Regular),
        _ => None,
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
    use super::{action_label, button_size, button_variant, consequence, prompt};
    use ds::{ButtonSize, ButtonVariant};
    use sill_model::session::SessionAction;

    #[test]
    fn every_button_has_its_label_and_variant() {
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
            (PowerOff, PowerOff, ButtonVariant::Primary),
            (Restart, PowerOff, ButtonVariant::Danger),
            (PowerOff, LogOut, ButtonVariant::Danger),
            (LogOut, PowerOff, ButtonVariant::Secondary),
            (Suspend, PowerOff, ButtonVariant::Secondary),
        ];
        for &(action, _) in LABELS {
            assert!(prompt(action).ends_with('?'), "{action:?}");
            assert!(consequence(action).ends_with('.'), "{action:?}");
        }
        for (action, focused, want) in cases {
            assert_eq!(
                button_variant(action, focused),
                want,
                "{action:?} {focused:?}"
            );
        }
    }

    #[test]
    fn danger_buttons_stand_level_with_the_rest() {
        let cases = [
            (ButtonVariant::Danger, Some(ButtonSize::Regular)),
            (ButtonVariant::Primary, None),
            (ButtonVariant::Secondary, None),
        ];
        for (variant, want) in cases {
            assert_eq!(button_size(variant), want, "{variant:?}");
        }
    }
}
