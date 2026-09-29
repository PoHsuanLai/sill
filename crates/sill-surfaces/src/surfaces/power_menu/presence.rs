//! Whether the power menu is showing, shared by every output's power menu surface, the daemon,
//! the bar and the launcher (the launcher's `presence.rs` pattern: one `watch` channel, pure
//! transitions, each surface mirroring it onto its own visibility).

use shell_host::{Focus, OutputId};
use sill_model::session::SessionAction;
use tokio::sync::watch;

use crate::surfaces::launcher::presence::{OpenSerial, PanelFocus, Visible};

/// The power menu's one piece of shared state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PowerMenuShown {
    /// Hidden, remembering the last opening's serial so the next one counts on.
    Hidden {
        last: OpenSerial,
    },
    Shown(PowerMenuOpening),
}

impl Default for PowerMenuShown {
    fn default() -> Self {
        PowerMenuShown::Hidden {
            last: OpenSerial::default(),
        }
    }
}

/// One opening: where, which button starts focused, and which opening it is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PowerMenuOpening {
    pub output: OutputId,
    /// The button focused on open.
    pub preselect: Preselect,
    /// A new opening remounts the sheet (its entrance plays, focus starts over).
    pub serial: OpenSerial,
    /// Whether the surface has held keyboard focus since it opened.
    pub focus: PanelFocus,
    /// Whether the sheet is up or playing its exit.
    pub showing: Showing,
}

/// Whether an opening's sheet is up or on its way out (quire's `Sheet` plays the exit; the
/// menu is hidden only once it has settled).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Showing {
    Up,
    Leaving,
}

/// Which button an opening focuses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Preselect {
    /// `power_menu.default_action`.
    Default,
    /// A launcher row's own action (its Restart row opens on Restart).
    Action(SessionAction),
}

impl PowerMenuShown {
    /// The opening, while shown.
    pub fn opening(&self) -> Option<&PowerMenuOpening> {
        match self {
            PowerMenuShown::Shown(opening) => Some(opening),
            PowerMenuShown::Hidden { .. } => None,
        }
    }

    /// Whether the menu shows on `output`.
    pub fn shows_on(&self, output: OutputId) -> Visible {
        match self.opening() {
            Some(opening) if opening.output == output => Visible::Yes,
            _ => Visible::No,
        }
    }

    /// The current or last opening's serial.
    pub fn serial(&self) -> OpenSerial {
        match self {
            PowerMenuShown::Shown(opening) => opening.serial,
            PowerMenuShown::Hidden { last } => *last,
        }
    }

    /// Hidden, keeping the serial.
    pub fn hidden(&self) -> PowerMenuShown {
        PowerMenuShown::Hidden {
            last: self.serial(),
        }
    }
}

/// `shown` opened on `output` focusing `preselect`. Opening an open menu on the same output
/// with the same preselect keeps it as it is (no second entrance), and shows it again if it
/// was leaving (the sheet enters again); anything else is a new opening.
pub fn open(shown: &PowerMenuShown, output: OutputId, preselect: Preselect) -> PowerMenuShown {
    match shown.opening() {
        Some(opening) if opening.output == output && opening.preselect == preselect => {
            PowerMenuShown::Shown(PowerMenuOpening {
                showing: Showing::Up,
                ..opening.clone()
            })
        }
        _ => PowerMenuShown::Shown(PowerMenuOpening {
            output,
            preselect,
            serial: OpenSerial(shown.serial().0 + 1),
            focus: PanelFocus::Awaited,
            showing: Showing::Up,
        }),
    }
}

/// `power-menu toggle`: open (at the default button) when hidden or leaving, hide when up.
pub fn toggle(shown: &PowerMenuShown, output: OutputId) -> PowerMenuShown {
    match shown.opening().map(|opening| opening.showing) {
        Some(Showing::Up) => shown.hidden(),
        Some(Showing::Leaving) | None => open(shown, output, Preselect::Default),
    }
}

/// The sheet started its exit: still shown (the surface stays mapped) until it settles.
pub fn leaving(shown: &PowerMenuShown) -> PowerMenuShown {
    match shown {
        PowerMenuShown::Shown(opening) => PowerMenuShown::Shown(PowerMenuOpening {
            showing: Showing::Leaving,
            ..opening.clone()
        }),
        PowerMenuShown::Hidden { .. } => shown.clone(),
    }
}

/// The surface's keyboard focus changed: gaining it is noted, losing it after holding it
/// closes the menu.
pub fn after_focus(shown: &PowerMenuShown, focus: Focus) -> PowerMenuShown {
    match (shown, focus) {
        (PowerMenuShown::Shown(opening), Focus::Gained) => {
            PowerMenuShown::Shown(PowerMenuOpening {
                focus: PanelFocus::Held,
                ..opening.clone()
            })
        }
        (PowerMenuShown::Shown(opening), Focus::Lost) if opening.focus == PanelFocus::Held => {
            shown.hidden()
        }
        _ => shown.clone(),
    }
}

/// The shared channel, given to every surface as a root context and kept by the daemon. Clone
/// is cheap.
#[derive(Debug, Clone)]
pub struct PowerMenuLink {
    shown: watch::Sender<PowerMenuShown>,
}

impl Default for PowerMenuLink {
    fn default() -> Self {
        PowerMenuLink::new()
    }
}

impl PowerMenuLink {
    /// A hidden power menu.
    pub fn new() -> PowerMenuLink {
        PowerMenuLink {
            shown: watch::Sender::new(PowerMenuShown::default()),
        }
    }

