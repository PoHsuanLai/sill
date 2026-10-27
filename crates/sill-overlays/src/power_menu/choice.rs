//! Which action the power menu's alert is about, pure: the action an opening starts on (the system
//! menu's or the launcher row's, else Shut Down). As macOS's shut-down dialog
//! does, the alert asks about that one action only: its button, the one Return presses, and
//! Cancel.

use sill_model::session::{Offer, Session, SessionAction};
use sill_ui_kit::links::Preselect;

/// The action an opening starts on: the preselected action (a launcher row's, else Shut Down,
/// as macOS's dialog focuses) when logind offers it (`Available` or `NeedsAuth`), else the
/// first offered one in `SessionAction::POWER_MENU` order, else the preselected one anyway (no
/// action is then offered and the alert only says so and cancels).
pub fn initial(preselect: Preselect, session: &Session) -> SessionAction {
    let wanted = match preselect {
        Preselect::Action(action) => action,
        Preselect::Default => SessionAction::PowerOff,
    };
    if offered(session, wanted) {
        wanted
    } else {
        SessionAction::POWER_MENU
            .into_iter()
            .find(|&action| offered(session, action))
            .unwrap_or(wanted)
    }
}

/// The action the alert answers with besides Cancel: `default`, when logind offers it.
pub fn answers(default: SessionAction, session: &Session) -> Vec<SessionAction> {
    offered(session, default)
        .then_some(default)
        .into_iter()
        .collect()
}

/// Whether logind will carry `action` out (polkit asking first counts).
pub(crate) fn offered(session: &Session, action: SessionAction) -> bool {
    matches!(session.offer(action), Offer::Available | Offer::NeedsAuth)
}

#[cfg(test)]
mod tests {
    use super::{answers, initial};
    use sill_model::session::{Offer, Session, SessionAction};
    use sill_model::units::Presence;
    use sill_ui_kit::links::Preselect;

    fn session(suspend: Offer) -> Session {
        Session {
            presence: Presence::Present,
            lock: Offer::Available,
            log_out: Offer::Available,
            restart: Offer::NeedsAuth,
            power_off: Offer::Available,
            suspend,
        }
    }

    #[test]
    fn an_opening_starts_on_the_preselected_offered_action() {
        use SessionAction::{LogOut, PowerOff, Restart, Suspend};
        let cases: Vec<(Preselect, Session, SessionAction)> = vec![
            (Preselect::Default, session(Offer::Available), PowerOff),
            (
                Preselect::Default,
                Session {
                    power_off: Offer::Unavailable,
                    ..session(Offer::Available)
                },
                LogOut,
            ),
            (
                Preselect::Action(Restart),
                session(Offer::Available),
                Restart,
            ),
            (
                Preselect::Action(Suspend),
                session(Offer::Unavailable),
                LogOut,
            ),
            (Preselect::Default, Session::default(), PowerOff),
        ];
        for (preselect, session, want) in cases {
            assert_eq!(initial(preselect, &session), want, "{preselect:?}");
        }
    }

    #[test]
    fn the_alert_answers_with_its_own_action_when_offered() {
        use SessionAction::{LogOut, PowerOff, Restart, Suspend};
        let all = session(Offer::Available);
        let no_suspend = session(Offer::Unavailable);
        let none = Session::default();
        let cases: Vec<(SessionAction, &Session, Vec<SessionAction>)> = vec![
            (PowerOff, &all, vec![PowerOff]),
            (LogOut, &all, vec![LogOut]),
            (Restart, &no_suspend, vec![Restart]),
            (Suspend, &no_suspend, Vec::new()),
            (PowerOff, &none, Vec::new()),
        ];
        for (default, session, want) in cases {
            assert_eq!(answers(default, session), want, "{default:?}");
        }
    }
}
