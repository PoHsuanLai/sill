//! Whether a panel is showing, and the opening it shows, shared by every surface that follows
//! one link and the daemon that writes it: `Reveal<O>` is a link's value. The transitions that
//! hold for every panel (the serial that counts openings, the hidden value that remembers it,
//! what a keyboard focus change does) live here; what an opening carries is the panel's own `O`.

use shell_host::{KeyboardFocus, OutputId, Visibility};

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
