//! The power menu's keyboard (design/20 §1.8: arrows move, Enter confirms, Esc closes), pure:
//! which button has focus, and what a key does to it.

use sill_model::session::{Offer, Session, SessionAction};
use sill_settings::PowerMenuDefault;

use crate::links::Preselect;

/// A key the menu handles; every other key is ignored.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PowerMenuKey {
    Left,
    Right,
    Up,
    Down,
    Tab,
    BackTab,
    Enter,
    Escape,
}

/// The focused button.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PowerMenuState {
    pub focused: SessionAction,
}

/// What a key asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PowerMenuOut {
    Stay,
    /// Carry out the action (then close).
    Run(SessionAction),
    Close,
}

/// The focus an opening starts with: the preselected action (a launcher row's, else
/// `power_menu.default_action`) when logind offers it (`Available` or `NeedsAuth`), else the
/// first offered one in `SessionAction::POWER_MENU` order, else the preselected one anyway (every
/// button is then unavailable and Enter does nothing).
pub fn initial(
    preselect: Preselect,
    default: PowerMenuDefault,
    session: &Session,
) -> PowerMenuState {
    let wanted = match preselect {
        Preselect::Action(action) => action,
        Preselect::Default => default_action(default),
    };
    let focused = if offered(session, wanted) {
        wanted
    } else {
        SessionAction::POWER_MENU
            .into_iter()
            .find(|&action| offered(session, action))
            .unwrap_or(wanted)
    };
    PowerMenuState { focused }
}

/// `state` after `key`. Left, Up and Shift+Tab move to the previous offered button, Right, Down
/// and Tab to the next, both wrapping and skipping unoffered ones; Enter runs the focused
/// action when it is offered (`NeedsAuth` counts: polkit asks) and stays otherwise; Escape
/// closes.
pub fn step(
    state: PowerMenuState,
    key: PowerMenuKey,
    session: &Session,
) -> (PowerMenuState, PowerMenuOut) {
    match key {
        PowerMenuKey::Left | PowerMenuKey::Up | PowerMenuKey::BackTab => {
            (moved(state, Direction::Back, session), PowerMenuOut::Stay)
        }
        PowerMenuKey::Right | PowerMenuKey::Down | PowerMenuKey::Tab => {
            (moved(state, Direction::On, session), PowerMenuOut::Stay)
        }
        PowerMenuKey::Enter if offered(session, state.focused) => {
            (state, PowerMenuOut::Run(state.focused))
        }
        PowerMenuKey::Enter => (state, PowerMenuOut::Stay),
        PowerMenuKey::Escape => (state, PowerMenuOut::Close),
    }
}

/// Which way the focus moves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Direction {
    Back,
    On,
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

/// The next offered button from `state` in `direction`, wrapping; `state` when none other is.
fn moved(state: PowerMenuState, direction: Direction, session: &Session) -> PowerMenuState {
    let buttons = SessionAction::POWER_MENU;
    let count = buttons.len();
    let at = buttons
        .iter()
        .position(|&action| action == state.focused)
        .unwrap_or(0);
    let step = match direction {
        Direction::On => 1,
        Direction::Back => count - 1,
    };
    (1..count)
        .map(|n| buttons[(at + n * step) % count])
        .find(|&action| offered(session, action))
        .map_or(state, |focused| PowerMenuState { focused })
}

#[cfg(test)]
mod tests {
    use super::{PowerMenuKey, PowerMenuOut, PowerMenuState, initial, step};
    use crate::links::Preselect;
    use sill_model::session::{Offer, Session, SessionAction};
    use sill_model::units::Presence;
    use sill_settings::PowerMenuDefault;

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

    fn at(focused: SessionAction) -> PowerMenuState {
        PowerMenuState { focused }
    }

    #[test]
    fn an_opening_focuses_the_preselected_offered_button() {
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
                initial(preselect, default, &session).focused,
                want,
                "{preselect:?} {default:?}"
            );
        }
    }

    #[test]
    fn arrows_move_and_wrap_over_offered_buttons_enter_runs_escape_closes() {
        use PowerMenuKey::{BackTab, Down, Enter, Escape, Left, Right, Tab, Up};
        use SessionAction::{LogOut, PowerOff, Restart, Suspend};
        let no_suspend = session(Offer::Unavailable);
        let all = session(Offer::Available);
        let cases: Vec<(
            SessionAction,
            PowerMenuKey,
            &Session,
            SessionAction,
            PowerMenuOut,
        )> = vec![
            (LogOut, Right, &all, Restart, PowerMenuOut::Stay),
            (Restart, Down, &all, PowerOff, PowerMenuOut::Stay),
            (Suspend, Tab, &all, LogOut, PowerMenuOut::Stay),
            (LogOut, Left, &all, Suspend, PowerMenuOut::Stay),
            (PowerOff, Right, &no_suspend, LogOut, PowerMenuOut::Stay),
            (LogOut, Up, &no_suspend, PowerOff, PowerMenuOut::Stay),
            (Restart, BackTab, &all, LogOut, PowerMenuOut::Stay),
            (Restart, Enter, &all, Restart, PowerMenuOut::Run(Restart)),
            (Suspend, Enter, &no_suspend, Suspend, PowerMenuOut::Stay),
            (PowerOff, Escape, &all, PowerOff, PowerMenuOut::Close),
        ];
        for (from, key, session, focused, out) in cases {
            assert_eq!(
                step(at(from), key, session),
                (at(focused), out),
                "{from:?} {key:?}"
            );
        }
    }
}
