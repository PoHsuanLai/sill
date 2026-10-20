//! Whether the launcher is showing, shared by its two surfaces and the daemon (two surfaces kept
//! warm, shown and hidden with `set_visibility`).
//!
//! One `watch` channel holds a [`Reveal<LauncherOpening>`]. The daemon writes it for `sill launcher
//! toggle|open|close` and for the panel's `Focus::Lost`; the panel writes it for Escape and an
//! activation, the catcher for a click. Each surface mirrors it onto its own visibility, so no
//! surface needs another's id. The transitions are pure functions of the value.

mod debug;
mod opening;
mod panel_hit;

use shell_host::{KeyboardFocus, OutputId};
use sill_launcher::LauncherMode;
use tokio::sync::watch;

use crate::links::{PanelLink, Reveal, Shared};

pub use debug::{CLIPBOARD_TITLE, ShownRows, ShownRowsSection};
pub use opening::{LauncherAsk, LauncherOpening};
pub use panel_hit::PanelHit;

/// `shown` with its mode changed in place: the same opening (the serial stays, so the panel does
/// not remount), a hidden launcher unchanged.
fn switched(shown: &Reveal<LauncherOpening>, mode: LauncherMode) -> Reveal<LauncherOpening> {
    match shown {
        Reveal::Revealed(opening) => Reveal::Revealed(LauncherOpening {
            payload: LauncherAsk {
                mode,
                ..opening.payload.clone()
            },
            ..opening.clone()
        }),
        Reveal::Hidden { .. } => shown.clone(),
    }
}

/// What a request opens the launcher with, arriving now.
fn ask(query: String, mode: LauncherMode) -> LauncherAsk {
    LauncherAsk {
        query,
        mode,
        requested: ds::base::time::clock::now(),
    }
}

/// The shared channels, given to both launcher surfaces as a root context and kept by the
/// daemon: whether the launcher shows, and which part of the panel takes presses. Clone is
/// cheap.
#[derive(Debug, Clone, Default)]
pub struct LauncherLink {
    panel: PanelLink<LauncherAsk>,
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
    pub fn now(&self) -> Reveal<LauncherOpening> {
        self.panel.now()
    }

    /// `launcher toggle` on `output`: opens with an empty query when hidden, hides when shown.
    pub fn toggle(&self, output: OutputId) {
        self.panel
            .toggle(output, ask(String::new(), LauncherMode::Search));
    }

    /// `launcher open --query` on `output`.
    pub fn open(&self, output: OutputId, query: String) {
        self.open_in(output, query, LauncherMode::Search);
    }

    /// Open on `output` with `query`, in `mode` (the companion opens its prompt this way). Every
    /// opening is a new one, even over an open panel.
    pub fn open_in(&self, output: OutputId, query: String, mode: LauncherMode) {
        self.panel.reopen(output, ask(query, mode));
    }

    /// Change the open panel's mode in place; nothing when it is hidden.
    pub fn switch_mode(&self, mode: LauncherMode) {
        self.panel.update(|shown| switched(shown, mode));
    }

    /// `launcher close`, Escape, an activation or a catcher click.
    pub fn close(&self) {
        self.panel.close();
    }

    /// The panel's keyboard focus changed.
    pub fn focus(&self, focus: KeyboardFocus) {
        self.panel.focus(focus);
    }

    /// A receiver for a surface to follow.
    pub fn subscribe(&self) -> watch::Receiver<Reveal<LauncherOpening>> {
        self.panel.subscribe()
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
    use super::{ask, switched};
    use crate::links::{LauncherLink, LauncherOpening, Reveal};
    use shell_host::{KeyboardFocus, OutputId};
    use sill_launcher::LauncherMode;

    const ONE: OutputId = OutputId(1);

    #[test]
    fn toggle_opens_fresh_and_a_request_over_an_open_panel_is_a_new_opening() {
        let link = LauncherLink::new();
        link.toggle(ONE);
        let first = link.now();
        assert_eq!(first.opening().map(|o| o.query.as_str()), Some(""));
        link.open_in(ONE, "fi".to_owned(), LauncherMode::Search);
        let typed = link.now();
        assert_eq!(typed.serial().0, first.serial().0 + 1);
        assert_eq!(typed.opening().map(|o| o.query.as_str()), Some("fi"));
    }

    #[test]
    fn switching_the_mode_keeps_the_opening_and_its_serial() {
        let shown = Reveal::<LauncherOpening>::default()
            .reopen(ONE, ask("fi".to_owned(), LauncherMode::Search));
        let switched_to = switched(&shown, LauncherMode::Companion);
        let (before, after) = (shown.opening().cloned(), switched_to.opening().cloned());
        assert_eq!(
            after.as_ref().map(|o| o.mode),
            Some(LauncherMode::Companion)
        );
        assert_eq!(
            after.map(|o| (o.serial, o.query.clone(), o.output)),
            before.map(|o| (o.serial, o.query.clone(), o.output)),
            "only the mode changed"
        );
        let hidden = shown.hidden();
        assert_eq!(switched(&hidden, LauncherMode::Companion), hidden);
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