    /// The current value.
    pub fn now(&self) -> PowerMenuShown {
        self.shown.borrow().clone()
    }

    /// Open on `output`, focusing `preselect`.
    pub fn open(&self, output: OutputId, preselect: Preselect) {
        self.shown
            .send_if_modified(|shown| replace(shown, open(shown, output, preselect)));
    }

    /// `power-menu toggle` on `output`.
    pub fn toggle(&self, output: OutputId) {
        self.shown
            .send_if_modified(|shown| replace(shown, toggle(shown, output)));
    }

    /// `power-menu close`, Escape, a scrim click or a chosen action.
    pub fn close(&self) {
        self.shown
            .send_if_modified(|shown| replace(shown, shown.hidden()));
    }

    /// The sheet started its exit (Escape, a scrim click or a chosen action).
    pub fn leaving(&self) {
        self.shown
            .send_if_modified(|shown| replace(shown, leaving(shown)));
    }

    /// The surface's keyboard focus changed.
    pub fn focus(&self, focus: Focus) {
        self.shown
            .send_if_modified(|shown| replace(shown, after_focus(shown, focus)));
    }

    /// A receiver for a surface to follow.
    pub fn subscribe(&self) -> watch::Receiver<PowerMenuShown> {
        self.shown.subscribe()
    }
}

/// Store `next`, saying whether it differs.
fn replace(shown: &mut PowerMenuShown, next: PowerMenuShown) -> bool {
    let changed = *shown != next;
    *shown = next;
    changed
}

#[cfg(test)]
mod tests {
    use super::{
        PowerMenuLink, PowerMenuShown, Preselect, Showing, after_focus, leaving, open, toggle,
    };
    use crate::surfaces::launcher::presence::{OpenSerial, PanelFocus, Visible};
    use shell_host::{Focus, OutputId};
    use sill_model::session::SessionAction;

    const ONE: OutputId = OutputId(1);
    const TWO: OutputId = OutputId(2);

    #[test]
    fn toggle_opens_at_the_default_and_closes_counting_openings() {
        let first = toggle(&PowerMenuShown::default(), ONE);
        let opening = first.opening().cloned();
        assert_eq!(opening.as_ref().map(|o| o.serial), Some(OpenSerial(1)));
        assert_eq!(
            opening.as_ref().map(|o| o.preselect),
            Some(Preselect::Default)
        );
        assert_eq!(opening.map(|o| o.focus), Some(PanelFocus::Awaited));
        let closed = toggle(&first, ONE);
        assert_eq!(
            closed,
            PowerMenuShown::Hidden {
                last: OpenSerial(1)
            }
        );
        assert_eq!(toggle(&closed, ONE).serial(), OpenSerial(2));
    }

    #[test]
    fn opening_again_the_same_way_is_no_new_opening() {
        let restart = Preselect::Action(SessionAction::Restart);
        let shown = open(&PowerMenuShown::default(), ONE, restart);
        assert_eq!(open(&shown, ONE, restart), shown, "same output and button");
        assert_eq!(open(&shown, TWO, restart).serial(), OpenSerial(2), "moved");
        assert_eq!(
            open(&shown, ONE, Preselect::Default).serial(),
            OpenSerial(2),
            "another button"
        );
        assert_eq!(shown.shows_on(ONE), Visible::Yes);
        assert_eq!(shown.shows_on(TWO), Visible::No);
    }

    #[test]
    fn a_leaving_menu_opened_again_the_same_way_is_shown_again_not_reopened() {
        let showing = |shown: &PowerMenuShown| shown.opening().map(|o| o.showing);
        let up = open(&PowerMenuShown::default(), ONE, Preselect::Default);
        let going = leaving(&up);
        let cases = [
            (showing(&going), Some(Showing::Leaving), "leaving"),
            (
                showing(&open(&going, ONE, Preselect::Default)),
                Some(Showing::Up),
                "shown again",
            ),
            (
                showing(&toggle(&going, ONE)),
                Some(Showing::Up),
                "toggle shows again",
            ),
            (showing(&toggle(&up, ONE)), None, "toggle hides an up menu"),
            (
                showing(&leaving(&up.hidden())),
                None,
                "a hidden menu stays hidden",
            ),
        ];
        for (got, want, what) in cases {
            assert_eq!(got, want, "{what}");
        }
        assert_eq!(
            open(&going, ONE, Preselect::Default),
            up,
            "the same opening"
        );
        assert_eq!(
            open(&going, TWO, Preselect::Default).serial(),
            OpenSerial(2),
            "elsewhere is a new opening"
        );
    }

    #[test]
    fn a_focus_loss_closes_only_a_menu_that_held_focus() {
        let shown = open(&PowerMenuShown::default(), ONE, Preselect::Default);
        assert_eq!(after_focus(&shown, Focus::Lost), shown);
        let held = after_focus(&shown, Focus::Gained);
        assert_eq!(after_focus(&held, Focus::Lost), shown.hidden());
    }

    #[test]
    fn the_link_notifies_only_on_change() {
        let link = PowerMenuLink::new();
        let mut rx = link.subscribe();
        link.close();
        assert!(!rx.has_changed().unwrap_or(true), "closing a closed menu");
        link.open(ONE, Preselect::Default);
        assert!(rx.has_changed().unwrap_or(false));
        let _ = rx.borrow_and_update();
        link.open(ONE, Preselect::Default);
        assert!(!rx.has_changed().unwrap_or(true), "opening an open menu");
        link.toggle(ONE);
        assert_eq!(rx.borrow_and_update().opening(), None);
    }
}
