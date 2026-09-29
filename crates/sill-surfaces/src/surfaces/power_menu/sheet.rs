//! One opening of the sheet, pure: the keyboard machine's focus ([`super::step`]) plus whether
//! the sheet is up or on its way out. Escape, a scrim click and a chosen action all leave the
//! same way, quire's `Sheet` playing its own exit (`sheet-out`, the scrim fading); at its
//! `on_hidden` the menu closes and, for a chosen action, the action runs, so a suspend or a
//! polkit prompt never finds the sheet still up. Shown again while leaving, it is up again.

use ds::Shown;
use sill_services::{Session, SessionAction};

use crate::surfaces::launcher::presence::OpenSerial;
use crate::surfaces::power_menu::keys::{
    PowerMenuKey, PowerMenuOut, PowerMenuState, offered, step,
};

/// What happens once the sheet has left.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Then {
    Close,
    Run(SessionAction),
}

/// Whether the sheet is up.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SheetPhase {
    Open,
    Leaving(Then),
}

impl SheetPhase {
    /// The panel's `data-look` word.
    pub fn slug(self) -> &'static str {
        match self {
            SheetPhase::Open => "open",
            SheetPhase::Leaving(_) => "leaving",
        }
    }

    /// quire's `Sheet { shown }` for this phase.
    pub fn shown(self) -> Shown {
        match self {
            SheetPhase::Open => Shown::Visible,
            SheetPhase::Leaving(_) => Shown::Hidden,
        }
    }
}

/// One opening's state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MenuSession {
    pub serial: OpenSerial,
    pub keys: PowerMenuState,
    pub phase: SheetPhase,
}

impl MenuSession {
    /// A fresh opening, focused on `keys`.
    pub fn opened(serial: OpenSerial, keys: PowerMenuState) -> MenuSession {
        MenuSession {
            serial,
            keys,
            phase: SheetPhase::Open,
        }
    }
}

/// What reaches the sheet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SheetInput {
    Key(PowerMenuKey),
    /// A button was clicked.
    Pressed(SessionAction),
    /// Escape through quire's `Sheet`, or a scrim click.
    Dismissed,
    /// The sheet's exit settled (quire's `on_hidden`).
    Settled,
    /// Opened again the same way while leaving.
    Reshown,
}

/// What the panel does after an input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SheetEffect {
    Nothing,
    /// The sheet started to leave: tell the link, which keeps the surface mapped meanwhile.
    Leave,
    /// Close the menu, then carry out `Then`.
    Finish(Then),
}

/// `menu` after `input`, with `session`'s offers.
pub fn advance(
    menu: MenuSession,
    input: SheetInput,
    session: &Session,
) -> (MenuSession, SheetEffect) {
    let leave = |keys: PowerMenuState, then: Then| {
        (
            MenuSession {
                keys,
                phase: SheetPhase::Leaving(then),
                ..menu
            },
            SheetEffect::Leave,
        )
    };
    match (menu.phase, input) {
        (SheetPhase::Open, SheetInput::Key(key)) => match step(menu.keys, key, session) {
            (keys, PowerMenuOut::Stay) => (MenuSession { keys, ..menu }, SheetEffect::Nothing),
            (keys, PowerMenuOut::Run(action)) => leave(keys, Then::Run(action)),
            (keys, PowerMenuOut::Close) => leave(keys, Then::Close),
        },
        (SheetPhase::Open, SheetInput::Pressed(action)) if offered(session, action) => {
            leave(PowerMenuState { focused: action }, Then::Run(action))
        }
        (SheetPhase::Open, SheetInput::Dismissed) => leave(menu.keys, Then::Close),
        (SheetPhase::Leaving(then), SheetInput::Settled) => (menu, SheetEffect::Finish(then)),
        (SheetPhase::Leaving(_), SheetInput::Reshown) => (
            MenuSession {
                phase: SheetPhase::Open,
                ..menu
            },
            SheetEffect::Nothing,
        ),
        _ => (menu, SheetEffect::Nothing),
    }
}

