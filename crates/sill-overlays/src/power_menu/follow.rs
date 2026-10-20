//! What a change on the [`sill_ui_kit::links::PowerMenuLink`] asks of the panel's current session, pure: a
//! new opening starts one, the same opening shown again while its sheet leaves brings the sheet
//! back, and a hidden menu drops it.

use super::sheet::MenuSession;
use sill_ui_kit::links::{PowerMenuOpening, Showing};
use sill_ui_kit::sheet_stage::SheetExit;

/// What the panel does when the link changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Follow {
    Nothing,
    /// A new opening: a fresh session (the sheet is keyed by it, so it enters).
    Start,
    /// The same opening, shown again while its sheet leaves.
    Reshow,
    /// Hidden: drop the session.
    Drop,
}

/// What the link's `opening` (on this output) asks of the panel's current `menu`.
pub fn follow(opening: Option<&PowerMenuOpening>, menu: Option<MenuSession>) -> Follow {
    match (opening, menu) {
        (Some(opening), Some(menu)) if opening.serial == menu.serial => {
            match (opening.showing, menu.phase) {
                (Showing::Up, SheetExit::Leaving(_)) => Follow::Reshow,
                _ => Follow::Nothing,
            }
        }
        (Some(_), _) => Follow::Start,
        (None, Some(_)) => Follow::Drop,
        (None, None) => Follow::Nothing,
    }
}

#[cfg(test)]
mod tests {
    use super::{Follow, follow};
    use crate::power_menu::sheet::{MenuSession, Then};
    use shell_host::OutputId;
    use sill_model::session::SessionAction;
    use sill_ui_kit::links::{OpenSerial, PanelFocus};
    use sill_ui_kit::links::{PowerMenuOpening, PowerMenuPayload, Preselect, Showing};
    use sill_ui_kit::sheet_stage::SheetExit;

    fn menu(default: SessionAction, phase: SheetExit<Then>) -> MenuSession {
        MenuSession {
            serial: OpenSerial(1),
            default,
            phase,
        }
    }

    #[test]
    fn the_link_starts_reshows_or_drops_the_session() {
        use SessionAction::PowerOff;
        use SheetExit::{Leaving, Open};
        let opening = |serial: u64, showing: Showing| PowerMenuOpening {
            output: OutputId(1),
            serial: OpenSerial(serial),
            focus: PanelFocus::Awaited,
            payload: PowerMenuPayload {
                preselect: Preselect::Default,
                showing,
            },
        };
        let at = |serial: u64, phase: SheetExit<Then>| MenuSession {
            serial: OpenSerial(serial),
            ..menu(PowerOff, phase)
        };
        let leaving = Leaving(Then::Close);
        let cases = [
            (Some(opening(1, Showing::Up)), None, Follow::Start),
            (
                Some(opening(2, Showing::Up)),
                Some(at(1, Open)),
                Follow::Start,
            ),
            (
                Some(opening(1, Showing::Up)),
                Some(at(1, Open)),
                Follow::Nothing,
            ),
            (
                Some(opening(1, Showing::Leaving)),
                Some(at(1, leaving)),
                Follow::Nothing,
            ),
            (
                Some(opening(1, Showing::Up)),
                Some(at(1, leaving)),
                Follow::Reshow,
            ),
            (
                Some(opening(1, Showing::Leaving)),
                Some(at(1, Open)),
                Follow::Nothing,
            ),
            (None, Some(at(1, leaving)), Follow::Drop),
            (None, None, Follow::Nothing),
        ];
        for (link, now, want) in cases {
            assert_eq!(follow(link.as_ref(), now), want, "{link:?} {now:?}");
        }
    }
}
