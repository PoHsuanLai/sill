//! Mission Control's Spaces bar, two channels and the desktop. The feed: the daemon writes whether
//! the bar is up (from the compositor's overview state, and per output), the pictures of Spaces
//! and the Space a dragged window is over, and the bar surfaces follow them; the surfaces write
//! back where their cards lie on the same feed, which the daemon reads to place the drop targets
//! and to ask for pictures. The picks: the surface writes what the person clicked, and the daemon
//! carries it out. The desktop's view, for the Spaces and their windows, is the one channel the
//! windows service writes; the link holds its receiver.

use porter_core::SpaceId;
use sill_model::desktop::{
    BarDrops, BarLayout, BarLayouts, BarOutputs, BarPick, BarPictures, BarShow, DesktopView, Shown,
    SpacePicture,
};
use tokio::sync::watch;

use super::{Outbox, Sent, Shared};

/// What the bar surfaces draw from.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SpacesBarFeed {
    /// Whether the bar is up.
    pub show: BarShow,
    /// On which outputs Mission Control is up.
    pub outputs: BarOutputs,
    /// The pictures of Spaces that have arrived.
    pub pictures: BarPictures,
    /// The Space a dragged window is over, per output.
    pub drops: BarDrops,
    /// Where each laid-out bar's cards lie (written by the surfaces).
    pub layouts: BarLayouts,
}

/// A click in the bar, numbered; the click is its `item`.
pub type SpacesBarPicked = Sent<BarPick>;

/// The shared channels, a root context of every bar surface and kept by the daemon. Clone is
/// cheap.
#[derive(Debug, Clone)]
pub struct SpacesBarLink {
    feed: Shared<SpacesBarFeed>,
    /// The desktop's view: the one channel the windows service writes.
    desktop: watch::Receiver<DesktopView>,
    picked: Outbox<BarPick>,
}

impl SpacesBarLink {
    /// Down, following the Spaces of `desktop`, no click.
    pub fn following(desktop: watch::Receiver<DesktopView>) -> SpacesBarLink {
        SpacesBarLink {
            feed: Shared::default(),
            desktop,
            picked: Outbox::default(),
        }
    }

    /// Down, with no Spaces (a desktop nobody writes) and no click.
    pub fn new() -> SpacesBarLink {
        SpacesBarLink::following(watch::Sender::new(DesktopView::default()).subscribe())
    }

    /// The bar goes up or down.
    pub fn set_show(&self, show: BarShow) {
        self.feed.modify_if(|feed| {
            let changed = feed.show != show;
            feed.show = show;
            changed
        });
    }

    /// A receiver for the desktop whose Spaces the bar shows.
    pub fn subscribe_desktop(&self) -> watch::Receiver<DesktopView> {
        self.desktop.clone()
    }

    /// Change one part of the feed; nothing is sent when the part comes out the same.
    fn edit<T: Clone + PartialEq>(
        &self,
        part: fn(&mut SpacesBarFeed) -> &mut T,
        change: impl FnOnce(T) -> T,
    ) {
        self.feed.modify_if(|feed| {
            let slot = part(feed);
            let next = change(slot.clone());
            let changed = next != *slot;
            *slot = next;
            changed
        });
    }

    /// The compositor says Mission Control is up or closed on `output`.
    pub fn set_output(&self, output: &str, shown: Shown) {
        self.edit(
            |feed| &mut feed.outputs,
            |outputs| outputs.with(output, shown),
        );
    }

    /// The dragged window is over `space` on `output`, or over none.
    pub fn set_drop(&self, output: &str, space: Option<SpaceId>) {
        self.edit(|feed| &mut feed.drops, |drops| drops.with(output, space));
    }

    /// The dragged window entered (`Shown`) or left (`Hidden`) the plus of `output`.
    pub fn set_drop_new(&self, output: &str, over: Shown) {
        self.edit(|feed| &mut feed.drops, |drops| drops.with_new(output, over));
    }

    /// A window drag began (`Shown`) or ended (`Hidden`) on `output`: the tiles show that they
    /// accept the window.
    pub fn set_dragging(&self, output: &str, drag: Shown) {
        self.edit(
            |feed| &mut feed.drops,
            |drops| drops.with_dragging(output, drag),
        );
    }

