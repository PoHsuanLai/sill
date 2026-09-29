//! Whether the launcher is showing, shared by its two surfaces and the daemon (two surfaces kept
//! warm, shown and hidden with `set_visibility`).
//!
//! One `watch` channel holds a [`Reveal<Opening>`]. The daemon writes it for `sill launcher
//! toggle|open|close` and for the panel's `Focus::Lost`; the panel writes it for Escape and an
//! activation, the catcher for a click. Each surface mirrors it onto its own visibility, so no
//! surface needs another's id. The transitions are pure functions of the value.

mod debug;
mod opening;
mod panel_hit;

use std::time::Instant;

use shell_host::{KeyboardFocus, OutputId};
use tokio::sync::watch;

use crate::links::{OpenSerial, PanelFocus, Reveal};

pub use debug::{CLIPBOARD_TITLE, LinkKey, LinkPress, ShownRows, ShownRowsSection};
pub use opening::Opening;
pub use panel_hit::PanelHit;

/// `shown` opened on `output` with `query`, as a new opening.
fn open(shown: &Reveal<Opening>, output: OutputId, query: String, now: Instant) -> Reveal<Opening> {
    Reveal::Revealed(Opening {
        output,
        query,
        serial: OpenSerial(shown.serial().0 + 1),
        requested: now,
        focus: PanelFocus::Awaited,
    })
}

/// `launcher toggle`: open when hidden (with an empty query), hide when shown.
fn toggle(shown: &Reveal<Opening>, output: OutputId, now: Instant) -> Reveal<Opening> {
    match shown {
        Reveal::Hidden { .. } => open(shown, output, String::new(), now),
        Reveal::Revealed(_) => shown.hidden(),
    }
}

/// The shared channels, given to both launcher surfaces as a root context and kept by the
/// daemon: whether the launcher shows, and which part of the panel takes presses. Clone is
/// cheap.
#[derive(Debug, Clone)]
pub struct LauncherLink {
    shown: watch::Sender<Reveal<Opening>>,
    hit: watch::Sender<PanelHit>,
    /// `sill debug launcher-key`'s presses, which the open panel steps.
    keys: watch::Sender<LinkPress>,
    /// The rows the open panel shows, for `sill debug launcher-rows`.
    rows: watch::Sender<Option<ShownRows>>,
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
            shown: watch::Sender::new(Reveal::default()),
            hit: watch::Sender::new(PanelHit::default()),
            keys: watch::Sender::new(LinkPress::default()),
            rows: watch::Sender::new(None),
        }
    }

    /// The current value.
    pub fn now(&self) -> Reveal<Opening> {
        self.shown.borrow().clone()
    }

    /// `launcher toggle` on `output`.
    pub fn toggle(&self, output: OutputId) {
        let now = ds::time::now();
        self.shown
            .send_modify(|shown| *shown = toggle(shown, output, now));
    }

    /// `launcher open --query` on `output`.
    pub fn open(&self, output: OutputId, query: String) {
        let now = ds::time::now();
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
    pub fn focus(&self, focus: KeyboardFocus) {
        self.shown.send_if_modified(|shown| {
            let next = shown.after_focus(focus);
            let changed = next != *shown;
            *shown = next;
            changed
        });
    }

    /// A receiver for a surface to follow.
    pub fn subscribe(&self) -> watch::Receiver<Reveal<Opening>> {
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

    /// Press `key` on the open panel, as if typed (`sill debug launcher-key`). A panel that is
    /// not open ignores it.
    pub fn press(&self, key: LinkKey) {
        self.keys.send_modify(|press| {
            press.count += 1;
            press.key = Some(key);
        });
    }

    /// A receiver for the panel to step every press from.
    pub fn subscribe_keys(&self) -> watch::Receiver<LinkPress> {
        self.keys.subscribe()
    }

    /// The panel shows `rows` (`None`: it is hidden).
    pub fn show_rows(&self, rows: Option<ShownRows>) {
        self.rows.send_if_modified(|now| {
            let changed = *now != rows;
            *now = rows;
            changed
        });
    }

    /// The rows the open panel shows, `None` while no panel is open.
    pub fn rows(&self) -> Option<ShownRows> {
        self.rows.borrow().clone()
    }
}

#[cfg(test)]
mod tests {
    use super::{open, toggle};
    use crate::links::{LauncherLink, OpenSerial, Opening, PanelFocus, Reveal};
    use shell_host::{KeyboardFocus, OutputId, Visibility};
    use std::time::Instant;

    const ONE: OutputId = OutputId(1);

    #[test]
    fn toggle_opens_fresh_and_closes() {
        let now = Instant::now();
        let first = toggle(&Reveal::default(), ONE, now);
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
            Reveal::Hidden {
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
        let shown = open(&Reveal::default(), ONE, String::new(), now);
        // The late leave from the previous opening's unmap: ignored.
        assert_eq!(shown.after_focus(KeyboardFocus::Lost), shown);
        let held = shown.after_focus(KeyboardFocus::Gained);
        assert_eq!(held.opening().map(|o| o.focus), Some(PanelFocus::Held));
        assert_eq!(held.after_focus(KeyboardFocus::Lost), shown.hidden());
        let hidden = Reveal::<Opening>::default();
        assert_eq!(hidden.after_focus(KeyboardFocus::Gained), hidden);
    }

    #[test]
    fn the_launcher_shows_on_its_output_only() {
        let shown = open(&Reveal::default(), ONE, String::new(), Instant::now());
        assert_eq!(shown.shows_on(ONE), Visibility::Shown);
        assert_eq!(shown.shows_on(OutputId(2)), Visibility::Hidden);
        assert_eq!(shown.hidden().shows_on(ONE), Visibility::Hidden);
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
        link.focus(KeyboardFocus::Lost);
        assert!(!rx.has_changed().unwrap_or(true), "a loss before any gain");
        link.close();
        assert_eq!(rx.borrow_and_update().opening(), None);
    }
}
