//! The Force Quit Applications panel's three channels. The daemon writes whether the panel is
//! open and on which output (`sill force-quit`, the keyboard shortcut) and which apps the desktop
//! lists, and every output's panel surface follows both; the surface that shows the panel writes
//! the app the person confirmed, and the daemon carries it out.

use shell_host::OutputId;
use sill_model::desktop::ForceQuitRow;
use tokio::sync::watch;

/// Counts openings; each is one more than the last.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct ForceQuitSerial(pub u64);

/// One opening of the panel: where, and which opening it is (a new one starts afresh).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ForceQuitOpening {
    pub serial: ForceQuitSerial,
    pub output: OutputId,
}

/// Whether the panel is open, remembering the last opening's serial so the next one counts on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForceQuitShown {
    Closed { last: ForceQuitSerial },
    Open(ForceQuitOpening),
}

impl Default for ForceQuitShown {
    fn default() -> Self {
        ForceQuitShown::Closed {
            last: ForceQuitSerial::default(),
        }
    }
}

impl ForceQuitShown {
    /// The opening, while open.
    pub fn opening(&self) -> Option<&ForceQuitOpening> {
        match self {
            ForceQuitShown::Open(opening) => Some(opening),
            ForceQuitShown::Closed { .. } => None,
        }
    }

    /// The current or last opening's serial.
    pub fn serial(&self) -> ForceQuitSerial {
        match self {
            ForceQuitShown::Open(opening) => opening.serial,
            ForceQuitShown::Closed { last } => *last,
        }
    }

    /// Closed, keeping the serial.
    pub fn closed(&self) -> ForceQuitShown {
        ForceQuitShown::Closed {
            last: self.serial(),
        }
    }

    /// Open on `output`: opening an open panel where it is stays the same opening, anywhere else
    /// is a new one.
    pub fn opened(&self, output: OutputId) -> ForceQuitShown {
        match self.opening() {
            Some(opening) if opening.output == output => *self,
            Some(_) | None => ForceQuitShown::Open(ForceQuitOpening {
                serial: ForceQuitSerial(self.serial().0 + 1),
                output,
            }),
        }
    }

    /// Open on `output` when closed, close when open: the shortcut.
    pub fn toggled(&self, output: OutputId) -> ForceQuitShown {
        match self {
            ForceQuitShown::Open(_) => self.closed(),
            ForceQuitShown::Closed { .. } => self.opened(output),
        }
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
#[derive(Debug, Clone)]
pub struct ForceQuitLink {
    panel: watch::Sender<ForceQuitShown>,
    rows: watch::Sender<Vec<ForceQuitRow>>,
    asked: watch::Sender<Option<ForceQuitAsked>>,
}

impl Default for ForceQuitLink {
    fn default() -> Self {
        ForceQuitLink::new()
    }
}

impl ForceQuitLink {
    /// Closed, no apps listed, nothing asked.
    pub fn new() -> ForceQuitLink {
        ForceQuitLink {
            panel: watch::Sender::new(ForceQuitShown::default()),
            rows: watch::Sender::new(Vec::new()),
            asked: watch::Sender::new(None),
        }
    }

    /// The current value.
    pub fn now(&self) -> ForceQuitShown {
        *self.panel.borrow()
    }

    /// Open on `output`.
    pub fn open(&self, output: OutputId) {
        self.panel
            .send_if_modified(|panel| replace(panel, panel.opened(output)));
    }

    /// Open on `output` when closed, close when open.
    pub fn toggle(&self, output: OutputId) {
        self.panel
            .send_if_modified(|panel| replace(panel, panel.toggled(output)));
    }

    /// Close: Escape, Done, a press away from the panel, or `sill force-quit close`.
    pub fn close(&self) {
        self.panel
            .send_if_modified(|panel| replace(panel, panel.closed()));
    }

    /// A receiver for a surface to follow.
    pub fn subscribe(&self) -> watch::Receiver<ForceQuitShown> {
        self.panel.subscribe()
    }

    /// The apps the desktop lists now.
    pub fn set_rows(&self, rows: Vec<ForceQuitRow>) {
        self.rows.send_if_modified(|shown| {
            let changed = *shown != rows;
            if changed {
                *shown = rows;
            }
            changed
        });
    }

    /// A receiver for the apps the desktop lists.
    pub fn subscribe_rows(&self) -> watch::Receiver<Vec<ForceQuitRow>> {
        self.rows.subscribe()
    }

    /// The person confirmed force quitting `app`.
    pub fn ask(&self, app: impl Into<String>) {
        let app = app.into();
        self.asked.send_modify(|last| {
            let seq = last.as_ref().map_or(1, |last| last.seq + 1);
            *last = Some(ForceQuitAsked { seq, app });
        });
    }

    /// The latest confirmation.
    pub fn last_asked(&self) -> Option<ForceQuitAsked> {
        self.asked.borrow().clone()
    }

    /// A receiver for the daemon to follow.
    pub fn subscribe_asked(&self) -> watch::Receiver<Option<ForceQuitAsked>> {
        self.asked.subscribe()
    }
}

/// Store `next`, saying whether it differs.
fn replace(panel: &mut ForceQuitShown, next: ForceQuitShown) -> bool {
    let changed = *panel != next;
    *panel = next;
    changed
}

#[cfg(test)]
mod tests {
    use shell_host::OutputId;

    use super::{ForceQuitLink, ForceQuitSerial, ForceQuitShown};

    const ONE: OutputId = OutputId(1);
    const TWO: OutputId = OutputId(2);

    #[test]
    fn the_shortcut_opens_and_closes_and_each_opening_counts_on() {
        let closed = ForceQuitShown::default();
        let open = closed.toggled(ONE);
        assert_eq!(open.serial(), ForceQuitSerial(1));
        assert_eq!(open.opening().map(|o| o.output), Some(ONE));
        let shut = open.toggled(ONE);
        assert_eq!(
            shut,
            ForceQuitShown::Closed {
                last: ForceQuitSerial(1)
            }
        );
        assert_eq!(shut.toggled(TWO).serial(), ForceQuitSerial(2));
    }

    #[test]
    fn opening_where_it_is_open_is_no_new_opening_but_elsewhere_is() {
        let open = ForceQuitShown::default().opened(ONE);
        assert_eq!(open.opened(ONE), open);
        assert_eq!(open.opened(TWO).serial(), ForceQuitSerial(2));
        assert_eq!(open.opened(TWO).opening().map(|o| o.output), Some(TWO));
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
