//! The window menu's two channels: the daemon writes each opening the compositor asked for (the
//! pointer rested on a window's green light) and every output's window menu surface follows it;
//! the surface that shows the menu writes the entry the person picked, and the daemon carries it
//! out. A dismissal writes nothing. Each opening and each pick is numbered, so a second one for
//! the same window is a second change.

use sill_model::desktop::WindowEntry;
use tokio::sync::watch;

use super::Shared;

/// Counts openings; each is one more than the last.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct MenuSerial(pub u64);

/// One window menu to show.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowMenuOpening {
    /// Which opening it is.
    pub serial: MenuSerial,
    /// The window's identifier, as the pick quotes it.
    pub window: String,
    /// Where the menu's top left goes, in the desktop's logical pixels.
    pub at: (i32, i32),
    /// A bit per `WindowEntry` code, set when the entry can be picked.
    pub enabled: u32,
}

/// An entry the person picked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowMenuPicked {
    /// Counts up from 1 per link.
    pub seq: u64,
    /// The window the menu was for.
    pub window: String,
    /// The entry.
    pub entry: WindowEntry,
}

/// The shared channels, a root context of every window menu surface and kept by the daemon.
/// Clone is cheap.
#[derive(Debug, Clone, Default)]
pub struct WindowMenuLink {
    opening: Shared<Option<WindowMenuOpening>>,
    picked: Shared<Option<WindowMenuPicked>>,
}

impl WindowMenuLink {
    /// No menu asked for, nothing picked.
    pub fn new() -> WindowMenuLink {
        WindowMenuLink::default()
    }

    /// Ask for the menu of `window` at `at`, with the entries `enabled` marks pickable.
    pub fn open(&self, window: impl Into<String>, at: (i32, i32), enabled: u32) {
        let window = window.into();
        self.opening.modify(|latest| {
            let serial = MenuSerial(latest.as_ref().map_or(0, |last| last.serial.0) + 1);
            *latest = Some(WindowMenuOpening {
                serial,
                window,
                at,
                enabled,
            });
        });
    }

    /// The latest opening.
    pub fn latest(&self) -> Option<WindowMenuOpening> {
        self.opening.now()
    }

    /// A receiver for the surfaces to follow.
    pub fn subscribe(&self) -> watch::Receiver<Option<WindowMenuOpening>> {
        self.opening.subscribe()
    }

    /// The person picked `entry` in the menu of `window`.
    pub fn pick(&self, window: impl Into<String>, entry: WindowEntry) {
        let window = window.into();
        self.picked.modify(|last| {
            let seq = last.as_ref().map_or(1, |last| last.seq + 1);
            *last = Some(WindowMenuPicked { seq, window, entry });
        });
    }

    /// The latest pick.
    pub fn last_pick(&self) -> Option<WindowMenuPicked> {
        self.picked.now()
    }

    /// A receiver for the daemon to follow.
    pub fn subscribe_picks(&self) -> watch::Receiver<Option<WindowMenuPicked>> {
        self.picked.subscribe()
    }
}

#[cfg(test)]
mod tests {
    use super::{MenuSerial, WindowMenuLink};
    use sill_model::desktop::WindowEntry;

    #[test]
    fn each_opening_and_each_pick_counts_on() {
        let link = WindowMenuLink::new();
        assert_eq!(link.latest(), None);
        link.open("w1", (10, 20), 0b101);
        link.open("w1", (10, 20), 0b101);
        let latest = link.latest().expect("opened");
        assert_eq!(
            latest.serial,
            MenuSerial(2),
            "the same menu again is a new opening"
        );
        assert_eq!(
            (latest.window.as_str(), latest.at, latest.enabled),
            ("w1", (10, 20), 0b101)
        );
        assert_eq!(link.last_pick(), None);
        link.pick("w1", WindowEntry::Left);
        link.pick("w1", WindowEntry::Fill);
        let picked = link.last_pick().expect("picked");
        assert_eq!((picked.seq, picked.entry), (2, WindowEntry::Fill));
    }
}