    /// The compositor answered a request for the picture of `space` on `output`.
    pub fn set_picture(&self, output: &str, space: SpaceId, answer: &SpacePicture) {
        self.edit(
            |feed| &mut feed.pictures,
            |pictures| pictures.with(output, space, answer),
        );
    }

    /// The bar on `output` has its cards at `layout`, or is gone (`None`).
    pub fn set_layout(&self, output: &str, layout: Option<BarLayout>) {
        self.edit(
            |feed| &mut feed.layouts,
            |mut layouts| {
                match layout {
                    Some(layout) => layouts.0.insert(output.to_owned(), layout),
                    None => layouts.0.remove(output),
                };
                layouts
            },
        );
    }

    /// What the surfaces draw now.
    pub fn latest(&self) -> SpacesBarFeed {
        self.feed.now()
    }

    /// A receiver for the surfaces to follow.
    pub fn subscribe(&self) -> watch::Receiver<SpacesBarFeed> {
        self.feed.subscribe()
    }

    /// The person clicked in the bar.
    pub fn pick(&self, pick: BarPick) {
        self.picked.send(pick);
    }

    /// The latest click.
    pub fn last_pick(&self) -> Option<SpacesBarPicked> {
        self.picked.latest()
    }

    /// A receiver for the daemon to follow.
    pub fn subscribe_picks(&self) -> watch::Receiver<Option<SpacesBarPicked>> {
        self.picked.subscribe()
    }
}

#[cfg(test)]
mod tests {
    use ds_core::vocab::DropState;
    use porter_core::SpaceId;
    use sill_model::desktop::{BarLayout, BarPick, BarShow, Shown, SpacePicture};

    use super::SpacesBarLink;

    #[test]
    fn the_bar_follows_the_show_and_a_repeat_wakes_no_one() {
        let link = SpacesBarLink::new();
        let mut follower = link.subscribe();
        assert_eq!(link.latest().show, BarShow::Hide);
        link.set_show(BarShow::Show);
        assert_eq!(follower.has_changed().ok(), Some(true));
        drop(follower.borrow_and_update());
        link.set_show(BarShow::Show);
        assert_eq!(follower.has_changed().ok(), Some(false));
        assert_eq!(link.latest().show, BarShow::Show);
    }

    #[test]
    fn the_wire_parts_are_written_one_by_one_and_a_repeat_wakes_no_one() {
        let link = SpacesBarLink::new();
        let mut follower = link.subscribe();
        let space = SpaceId::parse("ws-1").unwrap_or_else(|e| panic!("{e}"));
        link.set_output("DP-1", Shown::Shown);
        link.set_drop("DP-1", Some(space.clone()));
        link.set_drop_new("DP-2", Shown::Shown);
        link.set_dragging("DP-3", Shown::Shown);
        link.set_picture("DP-1", space.clone(), &SpacePicture::Png(vec![1]));
        link.set_layout("DP-1", Some(BarLayout::default()));
        let now = link.latest();
        assert_eq!(
            now.outputs.bar_on(BarShow::Show, Some("DP-1")),
            BarShow::Show
        );
        assert_eq!(now.drops.state_of(Some("DP-1"), &space), DropState::Target);
        assert_eq!(now.drops.new_state_of(Some("DP-2")), DropState::Target);
        assert_eq!(now.drops.new_state_of(Some("DP-3")), DropState::Accepts);
        assert_eq!(now.pictures.seq_of("DP-1", &space), 1);
        assert_eq!(now.layouts.0.len(), 1);
        drop(follower.borrow_and_update());
        link.set_output("DP-1", Shown::Shown);
        link.set_drop("DP-1", Some(space));
        link.set_drop_new("DP-2", Shown::Shown);
        link.set_dragging("DP-3", Shown::Shown);
        link.set_layout("DP-1", Some(BarLayout::default()));
        assert_eq!(follower.has_changed().ok(), Some(false));
        link.set_layout("DP-1", None);
        assert_eq!(follower.has_changed().ok(), Some(true));
        assert!(link.latest().layouts.0.is_empty());
    }

    #[test]
    fn each_click_counts_on() {
        let link = SpacesBarLink::new();
        assert_eq!(link.last_pick(), None);
        link.pick(BarPick::Add("DP-1".to_owned()));
        link.pick(BarPick::Add("DP-2".to_owned()));
        let last = link.last_pick().expect("picked");
        assert_eq!((last.seq, last.item), (2, BarPick::Add("DP-2".to_owned())));
    }
}
