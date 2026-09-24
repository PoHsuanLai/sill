//! Whether the launcher is showing, shared by its two surfaces and the daemon (plan "Launcher
//! v1": two surfaces kept warm, shown and hidden with `set_visibility`).
//!
//! One `watch` channel holds a [`LauncherShown`]. The daemon writes it for `sill launcher
//! toggle|open|close` and for the panel's `Focus::Lost`; the panel writes it for Escape and an
//! activation, the catcher for a click. Each surface mirrors it onto its own visibility, so no
//! surface needs another's id. The transitions are pure functions of the value.

use std::time::Instant;

use shell_host::{Focus, OutputId};
use tokio::sync::watch;

use crate::surfaces::launcher::hit::PanelHit;

/// The launcher's one piece of shared state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LauncherShown {
    /// Hidden, remembering the last opening's serial so the next one counts on.
    Hidden {
        last: OpenSerial,
    },
    Shown(Opening),
}

impl Default for LauncherShown {
    fn default() -> Self {
        LauncherShown::Hidden {
            last: OpenSerial::default(),
        }
    }
}

/// One opening of the launcher: where, with what already typed, and since when.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Opening {
    /// The output whose panel shows.
    pub output: OutputId,
    /// The query the field starts with (`sill launcher open --query`).
    pub query: String,
    /// Which opening this is: a new one remounts the panel's contents (the entrance plays,
    /// the field is focused and the selection starts at the top).
    pub serial: OpenSerial,
    /// When the request arrived, for `open_latency_ms`.
    pub requested: Instant,
    /// Whether the panel has had keyboard focus since it opened.
    pub focus: PanelFocus,
}

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

impl LauncherShown {
    /// The opening, while shown.
    pub fn opening(&self) -> Option<&Opening> {
        match self {
            LauncherShown::Shown(opening) => Some(opening),
            LauncherShown::Hidden { .. } => None,
        }
    }

    /// Whether the launcher shows on `output`.
    pub fn shows_on(&self, output: OutputId) -> Visible {
        match self.opening() {
            Some(opening) if opening.output == output => Visible::Yes,
            _ => Visible::No,
        }
    }

    /// The current or last opening's serial.
    pub fn serial(&self) -> OpenSerial {
        match self {
            LauncherShown::Shown(opening) => opening.serial,
            LauncherShown::Hidden { last } => *last,
        }
    }

    /// Hidden, keeping the serial.
    pub fn hidden(&self) -> LauncherShown {
        LauncherShown::Hidden {
            last: self.serial(),
        }
    }
}

/// Whether a surface should be mapped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Visible {
    Yes,
    No,
}

/// `shown` opened on `output` with `query`, as a new opening.
pub fn open(shown: &LauncherShown, output: OutputId, query: String, now: Instant) -> LauncherShown {
    LauncherShown::Shown(Opening {
        output,
        query,
        serial: OpenSerial(shown.serial().0 + 1),
        requested: now,
        focus: PanelFocus::Awaited,
    })
}

/// `launcher toggle`: open when hidden (with an empty query), hide when shown.
pub fn toggle(shown: &LauncherShown, output: OutputId, now: Instant) -> LauncherShown {
    match shown {
        LauncherShown::Hidden { .. } => open(shown, output, String::new(), now),
        LauncherShown::Shown(_) => shown.hidden(),
    }
}

/// The panel's keyboard focus changed: gaining it is noted, losing it after holding it closes
/// the launcher (design/06 §20.2 "focus lost").
pub fn after_focus(shown: &LauncherShown, focus: Focus) -> LauncherShown {
    match (shown, focus) {
        (LauncherShown::Shown(opening), Focus::Gained) => LauncherShown::Shown(Opening {
            focus: PanelFocus::Held,
            ..opening.clone()
        }),
        (LauncherShown::Shown(opening), Focus::Lost) if opening.focus == PanelFocus::Held => {
            shown.hidden()
        }
        _ => shown.clone(),
    }
}

/// The shared channels, given to both launcher surfaces as a root context and kept by the
/// daemon: whether the launcher shows, and which part of the panel takes presses. Clone is
/// cheap.
#[derive(Debug, Clone)]
pub struct LauncherLink {
    shown: watch::Sender<LauncherShown>,
    hit: watch::Sender<PanelHit>,
}

impl Default for LauncherLink {
    fn default() -> Self {
        LauncherLink::new()
    }
}

impl LauncherLink {
    /// A hidden launcher.
    pub fn new() -> LauncherLink {
        LauncherLink {
            shown: watch::Sender::new(LauncherShown::default()),
            hit: watch::Sender::new(PanelHit::default()),
        }
    }

