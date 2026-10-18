//! Firings, from every corner surface to the daemon: one shared watch of the latest hit, each
//! hit numbered so two firings of one corner are two changes.

use shell_host::OutputId;
pub use sill_model::hot_corners::Corner;
use tokio::sync::watch;

use super::Shared;

/// One firing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CornerHit {
    pub corner: Corner,
    pub output: OutputId,
    /// Counts up from 1 per link.
    pub seq: u64,
}

/// The surfaces' half and the daemon's receiver. Clone is cheap.
#[derive(Debug, Clone, Default)]
pub struct CornerLink {
    hits: Shared<Option<CornerHit>>,
}

impl CornerLink {
    /// No hit yet.
    pub fn new() -> CornerLink {
        CornerLink::default()
    }

    /// `corner` of `output` fired (its surface's machine said `Fire`, or `sill debug corner`).
    pub fn hit(&self, corner: Corner, output: OutputId) {
        self.hits.modify(|last| {
            let seq = last.map_or(1, |hit| hit.seq + 1);
            *last = Some(CornerHit {
                corner,
                output,
                seq,
            });
        });
    }

    /// The latest hit.
    pub fn last(&self) -> Option<CornerHit> {
        self.hits.now()
    }

    /// A receiver for the daemon to follow.
    pub fn subscribe(&self) -> watch::Receiver<Option<CornerHit>> {
        self.hits.subscribe()
    }
}

#[cfg(test)]
mod tests {
    use shell_host::OutputId;

    use crate::links::{Corner, CornerHit, CornerLink};

    #[test]
    fn every_hit_is_news_even_the_same_corner_twice() {
        let link = CornerLink::new();
        let mut rx = link.subscribe();
        assert_eq!(link.last(), None);
        link.hit(Corner::TopLeft, OutputId(1));
        assert!(rx.has_changed().unwrap_or(false));
        rx.borrow_and_update();
        link.hit(Corner::TopLeft, OutputId(1));
        assert!(rx.has_changed().unwrap_or(false));
        assert_eq!(
            *rx.borrow_and_update(),
            Some(CornerHit {
                corner: Corner::TopLeft,
                output: OutputId(1),
                seq: 2,
            })
        );
    }
}
