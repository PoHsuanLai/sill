//! The session's power actions: the one list every surface offers them from (the launcher's
//! System rows, the bar's system menu, the power menu's buttons). What each is called and asked
//! as is `sill_model::session`'s; what logind allows of them is the session service's.

use serde::{Deserialize, Serialize};

/// One power action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionAction {
    Lock,
    LogOut,
    Restart,
    PowerOff,
    Suspend,
}

impl SessionAction {
    /// Every action, in the system menu's order (the Mac's: Sleep, Restart, Shut Down, then Lock
    /// Screen and Log Out).
    pub const ALL: [SessionAction; 5] = [
        SessionAction::Suspend,
        SessionAction::Restart,
        SessionAction::PowerOff,
        SessionAction::Lock,
        SessionAction::LogOut,
    ];

    /// The power menu's buttons, left to right (design/20 §1.8: Lock is not one of them).
    pub const POWER_MENU: [SessionAction; 4] = [
        SessionAction::LogOut,
        SessionAction::Restart,
        SessionAction::PowerOff,
        SessionAction::Suspend,
    ];

    /// Whether choosing the action asks first: Log Out, Restart and Shut Down end the session's
    /// apps, so the power menu asks about them; Lock and Sleep keep every app and happen at once.
    pub fn confirm(self) -> Confirm {
        match self {
            SessionAction::LogOut | SessionAction::Restart | SessionAction::PowerOff => {
                Confirm::Ask
            }
            SessionAction::Lock | SessionAction::Suspend => Confirm::Now,
        }
    }
}

/// Where choosing an action goes: the one answer every entrance shares, so no surface decides
/// "ask first or do it" for itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Route {
    /// To the power menu, which asks and then carries it out.
    Ask(SessionAction),
    /// To the session service, at once.
    Now(SessionAction),
}

impl SessionAction {
    /// Where choosing the action goes, by its [`confirm`](SessionAction::confirm).
    pub fn route(self) -> Route {
        match self.confirm() {
            Confirm::Ask => Route::Ask(self),
            Confirm::Now => Route::Now(self),
        }
    }
}

/// Whether an action is asked about before it is carried out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Confirm {
    /// The power menu opens on it and asks.
    Ask,
    /// It is carried out at once.
    Now,
}

#[cfg(test)]
mod tests {
    use super::{Confirm, Route, SessionAction};

    #[test]
    fn a_route_keeps_the_action_and_follows_its_confirm() {
        for action in SessionAction::ALL {
            let want = match action.confirm() {
                Confirm::Ask => Route::Ask(action),
                Confirm::Now => Route::Now(action),
            };
            assert_eq!(action.route(), want, "{action:?}");
        }
    }

    #[test]
    fn only_what_ends_the_apps_is_asked_about() {
        let asked: Vec<SessionAction> = SessionAction::ALL
            .into_iter()
            .filter(|action| action.confirm() == Confirm::Ask)
            .collect();
        assert_eq!(
            asked,
            vec![
                SessionAction::Restart,
                SessionAction::PowerOff,
                SessionAction::LogOut
            ]
        );
    }

    #[test]
    fn the_power_menu_offers_every_action_but_lock() {
        let mut menu = SessionAction::POWER_MENU.to_vec();
        menu.push(SessionAction::Lock);
        menu.sort_by_key(|action| SessionAction::ALL.iter().position(|a| a == action));
        assert_eq!(menu, SessionAction::ALL.to_vec());
    }
}
