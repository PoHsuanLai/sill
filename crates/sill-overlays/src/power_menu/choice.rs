//! Which actions the power menu's alert offers and which one it is about, pure: the action an
//! opening starts on (the launcher row's, else `power_menu.default_action`) is the alert's
//! default, the one Return presses, and the alert's answers are that action first, the other
//! actions logind offers after it, and Cancel.

use sill_model::session::{Offer, Session, SessionAction};
use sill_settings::PowerMenuDefault;

use sill_ui_kit::links::Preselect;

/// The action an opening starts on: the preselected action (a launcher row's, else
/// `power_menu.default_action`) when logind offers it (`Available` or `NeedsAuth`), else the
/// first offered one in `SessionAction::POWER_MENU` order, else the preselected one anyway (no
/// action is then offered and the alert only says so and cancels).
pub fn initial(
    preselect: Preselect,
    default: PowerMenuDefault,
    session: &Session,
) -> SessionAction {
    let wanted = match preselect {
        Preselect::Action(action) => action,
        Preselect::Default => default_action(default),
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

/// The actions the alert answers with, in its order: `default` first when it is offered, then the
/// rest of `SessionAction::POWER_MENU` that logind offers.
pub fn answers(default: SessionAction, session: &Session) -> Vec<SessionAction> {
    let first = offered(session, default).then_some(default);
    let rest = SessionAction::POWER_MENU
        .into_iter()
        .filter(|&action| action != default && offered(session, action));
    first.into_iter().chain(rest).collect()
}

/// `power_menu.default_action` as the action it names.
fn default_action(default: PowerMenuDefault) -> SessionAction {
    match default {
        PowerMenuDefault::LogOut => SessionAction::LogOut,
        PowerMenuDefault::Restart => SessionAction::Restart,
        PowerMenuDefault::ShutDown => SessionAction::PowerOff,
        PowerMenuDefault::Suspend => SessionAction::Suspend,
    }
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
    use sill_settings::PowerMenuDefault;
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
        let cases: Vec<(Preselect, PowerMenuDefault, Session, SessionAction)> = vec![
            (
                Preselect::Default,
                PowerMenuDefault::ShutDown,
                session(Offer::Available),
                PowerOff,
            ),
            (
                Preselect::Default,
                PowerMenuDefault::Suspend,
                session(Offer::Available),
                Suspend,
            ),
            (
                Preselect::Default,
                PowerMenuDefault::Suspend,
                session(Offer::Unavailable),
                LogOut,
            ),
            (
                Preselect::Action(Restart),
                PowerMenuDefault::ShutDown,
                session(Offer::Available),
                Restart,
            ),
            (
                Preselect::Default,
                PowerMenuDefault::LogOut,
                Session::default(),
                LogOut,
            ),
        ];
        for (preselect, default, session, want) in cases {
            assert_eq!(
                initial(preselect, default, &session),
                want,
                "{preselect:?} {default:?}"
            );
        }
    }

    #[test]
    fn the_default_leads_and_only_offered_actions_answer() {
        use SessionAction::{LogOut, PowerOff, Restart, Suspend};
        let all = session(Offer::Available);
        let no_suspend = session(Offer::Unavailable);
        let none = Session::default();
        let cases: Vec<(SessionAction, &Session, Vec<SessionAction>)> = vec![
            (PowerOff, &all, vec![PowerOff, LogOut, Restart, Suspend]),
            (LogOut, &all, vec![LogOut, Restart, PowerOff, Suspend]),
            (Suspend, &no_suspend, vec![LogOut, Restart, PowerOff]),
            (PowerOff, &none, Vec::new()),
        ];
        for (default, session, want) in cases {
            assert_eq!(answers(default, session), want, "{default:?}");
        }
    }
}