#[cfg(test)]
mod tests {
    use super::{MenuSession, SheetEffect, SheetInput, SheetPhase, Then, advance};
    use crate::surfaces::launcher::presence::OpenSerial;
    use crate::surfaces::power_menu::keys::{PowerMenuKey, PowerMenuState};
    use ds::Shown;
    use sill_services::{Offer, Session, SessionAction};

    fn menu(focused: SessionAction, phase: SheetPhase) -> MenuSession {
        MenuSession {
            serial: OpenSerial(3),
            keys: PowerMenuState { focused },
            phase,
        }
    }

    #[test]
    fn every_way_out_leaves_first_and_acts_once_settled() {
        use SessionAction::{LogOut, PowerOff, Restart, Suspend};
        use SheetPhase::{Leaving, Open};
        let all = Session::everything();
        let no_suspend = Session {
            suspend: Offer::Unavailable,
            ..all
        };
        let cases: Vec<(MenuSession, SheetInput, &Session, MenuSession, SheetEffect)> = vec![
            (
                menu(PowerOff, Open),
                SheetInput::Key(PowerMenuKey::Right),
                &all,
                menu(Suspend, Open),
                SheetEffect::Nothing,
            ),
            (
                menu(Restart, Open),
                SheetInput::Key(PowerMenuKey::Enter),
                &all,
                menu(Restart, Leaving(Then::Run(Restart))),
                SheetEffect::Leave,
            ),
            (
                menu(Restart, Open),
                SheetInput::Key(PowerMenuKey::Escape),
                &all,
                menu(Restart, Leaving(Then::Close)),
                SheetEffect::Leave,
            ),
            (
                menu(PowerOff, Open),
                SheetInput::Pressed(LogOut),
                &all,
                menu(LogOut, Leaving(Then::Run(LogOut))),
                SheetEffect::Leave,
            ),
            (
                menu(PowerOff, Open),
                SheetInput::Pressed(Suspend),
                &no_suspend,
                menu(PowerOff, Open),
                SheetEffect::Nothing,
            ),
            (
                menu(PowerOff, Open),
                SheetInput::Dismissed,
                &all,
                menu(PowerOff, Leaving(Then::Close)),
                SheetEffect::Leave,
            ),
            (
                menu(Restart, Leaving(Then::Run(Restart))),
                SheetInput::Pressed(Restart),
                &all,
                menu(Restart, Leaving(Then::Run(Restart))),
                SheetEffect::Nothing,
            ),
            (
                menu(Restart, Leaving(Then::Run(Restart))),
                SheetInput::Key(PowerMenuKey::Left),
                &all,
                menu(Restart, Leaving(Then::Run(Restart))),
                SheetEffect::Nothing,
            ),
            (
                menu(Restart, Leaving(Then::Run(Restart))),
                SheetInput::Settled,
                &all,
                menu(Restart, Leaving(Then::Run(Restart))),
                SheetEffect::Finish(Then::Run(Restart)),
            ),
            (
                menu(PowerOff, Open),
                SheetInput::Settled,
                &all,
                menu(PowerOff, Open),
                SheetEffect::Nothing,
            ),
            (
                menu(Restart, Leaving(Then::Run(Restart))),
                SheetInput::Reshown,
                &all,
                menu(Restart, Open),
                SheetEffect::Nothing,
            ),
            (
                menu(Restart, Open),
                SheetInput::Reshown,
                &all,
                menu(Restart, Open),
                SheetEffect::Nothing,
            ),
        ];
        for (from, input, session, want, effect) in cases {
            assert_eq!(
                advance(from, input, session),
                (want, effect),
                "{from:?} {input:?}"
            );
        }
        assert_eq!(Open.slug(), "open");
        assert_eq!(Leaving(Then::Close).slug(), "leaving");
        assert_eq!(Open.shown(), Shown::Visible);
        assert_eq!(Leaving(Then::Close).shown(), Shown::Hidden);
    }
}
