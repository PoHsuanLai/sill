//! Whether the power menu is showing, shared by every output's power menu surface, the daemon,
//! the bar and the launcher (the launcher's pattern: one `watch` channel holding a `Reveal`, pure
//! transitions, each surface mirroring it onto its own visibility).

use shell_host::{KeyboardFocus, OutputId};
use sill_model::session::SessionAction;
use tokio::sync::watch;

use super::{Opening, PanelLink, Reveal};

/// What a power menu opening carries beside the shared parts: which button starts focused, and
/// whether the sheet is up or playing its exit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PowerMenuPayload {
    /// The button focused on open.
    pub preselect: Preselect,
    /// Whether the sheet is up or playing its exit.
    pub showing: Showing,
}

/// One opening: where, which button starts focused, and which opening it is (a new one remounts
/// the sheet: its entrance plays, focus starts over).
pub type PowerMenuOpening = Opening<PowerMenuPayload>;

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
    /// The chosen action's own button (the launcher's or the system menu's Restart opens on Restart).
    Action(SessionAction),
}

/// `shown` with `showing` as the opening's sheet state, a hidden menu unchanged.
fn showing_as(shown: &Reveal<PowerMenuOpening>, showing: Showing) -> Reveal<PowerMenuOpening> {
    match shown {
        Reveal::Revealed(opening) => Reveal::Revealed(Opening {
            payload: PowerMenuPayload {
                showing,
                ..opening.payload
            },
            ..*opening
        }),
        Reveal::Hidden { .. } => shown.clone(),
    }
}

/// `shown` opened on `output` focusing `preselect`. Opening an open menu on the same output
/// with the same preselect keeps it as it is (no second entrance), and shows it again if it
/// was leaving (the sheet enters again); anything else is a new opening.
fn open(
    shown: &Reveal<PowerMenuOpening>,
    output: OutputId,
    preselect: Preselect,
) -> Reveal<PowerMenuOpening> {
    match shown.opening() {
        Some(opening) if opening.output == output && opening.preselect == preselect => {
            showing_as(shown, Showing::Up)
        }
        _ => shown.reopen(
            output,
            PowerMenuPayload {
                preselect,
                showing: Showing::Up,
            },
        ),
    }
}

/// `power-menu toggle`: open (at the default button) when hidden or leaving, hide when up.
fn toggle(shown: &Reveal<PowerMenuOpening>, output: OutputId) -> Reveal<PowerMenuOpening> {
    match shown.opening().map(|opening| opening.showing) {
        Some(Showing::Up) => shown.hidden(),
        Some(Showing::Leaving) | None => open(shown, output, Preselect::Default),
    }
}

/// The sheet started its exit: still shown (the surface stays mapped) until it settles.
fn leaving(shown: &Reveal<PowerMenuOpening>) -> Reveal<PowerMenuOpening> {
    showing_as(shown, Showing::Leaving)
}

/// The shared channel, given to every surface as a root context and kept by the daemon. Clone
/// is cheap.
#[derive(Debug, Clone, Default)]
pub struct PowerMenuLink {
    panel: PanelLink<PowerMenuPayload>,
}

impl PowerMenuLink {
    /// A hidden power menu.
    pub fn new() -> PowerMenuLink {
        PowerMenuLink::default()
    }

    /// The current value.
    pub fn now(&self) -> Reveal<PowerMenuOpening> {
        self.panel.now()
    }

    /// Open on `output`, focusing `preselect`.
    pub fn open(&self, output: OutputId, preselect: Preselect) {
        self.panel.update(|shown| open(shown, output, preselect));
    }

    /// `power-menu toggle` on `output`.
    pub fn toggle(&self, output: OutputId) {
        self.panel.update(|shown| toggle(shown, output));
    }

    /// `power-menu close`, Escape, a scrim click or a chosen action.
    pub fn close(&self) {
        self.panel.close();
    }

    /// The sheet started its exit (Escape, a scrim click or a chosen action).
    pub fn leaving(&self) {
        self.panel.update(leaving);
    }

    /// The surface's keyboard focus changed.
    pub fn focus(&self, focus: KeyboardFocus) {
        self.panel.focus(focus);
    }

    /// A receiver for a surface to follow.
    pub fn subscribe(&self) -> watch::Receiver<Reveal<PowerMenuOpening>> {
        self.panel.subscribe()
    }
}

#[cfg(test)]
mod tests {
    use super::{leaving, open, toggle};
    use crate::links::{OpenSerial, PanelFocus, Reveal};
    use crate::links::{PowerMenuLink, PowerMenuOpening, Preselect, Showing};
    use shell_host::{OutputId, Visibility};
    use sill_model::session::SessionAction;

    const ONE: OutputId = OutputId(1);
    const TWO: OutputId = OutputId(2);

    #[test]
    fn toggle_opens_at_the_default_and_closes_counting_openings() {
        let first = toggle(&Reveal::default(), ONE);
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
            Reveal::Hidden {
                last: OpenSerial(1)
            }
        );
        assert_eq!(toggle(&closed, ONE).serial(), OpenSerial(2));
    }

    #[test]
    fn opening_again_the_same_way_is_no_new_opening() {
        let restart = Preselect::Action(SessionAction::Restart);
        let shown = open(&Reveal::default(), ONE, restart);
        assert_eq!(open(&shown, ONE, restart), shown, "same output and button");
        assert_eq!(open(&shown, TWO, restart).serial(), OpenSerial(2), "moved");
        assert_eq!(
            open(&shown, ONE, Preselect::Default).serial(),
            OpenSerial(2),
            "another button"
        );
        assert_eq!(shown.shows_on(ONE), Visibility::Shown);
        assert_eq!(shown.shows_on(TWO), Visibility::Hidden);
    }

    #[test]
    fn a_leaving_menu_opened_again_the_same_way_is_shown_again_not_reopened() {
        let showing = |shown: &Reveal<PowerMenuOpening>| shown.opening().map(|o| o.showing);
        let up = open(&Reveal::default(), ONE, Preselect::Default);
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
