//! The Force Quit Applications panel's three channels. The daemon writes whether the panel is
//! open and on which output (`sill force-quit`, the keyboard shortcut) and which apps the desktop
//! lists, and every output's panel surface follows both; the surface that shows the panel writes
//! the app the person confirmed, and the daemon carries it out.

use shell_host::OutputId;
use sill_model::desktop::ForceQuitRow;
use tokio::sync::watch;

use super::Shared;

use crate::links::{OpenSerial, Opened, PanelFocus, Reveal};

/// One opening of the panel: where, and which opening it is (a new one starts afresh).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ForceQuitOpening {
    pub serial: OpenSerial,
    pub output: OutputId,
    /// Whether the surface has held keyboard focus since it opened.
    pub focus: PanelFocus,
}

impl Opened for ForceQuitOpening {
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
        ForceQuitOpening {
            focus: PanelFocus::Held,
            ..self
        }
    }
}

/// Whether the panel is open, remembering the last opening's serial so the next one counts on.
pub type ForceQuitShown = Reveal<ForceQuitOpening>;

/// `shown` open on `output`: opening an open panel where it is stays the same opening, anywhere
/// else is a new one.
pub fn open(shown: &ForceQuitShown, output: OutputId) -> ForceQuitShown {
    match shown.opening() {
        Some(opening) if opening.output == output => shown.clone(),
        Some(_) | None => Reveal::Revealed(ForceQuitOpening {
            serial: OpenSerial(shown.serial().0 + 1),
            output,
            focus: PanelFocus::Awaited,
        }),
    }
}

/// Open on `output` when closed, close when open: the shortcut.
pub fn toggle(shown: &ForceQuitShown, output: OutputId) -> ForceQuitShown {
    match shown {
        Reveal::Revealed(_) => shown.hidden(),
        Reveal::Hidden { .. } => open(shown, output),
    }
}

/// An app the person confirmed force quitting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForceQuitAsked {
    /// Counts up from 1 per link.
    pub seq: u64,
    /// The app's key, as `DesktopCommand::ForceQuitApp` quotes it.
    pub app: String,
}

/// The shared channels, a root context of every panel surface and kept by the daemon. Clone is
/// cheap.
#[derive(Debug, Clone, Default)]
pub struct ForceQuitLink {
    panel: Shared<ForceQuitShown>,
    rows: Shared<Vec<ForceQuitRow>>,
    asked: Shared<Option<ForceQuitAsked>>,
}

impl ForceQuitLink {
    /// Closed, no apps listed, nothing asked.
    pub fn new() -> ForceQuitLink {
        ForceQuitLink::default()
    }

    /// The current value.
    pub fn now(&self) -> ForceQuitShown {
        self.panel.now()
    }

    /// Open on `output`.
    pub fn open(&self, output: OutputId) {
        self.panel.update(|panel| open(panel, output));
    }

    /// Open on `output` when closed, close when open.
    pub fn toggle(&self, output: OutputId) {
        self.panel.update(|panel| toggle(panel, output));
    }

    /// Close: Escape, Done, a press away from the panel, or `sill force-quit close`.
    pub fn close(&self) {
        self.panel.update(Reveal::hidden);
    }

    /// A receiver for a surface to follow.
    pub fn subscribe(&self) -> watch::Receiver<ForceQuitShown> {
        self.panel.subscribe()
    }

    /// The apps the desktop lists now.
    pub fn set_rows(&self, rows: Vec<ForceQuitRow>) {
        self.rows.set(rows);
    }

    /// A receiver for the apps the desktop lists.
    pub fn subscribe_rows(&self) -> watch::Receiver<Vec<ForceQuitRow>> {
        self.rows.subscribe()
    }

    /// The person confirmed force quitting `app`.
    pub fn ask(&self, app: impl Into<String>) {
        let app = app.into();
        self.asked.modify(|last| {
            let seq = last.as_ref().map_or(1, |last| last.seq + 1);
            *last = Some(ForceQuitAsked { seq, app });
        });
    }

    /// The latest confirmation.
    pub fn last_asked(&self) -> Option<ForceQuitAsked> {
        self.asked.now()
    }

    /// A receiver for the daemon to follow.
    pub fn subscribe_asked(&self) -> watch::Receiver<Option<ForceQuitAsked>> {
        self.asked.subscribe()
    }
}

#[cfg(test)]
mod tests {
    use shell_host::OutputId;

    use super::{ForceQuitLink, ForceQuitShown, open, toggle};
    use crate::links::{OpenSerial, Reveal};

    const ONE: OutputId = OutputId(1);
    const TWO: OutputId = OutputId(2);

    #[test]
    fn the_shortcut_opens_and_closes_and_each_opening_counts_on() {
        let closed = ForceQuitShown::default();
        let opened = toggle(&closed, ONE);
        assert_eq!(opened.serial(), OpenSerial(1));
        assert_eq!(opened.opening().map(|o| o.output), Some(ONE));
        let shut = toggle(&opened, ONE);
        assert_eq!(
            shut,
            Reveal::Hidden {
                last: OpenSerial(1)
            }
        );
        assert_eq!(toggle(&shut, TWO).serial(), OpenSerial(2));
    }

    #[test]
    fn opening_where_it_is_open_is_no_new_opening_but_elsewhere_is() {
        let opened = open(&ForceQuitShown::default(), ONE);
        assert_eq!(open(&opened, ONE), opened);
        assert_eq!(open(&opened, TWO).serial(), OpenSerial(2));
        assert_eq!(open(&opened, TWO).opening().map(|o| o.output), Some(TWO));
    }

    #[test]
    fn each_confirmation_counts_on() {
        let link = ForceQuitLink::new();
        assert_eq!(link.last_asked(), None);
        link.ask("term");
        link.ask("term");
        let last = link.last_asked().expect("asked");
        assert_eq!((last.seq, last.app.as_str()), (2, "term"));
    }
}
