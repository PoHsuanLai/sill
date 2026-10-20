//! Whether a panel is showing, and the opening it shows, shared by every surface that follows
//! one link and the daemon that writes it: `Reveal<O>` is a link's value. The transitions that
//! hold for every panel (the serial that counts openings, the hidden value that remembers it,
//! what a keyboard focus change does) live here; what an opening carries is the panel's own `O`.

use std::ops::Deref;

use shell_host::{KeyboardFocus, OutputId, Visibility};

use super::Shared;

/// Counts openings; each is one more than the last.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct OpenSerial(pub u64);

/// A focus loss only closes a panel that had focus: the leave the compositor sends for the
/// previous opening's unmap can arrive after the next opening began.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PanelFocus {
    /// Opened, no keyboard focus yet.
    Awaited,
    /// The panel has held keyboard focus.
    Held,
}

/// One opening of a panel: where it shows, which opening it is, and whether it has had keyboard
/// focus.
pub trait Opened: Clone {
    /// The output whose panel shows.
    fn output(&self) -> OutputId;
    /// Which opening this is: a new one remounts the panel's contents.
    fn serial(&self) -> OpenSerial;
    /// Whether the panel has held keyboard focus since it opened.
    fn focus(&self) -> PanelFocus;
    /// This opening once the panel has gained keyboard focus.
    fn focused(self) -> Self;
}

/// One opening of any panel: the parts every panel's opening has, and the panel's own `payload`
/// (what the launcher opens with, which button the power menu focuses; `()` when there is
/// nothing more). A payload's fields read through the opening (`opening.query`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Opening<P> {
    /// The output whose panel shows.
    pub output: OutputId,
    /// Which opening this is: a new one remounts the panel's contents.
    pub serial: OpenSerial,
    /// Whether the panel has held keyboard focus since it opened.
    pub focus: PanelFocus,
    pub payload: P,
}

impl<P> Deref for Opening<P> {
    type Target = P;

    fn deref(&self) -> &P {
        &self.payload
    }
}

impl<P: Clone> Opened for Opening<P> {
    fn output(&self) -> OutputId {
        self.output
    }

    fn serial(&self) -> OpenSerial {
        self.serial
    }

    fn focus(&self) -> PanelFocus {
        self.focus
    }

    fn focused(self) -> Self {
        Opening {
            focus: PanelFocus::Held,
            ..self
        }
    }
}

/// A link's value: hidden, remembering the last opening's serial so the next one counts on, or
/// revealed as one opening.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reveal<O> {
    Hidden { last: OpenSerial },
    Revealed(O),
}

impl<O> Default for Reveal<O> {
    fn default() -> Self {
        Reveal::Hidden {
            last: OpenSerial::default(),
        }
    }
}

impl<O: Opened> Reveal<O> {
    /// The opening, while revealed.
    pub fn opening(&self) -> Option<&O> {
        match self {
            Reveal::Revealed(opening) => Some(opening),
            Reveal::Hidden { .. } => None,
        }
    }

    /// Whether the panel shows on `output`.
    pub fn shows_on(&self, output: OutputId) -> Visibility {
        match self.opening() {
            Some(opening) if opening.output() == output => Visibility::Shown,
            _ => Visibility::Hidden,
        }
    }

    /// The current or last opening's serial.
    pub fn serial(&self) -> OpenSerial {
        match self {
            Reveal::Revealed(opening) => opening.serial(),
            Reveal::Hidden { last } => *last,
        }
    }

    /// Hidden, keeping the serial.
    pub fn hidden(&self) -> Reveal<O> {
        Reveal::Hidden {
            last: self.serial(),
        }
    }

    /// Whether the panel shows on `output`, as a plain question.
    pub fn open_on(&self, output: OutputId) -> bool {
        self.shows_on(output) == Visibility::Shown
    }

    /// The panel's keyboard focus changed: gaining it is noted, losing it after holding it
    /// closes the panel.
    pub fn after_focus(&self, focus: KeyboardFocus) -> Reveal<O> {
        match (self, focus) {
            (Reveal::Revealed(opening), KeyboardFocus::Gained) => {
                Reveal::Revealed(opening.clone().focused())
            }
            (Reveal::Revealed(opening), KeyboardFocus::Lost)
                if opening.focus() == PanelFocus::Held =>
            {
                self.hidden()
            }
            _ => self.clone(),
        }
    }
}

impl<P: Clone + PartialEq> Reveal<Opening<P>> {
    /// Opened on `output` with `payload`. An open panel on the same output with the same payload
    /// stays the same opening (no second entrance); anything else is a new one.
    pub fn open(&self, output: OutputId, payload: P) -> Reveal<Opening<P>> {
        match self.opening() {
            Some(open) if open.output == output && open.payload == payload => self.clone(),
            Some(_) | None => self.reopen(output, payload),
        }
    }

    /// Opened on `output` with `payload` as a new opening, even over an open one.
    pub fn reopen(&self, output: OutputId, payload: P) -> Reveal<Opening<P>> {
        Reveal::Revealed(Opening {
            output,
            serial: OpenSerial(self.serial().0 + 1),
            focus: PanelFocus::Awaited,
            payload,
        })
    }

