//! Which output shows the banners. macOS shows notifications on the main display only; sill's
//! stand-in is the daemon's first output (the one `sill launcher` and `sill power-menu` open on),
//! written here by the daemon whenever the outputs change. Every other output's banner surface
//! stays hidden. One output, so a banner is counted, paused and dismissed once.
//!
//! The link is also the transients' shared keyboard channel: `sill focus-latest` asks for the
//! keyboard here, and the surface that follows it (the screenshot thumbnail) answers when it is
//! showing. The compositor's focus loss on that surface comes back the same way.

use shell_host::OutputId;
use tokio::sync::watch;

/// The output the banners show on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum BannerOutput {
    /// No output yet.
    #[default]
    Nowhere,
    On(OutputId),
}

impl BannerOutput {
    /// Whether `output`'s banner surface is the one that shows.
    pub fn shows_on(self, output: OutputId) -> bool {
        self == BannerOutput::On(output)
    }
}

/// What happened to the keyboard of the newest transient, and which time it happened (two asks
/// in a row are two events).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct KeyEvent {
    pub serial: u64,
    pub what: KeyWhat,
}

/// The two things the keyboard of a transient can be told.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum KeyWhat {
    /// Nothing yet.
    #[default]
    Idle,
    /// The person asked for the keyboard on the newest transient (`sill focus-latest`).
    Asked,
    /// The compositor took the keyboard away from the surface that held it.
    Lost,
}

/// The daemon's half and every banner surface's receiver. Clone is cheap.
#[derive(Debug, Clone)]
pub struct BannerLink {
    output: watch::Sender<BannerOutput>,
    keys: watch::Sender<KeyEvent>,
}

impl Default for BannerLink {
    fn default() -> Self {
        BannerLink::new()
    }
}

impl BannerLink {
    /// Nowhere yet.
    pub fn new() -> BannerLink {
        BannerLink {
            output: watch::Sender::new(BannerOutput::Nowhere),
            keys: watch::Sender::new(KeyEvent::default()),
        }
    }

    /// Show the banners on `output` (a repeat of the current one notifies nobody).
    pub fn set(&self, output: BannerOutput) {
        self.output.send_if_modified(|now| {
            let changed = *now != output;
            *now = output;
            changed
        });
    }

    /// Where they show now.
    pub fn now(&self) -> BannerOutput {
        *self.output.borrow()
    }

    /// A receiver for a surface to follow.
    pub fn subscribe(&self) -> watch::Receiver<BannerOutput> {
        self.output.subscribe()
    }

    /// Ask for the keyboard on the newest transient (`sill focus-latest`).
    pub fn ask_keyboard(&self) {
        self.tell(KeyWhat::Asked);
    }

    /// The compositor took the keyboard from the surface that had it.
    pub fn keyboard_lost(&self) {
        self.tell(KeyWhat::Lost);
    }

    fn tell(&self, what: KeyWhat) {
        self.keys.send_modify(|now| {
            *now = KeyEvent {
                serial: now.serial + 1,
                what,
            };
        });
    }

    /// A receiver for the keyboard's asks.
    pub fn subscribe_keys(&self) -> watch::Receiver<KeyEvent> {
        self.keys.subscribe()
    }
}

#[cfg(test)]
mod tests {
    use crate::links::{BannerLink, BannerOutput, KeyWhat};
    use shell_host::OutputId;

    #[test]
    fn one_output_shows_and_a_repeat_is_not_news() {
        let link = BannerLink::new();
        let mut rx = link.subscribe();
        assert!(!link.now().shows_on(OutputId(1)));
        link.set(BannerOutput::On(OutputId(1)));
        assert!(rx.has_changed().unwrap_or(false));
        assert!(rx.borrow_and_update().shows_on(OutputId(1)));
        assert!(!link.now().shows_on(OutputId(2)));
        link.set(BannerOutput::On(OutputId(1)));
        assert!(!rx.has_changed().unwrap_or(true));
    }

    #[test]
    fn two_asks_in_a_row_are_two_events() {
        let link = BannerLink::new();
        let mut rx = link.subscribe_keys();
        link.ask_keyboard();
        let first = *rx.borrow_and_update();
        link.ask_keyboard();
        assert!(rx.has_changed().unwrap_or(false));
        let second = *rx.borrow_and_update();
        link.keyboard_lost();
        let third = *rx.borrow_and_update();
        assert_eq!(
            (first.what, second.serial - first.serial, third.what),
            (KeyWhat::Asked, 1, KeyWhat::Lost)
        );
    }
}
