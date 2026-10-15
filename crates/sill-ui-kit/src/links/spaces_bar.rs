//! Mission Control's Spaces bar, two channels: the daemon writes whether the bar is up (from the
//! compositor's overview state) and the desktop's view, and every output's bar surface follows
//! them; the surface writes what the person clicked, and the daemon carries it out.

use sill_model::desktop::{BarPick, BarShow, DesktopView};
use tokio::sync::watch;

/// What the bar surfaces draw from.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SpacesBarFeed {
    /// Whether the bar is up.
    pub show: BarShow,
    /// The desktop, for its Spaces and their windows.
    pub view: DesktopView,
}

/// A click in the bar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpacesBarPicked {
    /// Counts up from 1 per link.
    pub seq: u64,
    /// What was clicked.
    pub pick: BarPick,
}

/// The shared channels, a root context of every bar surface and kept by the daemon. Clone is
/// cheap.
#[derive(Debug, Clone)]
pub struct SpacesBarLink {
    feed: watch::Sender<SpacesBarFeed>,
    picked: watch::Sender<Option<SpacesBarPicked>>,
}

impl Default for SpacesBarLink {
    fn default() -> Self {
        SpacesBarLink::new()
    }
}

impl SpacesBarLink {
    /// Down, with no Spaces and no click.
    pub fn new() -> SpacesBarLink {
        SpacesBarLink {
            feed: watch::Sender::new(SpacesBarFeed::default()),
            picked: watch::Sender::new(None),
        }
    }

    /// The bar goes up or down.
    pub fn set_show(&self, show: BarShow) {
        self.feed.send_if_modified(|feed| {
            let changed = feed.show != show;
            feed.show = show;
            changed
        });
    }

    /// The desktop changed.
    pub fn set_view(&self, view: &DesktopView) {
        self.feed.send_if_modified(|feed| {
            let changed = feed.view != *view;
            if changed {
                feed.view = view.clone();
            }
            changed
        });
    }

    /// What the surfaces draw now.
    pub fn latest(&self) -> SpacesBarFeed {
        self.feed.borrow().clone()
    }

    /// A receiver for the surfaces to follow.
    pub fn subscribe(&self) -> watch::Receiver<SpacesBarFeed> {
        self.feed.subscribe()
    }

    /// The person clicked in the bar.
    pub fn pick(&self, pick: BarPick) {
        self.picked.send_modify(|last| {
            let seq = last.as_ref().map_or(1, |last| last.seq + 1);
            *last = Some(SpacesBarPicked { seq, pick });
        });
    }

    /// The latest click.
    pub fn last_pick(&self) -> Option<SpacesBarPicked> {
        self.picked.borrow().clone()
    }

    /// A receiver for the daemon to follow.
    pub fn subscribe_picks(&self) -> watch::Receiver<Option<SpacesBarPicked>> {
        self.picked.subscribe()
    }
}

#[cfg(test)]
mod tests {
    use sill_model::desktop::{BarPick, BarShow, DesktopView};

    use super::SpacesBarLink;

    #[test]
    fn the_bar_follows_the_show_and_the_view_and_a_repeat_wakes_no_one() {
        let link = SpacesBarLink::new();
        let mut follower = link.subscribe();
        assert_eq!(link.latest().show, BarShow::Hide);
        link.set_show(BarShow::Show);
        assert_eq!(follower.has_changed().ok(), Some(true));
        let _seen = follower.borrow_and_update();
        link.set_show(BarShow::Show);
        link.set_view(&DesktopView::default());
        assert_eq!(follower.has_changed().ok(), Some(false));
        assert_eq!(link.latest().show, BarShow::Show);
    }

    #[test]
    fn each_click_counts_on() {
        let link = SpacesBarLink::new();
        assert_eq!(link.last_pick(), None);
        link.pick(BarPick::Add("DP-1".to_owned()));
        link.pick(BarPick::Add("DP-2".to_owned()));
        let last = link.last_pick().expect("picked");
        assert_eq!((last.seq, last.pick), (2, BarPick::Add("DP-2".to_owned())));
    }
}
