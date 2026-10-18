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
use sill_launcher::LauncherMode;
use tokio::sync::watch;

use crate::links::{OpenSerial, PanelFocus, Reveal, Shared};

pub use debug::{CLIPBOARD_TITLE, ShownRows, ShownRowsSection};
pub use opening::Opening;
pub use panel_hit::PanelHit;

/// `shown` opened on `output` with `query` in `mode`, as a new opening.
fn open(
    shown: &Reveal<Opening>,
    output: OutputId,
    query: String,
    mode: LauncherMode,
    now: Instant,
) -> Reveal<Opening> {
    Reveal::Revealed(Opening {
        output,
        query,
        serial: OpenSerial(shown.serial().0 + 1),
        mode,
        requested: now,
        focus: PanelFocus::Awaited,
    })
}

/// `shown` with its mode changed in place: the same opening (the serial stays, so the panel does
/// not remount), a hidden launcher unchanged.
fn switched(shown: &Reveal<Opening>, mode: LauncherMode) -> Reveal<Opening> {
    match shown {
        Reveal::Revealed(opening) => Reveal::Revealed(Opening {
            mode,
            ..opening.clone()
        }),
        Reveal::Hidden { .. } => shown.clone(),
    }
}

/// `launcher toggle`: open when hidden (with an empty query), hide when shown.
fn toggle(shown: &Reveal<Opening>, output: OutputId, now: Instant) -> Reveal<Opening> {
    match shown {
        Reveal::Hidden { .. } => open(shown, output, String::new(), LauncherMode::Search, now),
        Reveal::Revealed(_) => shown.hidden(),
    }
}

/// The shared channels, given to both launcher surfaces as a root context and kept by the
/// daemon: whether the launcher shows, and which part of the panel takes presses. Clone is
/// cheap.
#[derive(Debug, Clone, Default)]
pub struct LauncherLink {
    shown: Shared<Reveal<Opening>>,
    hit: Shared<PanelHit>,
    /// The rows the open panel shows, for `sill debug launcher-rows`.
    rows: Shared<Option<ShownRows>>,
}

impl LauncherLink {
    /// A hidden launcher.
    pub fn new() -> LauncherLink {
        LauncherLink::default()
    }

    /// The current value.
    pub fn now(&self) -> Reveal<Opening> {
        self.shown.now()
    }

    /// `launcher toggle` on `output`.
    pub fn toggle(&self, output: OutputId) {
        let now = ds::base::time::clock::now();
        self.shown
            .modify(|shown| *shown = toggle(shown, output, now));
    }

    /// `launcher open --query` on `output`.
    pub fn open(&self, output: OutputId, query: String) {
        self.open_in(output, query, LauncherMode::Search);
    }

    /// Open on `output` with `query`, in `mode` (the companion opens its prompt this way).
    pub fn open_in(&self, output: OutputId, query: String, mode: LauncherMode) {
        let now = ds::base::time::clock::now();
        self.shown
            .modify(|shown| *shown = open(shown, output, query, mode, now));
    }

    /// Change the open panel's mode in place; nothing when it is hidden.
    pub fn switch_mode(&self, mode: LauncherMode) {
        self.shown.update(|shown| switched(shown, mode));
    }

    /// `launcher close`, Escape, an activation or a catcher click.
    pub fn close(&self) {
        self.shown.modify_if(|shown| {
            let was = shown.opening().is_some();
            *shown = shown.hidden();
            was
        });
    }

    /// The panel's keyboard focus changed.
    pub fn focus(&self, focus: KeyboardFocus) {
        self.shown.update(|shown| shown.after_focus(focus));
    }

    /// A receiver for a surface to follow.
    pub fn subscribe(&self) -> watch::Receiver<Reveal<Opening>> {
        self.shown.subscribe()
    }

    /// The panel measured which part of it takes presses.
    pub fn set_hit(&self, hit: PanelHit) {
        self.hit.set(hit);
    }

    /// What the panel takes now.
    pub fn hit(&self) -> PanelHit {
        self.hit.now()
    }

    /// A receiver for the catcher to follow the panel's part.
    pub fn subscribe_hit(&self) -> watch::Receiver<PanelHit> {
        self.hit.subscribe()
    }

    /// The panel shows `rows` (`None`: it is hidden).
    pub fn show_rows(&self, rows: Option<ShownRows>) {
        self.rows.set(rows);
    }

    /// The rows the open panel shows, `None` while no panel is open.
    pub fn rows(&self) -> Option<ShownRows> {
        self.rows.now()
    }
}

#[cfg(test)]
mod tests {
    use super::{open, switched, toggle};
    use crate::links::{LauncherLink, OpenSerial, Opening, PanelFocus, Reveal};
    use shell_host::{KeyboardFocus, OutputId, Visibility};
    use sill_launcher::LauncherMode;
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
        let typed = open(&again, ONE, "fi".to_owned(), LauncherMode::Search, now);
        assert_eq!(typed.opening().map(|o| o.serial), Some(OpenSerial(3)));
        assert_eq!(typed.opening().map(|o| o.query.as_str()), Some("fi"));
    }

    #[test]
    fn switching_the_mode_keeps_the_opening_and_its_serial() {
        let now = Instant::now();
        let shown = open(
            &Reveal::default(),
            ONE,
            "fi".to_owned(),
            LauncherMode::Search,
            now,
        );
        let switched = switched(&shown, LauncherMode::Companion);
        let (before, after) = (shown.opening().cloned(), switched.opening().cloned());
        assert_eq!(
            after.as_ref().map(|o| o.mode),
            Some(LauncherMode::Companion)
        );
        assert_eq!(
            after.map(|o| (o.serial, o.query, o.output)),
            before.map(|o| (o.serial, o.query, o.output)),
            "only the mode changed"
        );
        let hidden = shown.hidden();
        assert_eq!(super::switched(&hidden, LauncherMode::Companion), hidden);
    }

    #[test]
    fn a_focus_loss_closes_only_a_panel_that_held_focus() {
        let now = Instant::now();
        let shown = open(
            &Reveal::default(),
            ONE,
            String::new(),
            LauncherMode::Search,
            now,
        );
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
        let shown = open(
            &Reveal::default(),
            ONE,
            String::new(),
            LauncherMode::Search,
            Instant::now(),
        );
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