    /// The current value.
    pub fn now(&self) -> LauncherShown {
        self.shown.borrow().clone()
    }

    /// `launcher toggle` on `output`.
    pub fn toggle(&self, output: OutputId) {
        let now = Instant::now();
        self.shown
            .send_modify(|shown| *shown = toggle(shown, output, now));
    }

    /// `launcher open --query` on `output`.
    pub fn open(&self, output: OutputId, query: String) {
        let now = Instant::now();
        self.shown
            .send_modify(|shown| *shown = open(shown, output, query, now));
    }

    /// `launcher close`, Escape, an activation or a catcher click.
    pub fn close(&self) {
        self.shown.send_if_modified(|shown| {
            let was = shown.opening().is_some();
            *shown = shown.hidden();
            was
        });
    }

    /// The panel's keyboard focus changed.
    pub fn focus(&self, focus: Focus) {
        self.shown.send_if_modified(|shown| {
            let next = after_focus(shown, focus);
            let changed = next != *shown;
            *shown = next;
            changed
        });
    }

    /// A receiver for a surface to follow.
    pub fn subscribe(&self) -> watch::Receiver<LauncherShown> {
        self.shown.subscribe()
    }

    /// The panel measured which part of it takes presses.
    pub fn set_hit(&self, hit: PanelHit) {
        self.hit.send_if_modified(|now| {
            let changed = *now != hit;
            *now = hit;
            changed
        });
    }

    /// What the panel takes now.
    pub fn hit(&self) -> PanelHit {
        *self.hit.borrow()
    }

    /// A receiver for the catcher to follow the panel's part.
    pub fn subscribe_hit(&self) -> watch::Receiver<PanelHit> {
        self.hit.subscribe()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        LauncherLink, LauncherShown, OpenSerial, PanelFocus, Visible, after_focus, open, toggle,
    };
    use shell_host::{Focus, OutputId};
    use std::time::Instant;

    const ONE: OutputId = OutputId(1);

    #[test]
    fn toggle_opens_fresh_and_closes() {
        let now = Instant::now();
        let first = toggle(&LauncherShown::default(), ONE, now);
        let opening = first
            .opening()
            .cloned()
            .expect("toggle opens a hidden launcher");
        assert_eq!(opening.query, "");
        assert_eq!(opening.serial, OpenSerial(1));
        assert_eq!(opening.focus, PanelFocus::Awaited);
        let closed = toggle(&first, ONE, now);
        assert_eq!(
            closed,
            LauncherShown::Hidden {
                last: OpenSerial(1)
            }
        );
        // Every opening counts on, through a close or over an open one.
        let again = toggle(&closed, ONE, now);
        assert_eq!(again.opening().map(|o| o.serial), Some(OpenSerial(2)));
        let typed = open(&again, ONE, "fi".to_owned(), now);
        assert_eq!(typed.opening().map(|o| o.serial), Some(OpenSerial(3)));
        assert_eq!(typed.opening().map(|o| o.query.as_str()), Some("fi"));
    }

    #[test]
    fn a_focus_loss_closes_only_a_panel_that_held_focus() {
        let now = Instant::now();
        let shown = open(&LauncherShown::default(), ONE, String::new(), now);
        // The late leave from the previous opening's unmap: ignored.
        assert_eq!(after_focus(&shown, Focus::Lost), shown);
        let held = after_focus(&shown, Focus::Gained);
        assert_eq!(held.opening().map(|o| o.focus), Some(PanelFocus::Held));
        assert_eq!(after_focus(&held, Focus::Lost), shown.hidden());
        let hidden = LauncherShown::default();
        assert_eq!(after_focus(&hidden, Focus::Gained), hidden);
    }

    #[test]
    fn the_launcher_shows_on_its_output_only() {
        let shown = open(
            &LauncherShown::default(),
            ONE,
            String::new(),
            Instant::now(),
        );
        assert_eq!(shown.shows_on(ONE), Visible::Yes);
        assert_eq!(shown.shows_on(OutputId(2)), Visible::No);
        assert_eq!(shown.hidden().shows_on(ONE), Visible::No);
    }

    #[test]
    fn the_link_notifies_only_on_change() {
        let link = LauncherLink::new();
        let mut rx = link.subscribe();
        link.close();
        assert!(
            !rx.has_changed().unwrap_or(true),
            "closing a closed launcher"
        );
        link.toggle(ONE);
        assert!(rx.has_changed().unwrap_or(false));
        let _ = rx.borrow_and_update();
        link.focus(Focus::Lost);
        assert!(!rx.has_changed().unwrap_or(true), "a loss before any gain");
        link.close();
        assert_eq!(rx.borrow_and_update().opening(), None);
    }
}
