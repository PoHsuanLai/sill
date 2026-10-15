//! One opening of the sheet, pure: the action the alert is about ([`super::choice`]) plus whether
//! the sheet is up or on its way out. Escape, a press outside the sheet and a chosen action all leave the
//! same way, quire's `Sheet` playing its own exit (`sheet-out`, the scrim fading); at its
//! `on_hidden` the menu closes and, for a chosen action, the action runs, so a suspend or a
//! polkit prompt never finds the sheet still up. Shown again while leaving, it is up again.

use sill_model::session::{Session, SessionAction};

use super::choice::offered;
use sill_ui_kit::links::OpenSerial;
use sill_ui_kit::sheet_stage::{ExitIn, ExitOut, SheetExit};

/// What happens once the sheet has left.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Then {
    Close,
    Run(SessionAction),
}

/// One opening's state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MenuSession {
    pub serial: OpenSerial,
    /// The action the alert is about: its default button, the one Return presses.
    pub default: SessionAction,
    pub phase: SheetExit<Then>,
}

impl MenuSession {
    /// A fresh opening about `default`.
    pub fn opened(serial: OpenSerial, default: SessionAction) -> MenuSession {
        MenuSession {
            serial,
            default,
            phase: SheetExit::Open,
        }
    }
}

/// What reaches the sheet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SheetInput {
    /// A button was clicked.
    Pressed(SessionAction),
    /// Cancel, Escape through the alert, or a press outside it.
    Dismissed,
    /// The sheet's exit settled (quire's `on_hidden`).
    Settled,
    /// Opened again the same way while leaving.
    Reshown,
}

/// `menu` after `input`, with `session`'s offers: the shared exit machine does the rest.
pub fn advance(
    menu: MenuSession,
    input: SheetInput,
    session: &Session,
) -> (MenuSession, ExitOut<Then>) {
    let Some(input) = exit_input(input, session) else {
        return (menu, ExitOut::Nothing);
    };
    let (phase, out) = menu.phase.advance(input);
    (MenuSession { phase, ..menu }, out)
}

/// The exit machine's input for `input`; `None` for a press of an action not offered.
fn exit_input(input: SheetInput, session: &Session) -> Option<ExitIn<Then>> {
    match input {
        SheetInput::Pressed(action) if offered(session, action) => {
            Some(ExitIn::Leave(Then::Run(action)))
        }
        SheetInput::Pressed(_) => None,
        SheetInput::Dismissed => Some(ExitIn::Leave(Then::Close)),
        SheetInput::Settled => Some(ExitIn::Settled),
        SheetInput::Reshown => Some(ExitIn::Reshown),
    }
}

#[cfg(test)]
mod tests {
    use super::{MenuSession, SheetInput, Then, advance};
    use sill_model::session::{Offer, Session, SessionAction};
    use sill_ui_kit::links::OpenSerial;
    use sill_ui_kit::sheet_stage::{ExitOut, SheetExit};

    fn menu(default: SessionAction, phase: SheetExit<Then>) -> MenuSession {
        MenuSession {
            serial: OpenSerial(3),
            default,
            phase,
        }
    }

    #[test]
    fn every_way_out_leaves_first_and_acts_once_settled() {
        use SessionAction::{LogOut, PowerOff, Restart, Suspend};
        use SheetExit::{Leaving, Open};
        let all = Session::everything();
        let no_suspend = Session {
            suspend: Offer::Unavailable,
            ..all
        };
        let cases: Vec<(
            MenuSession,
            SheetInput,
            &Session,
            MenuSession,
            ExitOut<Then>,
        )> = vec![
            (
                menu(PowerOff, Open),
                SheetInput::Pressed(LogOut),
                &all,
                menu(PowerOff, Leaving(Then::Run(LogOut))),
                ExitOut::Left,
            ),
            (
                menu(PowerOff, Open),
                SheetInput::Pressed(Suspend),
                &no_suspend,
                menu(PowerOff, Open),
                ExitOut::Nothing,
            ),
            (
                menu(PowerOff, Open),
                SheetInput::Dismissed,
                &all,
                menu(PowerOff, Leaving(Then::Close)),
                ExitOut::Left,
            ),
            (
                menu(Restart, Leaving(Then::Run(Restart))),
                SheetInput::Pressed(Restart),
                &all,
                menu(Restart, Leaving(Then::Run(Restart))),
                ExitOut::Nothing,
            ),
            (
                menu(Restart, Leaving(Then::Run(Restart))),
                SheetInput::Settled,
                &all,
                menu(Restart, Leaving(Then::Run(Restart))),
                ExitOut::Finished(Then::Run(Restart)),
            ),
            (
                menu(PowerOff, Open),
                SheetInput::Settled,
                &all,
                menu(PowerOff, Open),
                ExitOut::Nothing,
            ),
            (
                menu(Restart, Leaving(Then::Run(Restart))),
                SheetInput::Reshown,
                &all,
                menu(Restart, Open),
                ExitOut::Nothing,
            ),
            (
                menu(Restart, Open),
                SheetInput::Reshown,
                &all,
                menu(Restart, Open),
                ExitOut::Nothing,
            ),
        ];
        for (from, input, session, want, effect) in cases {
            assert_eq!(
                advance(from, input, session),
                (want, effect),
                "{from:?} {input:?}"
            );
        }
    }
}