    /// Hidden when open, else opened on `output` with `payload`.
    pub fn toggle(&self, output: OutputId, payload: P) -> Reveal<Opening<P>> {
        match self {
            Reveal::Revealed(_) => self.hidden(),
            Reveal::Hidden { .. } => self.reopen(output, payload),
        }
    }
}

/// A panel's link: the value every one of its surfaces follows and the daemon, the bar and the
/// panel itself write. The verbs every panel has are here once; what a panel adds (the power
/// menu's exit, the launcher's mode switch) it does through [`PanelLink::update`]. Clone is cheap.
#[derive(Debug)]
pub struct PanelLink<P> {
    shown: Shared<Reveal<Opening<P>>>,
}

impl<P> Default for PanelLink<P> {
    fn default() -> Self {
        PanelLink {
            shown: Shared::default(),
        }
    }
}

impl<P: Clone> Clone for PanelLink<P> {
    fn clone(&self) -> Self {
        PanelLink {
            shown: self.shown.clone(),
        }
    }
}

impl<P: Clone + PartialEq> PanelLink<P> {
    /// A hidden panel.
    pub fn new() -> PanelLink<P> {
        PanelLink::default()
    }

    /// The current value.
    pub fn now(&self) -> Reveal<Opening<P>> {
        self.shown.now()
    }

    /// Open on `output` ([`Reveal::open`]).
    pub fn open(&self, output: OutputId, payload: P) {
        self.shown.update(|shown| shown.open(output, payload));
    }

    /// Open on `output` as a new opening, even when open ([`Reveal::reopen`]).
    pub fn reopen(&self, output: OutputId, payload: P) {
        self.shown.update(|shown| shown.reopen(output, payload));
    }

    /// Close when open, else open on `output` ([`Reveal::toggle`]).
    pub fn toggle(&self, output: OutputId, payload: P) {
        self.shown.update(|shown| shown.toggle(output, payload));
    }

    /// Close: Escape, a press away from the panel, a verb, a chosen action.
    pub fn close(&self) {
        self.shown.update(Reveal::hidden);
    }

    /// The panel's keyboard focus changed ([`Reveal::after_focus`]): losing it closes the panel.
    pub fn focus(&self, focus: KeyboardFocus) {
        self.shown.update(|shown| shown.after_focus(focus));
    }

    /// Whatever else a panel does to its own value; followers are told when it changed.
    pub fn update(&self, next: impl FnOnce(&Reveal<Opening<P>>) -> Reveal<Opening<P>>) {
        self.shown.update(next);
    }

    /// A receiver for a surface to follow.
    pub fn subscribe(&self) -> tokio::sync::watch::Receiver<Reveal<Opening<P>>> {
        self.shown.subscribe()
    }
}

#[cfg(test)]
mod tests {
    use shell_host::{KeyboardFocus, OutputId, Visibility};

    use super::{OpenSerial, Opening, PanelFocus, Reveal};

    const ONE: OutputId = OutputId(1);
    const TWO: OutputId = OutputId(2);

    type Shown = Reveal<Opening<&'static str>>;

    #[test]
    fn every_opening_counts_on_through_a_close_and_over_an_open_panel() {
        let first = Shown::default().toggle(ONE, "a");
        assert_eq!(first.serial(), OpenSerial(1));
        assert_eq!(first.opening().map(|o| o.focus), Some(PanelFocus::Awaited));
        let closed = first.toggle(ONE, "a");
        assert_eq!(
            closed,
            Reveal::Hidden {
                last: OpenSerial(1)
            }
        );
        assert_eq!(closed.toggle(ONE, "a").serial(), OpenSerial(2));
        assert_eq!(
            first.reopen(ONE, "a").serial(),
            OpenSerial(2),
            "even over itself"
        );
    }

    #[test]
    fn opening_the_same_way_again_is_no_new_opening() {
        let shown = Shown::default().open(ONE, "a");
        assert_eq!(shown.open(ONE, "a"), shown, "same output and payload");
        assert_eq!(shown.open(TWO, "a").serial(), OpenSerial(2), "moved");
        assert_eq!(
            shown.open(ONE, "b").serial(),
            OpenSerial(2),
            "another payload"
        );
        assert_eq!(shown.shows_on(ONE), Visibility::Shown);
        assert_eq!(shown.shows_on(TWO), Visibility::Hidden);
        assert_eq!(shown.hidden().shows_on(ONE), Visibility::Hidden);
        assert!(shown.open_on(ONE) && !shown.open_on(TWO));
    }

    #[test]
    fn a_focus_loss_closes_only_a_panel_that_held_focus() {
        let shown = Shown::default().open(ONE, "a");
        // The late leave from the previous opening's unmap: ignored.
        assert_eq!(shown.after_focus(KeyboardFocus::Lost), shown);
        let held = shown.after_focus(KeyboardFocus::Gained);
        assert_eq!(held.opening().map(|o| o.focus), Some(PanelFocus::Held));
        assert_eq!(held.after_focus(KeyboardFocus::Lost), shown.hidden());
        let hidden = Shown::default();
        assert_eq!(hidden.after_focus(KeyboardFocus::Gained), hidden);
    }
}
