//! The eyedropper's channel: the daemon holds the still pictures of the screens for the colour
//! being asked for ([`Frozen`]) and every output's eyedropper surface follows them; each surface
//! says it exists, so the daemon can tell the capture service an eyedropper is drawn.

use sill_model::capture_pick::{EyedropIn, EyedropOut, Frozen, eyedrop_step};
use tokio::sync::watch;

/// What the surfaces follow.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EyedropFeed {
    /// The pictures for the colour question open now; none when no colour is asked.
    pub frozen: Option<Frozen>,
    /// How many eyedropper surfaces exist.
    pub surfaces: u32,
}

/// The shared channel, a root context of every eyedropper surface and kept by the daemon. Clone is
/// cheap.
#[derive(Debug, Clone)]
pub struct EyedropperLink {
    feed: watch::Sender<EyedropFeed>,
}

impl Default for EyedropperLink {
    fn default() -> Self {
        EyedropperLink::new()
    }
}

impl EyedropperLink {
    /// No question, no surface.
    pub fn new() -> EyedropperLink {
        EyedropperLink {
            feed: watch::Sender::new(EyedropFeed::default()),
        }
    }

    /// Take `input` into the pictures held and return what that asks the daemon to do. The
    /// surfaces are woken only when the pictures changed.
    pub fn take(&self, input: EyedropIn) -> Vec<EyedropOut> {
        let mut out = Vec::new();
        self.feed.send_if_modified(|feed| {
            let before = feed.frozen.take();
            let (next, asked) = eyedrop_step(before.clone(), input);
            let changed = next != before;
            feed.frozen = next;
            out = asked;
            changed
        });
        out
    }

    /// An eyedropper surface exists.
    pub fn attach(&self) {
        self.feed.send_modify(|feed| feed.surfaces += 1);
    }

    /// An eyedropper surface is gone.
    pub fn detach(&self) {
        self.feed
            .send_modify(|feed| feed.surfaces = feed.surfaces.saturating_sub(1));
    }

    /// What is held now.
    pub fn latest(&self) -> EyedropFeed {
        self.feed.borrow().clone()
    }

    /// A receiver for the surfaces and the daemon to follow.
    pub fn subscribe(&self) -> watch::Receiver<EyedropFeed> {
        self.feed.subscribe()
    }
}

#[cfg(test)]
mod tests {
    use super::EyedropperLink;
    use sill_model::capture_pick::{AskId, EyedropIn, EyedropOut};

    fn asked(id: u64) -> EyedropIn {
        EyedropIn::Asked(Some((AskId(id), vec!["DP-1".to_owned()])))
    }

    #[test]
    fn a_question_wakes_the_surfaces_once_and_surfaces_are_counted() {
        let link = EyedropperLink::new();
        let mut follower = link.subscribe();
        assert_eq!(
            link.take(asked(1)),
            vec![EyedropOut::Capture("DP-1".to_owned())]
        );
        assert!(follower.has_changed().is_ok_and(|changed| changed));
        drop(follower.borrow_and_update());
        assert_eq!(link.take(asked(1)), vec![]);
        assert!(
            follower.has_changed().is_ok_and(|changed| !changed),
            "the same question changes nothing"
        );
        link.attach();
        link.attach();
        link.detach();
        assert_eq!(link.latest().surfaces, 1);
        link.detach();
        link.detach();
        assert_eq!(link.latest().surfaces, 0, "never below none");
        assert_eq!(link.take(EyedropIn::Asked(None)), vec![]);
        assert_eq!(link.latest().frozen, None);
    }
}
