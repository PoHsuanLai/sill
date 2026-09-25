//! The power menu's words and button variants, pure.

use ds::ButtonVariant;
use sill_services::SessionAction;

/// A button's label (the launcher's row titles: "Log Out", "Restart", "Shut Down", "Suspend";
/// "Lock" for completeness).
pub fn action_label(action: SessionAction) -> &'static str {
    let _ = action;
    todo!("M5 fill: power-menu (d)")
}

/// A button's quire variant (design/20 §1.8 `Button{Primary, Secondary, Danger}`): the focused
/// button `Primary`; Restart and Shut Down `Danger` (they end every running app); the rest
/// `Secondary`.
pub fn button_variant(action: SessionAction, focused: SessionAction) -> ButtonVariant {
    let _ = (action, focused);
    todo!("M5 fill: power-menu (d)")
}

#[cfg(test)]
mod tests {
    use super::{action_label, button_variant};
    use ds::ButtonVariant;
    use sill_services::SessionAction;

    #[test]
    #[ignore = "M5 fill: power-menu (d)"]
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
        for (action, focused, want) in cases {
            assert_eq!(
                button_variant(action, focused),
                want,
                "{action:?} {focused:?}"
            );
        }
    }
}
