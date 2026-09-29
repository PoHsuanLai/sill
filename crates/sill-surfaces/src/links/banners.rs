//! Which output shows the banners. macOS shows notifications on the main display only; sill's
//! stand-in is the daemon's first output (the one `sill launcher` and `sill power-menu` open on),
//! written here by the daemon whenever the outputs change. Every other output's banner surface
//! stays hidden. One output, so a banner is counted, paused and dismissed once.

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

/// The daemon's half and every banner surface's receiver. Clone is cheap.
#[derive(Debug, Clone)]
pub struct BannerLink {
    output: watch::Sender<BannerOutput>,
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
}

#[cfg(test)]
mod tests {
    use crate::links::{BannerLink, BannerOutput};
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
}
