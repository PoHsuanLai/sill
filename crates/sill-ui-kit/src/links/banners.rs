//! Which output shows the banners. macOS shows notifications on the main display only; sill's
//! stand-in is the daemon's first output (the one `sill launcher` and `sill power-menu` open on),
//! written here by the daemon whenever the outputs change. Every other output's banner surface
//! stays hidden. One output, so a banner is counted, paused and dismissed once.
//!
//! The link is also the transients' shared keyboard channel: each transient surface (the
//! screenshot thumbnail, the banner column) says here when a card appears and when none is left,
//! `sill focus-latest` asks for the keyboard and the link names the one that appeared last, and
//! only that surface answers. The compositor's focus loss on a surface comes back the same way,
//! naming the surface.

use shell_host::OutputId;
use tokio::sync::watch;

use super::Shared;

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

/// A kind of transient surface: it appears by itself and leaves by itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Transient {
    Thumbnail,
    Banner,
}

/// The two things the keyboard of a transient can be told.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum KeyWhat {
    /// Nothing yet.
    #[default]
    Idle,
    /// The person asked for the keyboard (`sill focus-latest`): this transient appeared last.
    Asked(Transient),
    /// The compositor took the keyboard away from this transient's surface.
    Lost(Transient),
}

/// The daemon's half and every banner surface's receiver. Clone is cheap.
#[derive(Debug, Clone, Default)]
pub struct BannerLink {
    output: Shared<BannerOutput>,
    keys: Shared<KeyEvent>,
    /// The transients that have a card up, the one that appeared last at the end.
    showing: Shared<Vec<Transient>>,
}

impl BannerLink {
    /// Nowhere yet.
    pub fn new() -> BannerLink {
        BannerLink::default()
    }

    /// Show the banners on `output` (a repeat of the current one notifies nobody).
    pub fn set(&self, output: BannerOutput) {
        self.output.set(output);
    }

    /// Where they show now.
    pub fn now(&self) -> BannerOutput {
        self.output.now()
    }

    /// A receiver for a surface to follow.
    pub fn subscribe(&self) -> watch::Receiver<BannerOutput> {
        self.output.subscribe()
    }

    /// A card of `transient` has appeared: it is now the newest.
    pub fn appeared(&self, transient: Transient) {
        self.showing.modify(|showing| {
            showing.retain(|t| *t != transient);
            showing.push(transient);
        });
    }

    /// `transient` has no card up any more.
    pub fn gone(&self, transient: Transient) {
        self.showing
            .modify(|showing| showing.retain(|t| *t != transient));
    }

    /// Ask for the keyboard on the transient that appeared last (`sill focus-latest`); nothing
    /// when none is up.
    pub fn ask_keyboard(&self) {
        let newest = self.showing.now().last().copied();
        if let Some(transient) = newest {
            self.tell(KeyWhat::Asked(transient));
        }
    }

    /// The compositor took the keyboard from `transient`'s surface.
    pub fn keyboard_lost(&self, transient: Transient) {
        self.tell(KeyWhat::Lost(transient));
    }

    fn tell(&self, what: KeyWhat) {
        self.keys.modify(|now| {
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
    use crate::links::{BannerLink, BannerOutput, KeyWhat, Transient};
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
        link.appeared(Transient::Banner);
        let mut rx = link.subscribe_keys();
        link.ask_keyboard();
        let first = *rx.borrow_and_update();
        link.ask_keyboard();
        assert!(rx.has_changed().unwrap_or(false));
        let second = *rx.borrow_and_update();
        link.keyboard_lost(Transient::Banner);
        let third = *rx.borrow_and_update();
        assert_eq!(
            (first.what, second.serial - first.serial, third.what),
            (
                KeyWhat::Asked(Transient::Banner),
                1,
                KeyWhat::Lost(Transient::Banner)
            )
        );
    }

    #[test]
    fn the_ask_names_the_transient_that_appeared_last_and_is_still_up() {
        use Transient::{Banner, Thumbnail};
        let asked = |link: &BannerLink| {
            let mut rx = link.subscribe_keys();
            link.ask_keyboard();
            rx.has_changed()
                .unwrap_or(false)
                .then(|| rx.borrow_and_update().what)
        };
        let link = BannerLink::new();
        assert_eq!(asked(&link), None, "nothing up: nobody is asked");
        link.appeared(Banner);
        link.appeared(Thumbnail);
        assert_eq!(asked(&link), Some(KeyWhat::Asked(Thumbnail)));
        link.appeared(Banner);
        assert_eq!(
            asked(&link),
            Some(KeyWhat::Asked(Banner)),
            "banner is newer"
        );
        link.appeared(Banner);
        link.appeared(Thumbnail);
        link.gone(Thumbnail);
        assert_eq!(
            asked(&link),
            Some(KeyWhat::Asked(Banner)),
            "the thumbnail left"
        );
        link.gone(Banner);
        assert_eq!(asked(&link), None);
    }
}
