//! The compositor's edge reveals (`quire_shell_v1.reveal_on_output`), from the daemon to the bar
//! and dock surfaces: in full screen it shows an output's bar when the pointer rests at the top
//! edge and its dock at the bottom, and hides them when it goes away. The compositor moves the
//! surface itself; the surfaces only make their content drawn and hit-testable. The daemon also
//! writes which outputs show a full-screen Space, for the bar modes that stay up there.

use std::collections::BTreeSet;

use shell_host::OutputId;
use sill_model::desktop::Shown;
use tokio::sync::watch;

use super::Shared;

/// One edge: the outputs it is revealed on.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EdgeState(BTreeSet<OutputId>);

impl EdgeState {
    /// Whether the surface on `output` is revealed.
    pub fn on(&self, output: OutputId) -> Shown {
        match self.0.contains(&output) {
            true => Shown::Shown,
            false => Shown::Hidden,
        }
    }
}

/// The outputs whose shown Space is a full-screen one.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FullScreens(pub BTreeSet<OutputId>);

impl FullScreens {
    /// Whether `output` shows a full-screen Space.
    pub fn on(&self, output: OutputId) -> Shown {
        match self.0.contains(&output) {
            true => Shown::Shown,
            false => Shown::Hidden,
        }
    }
}

/// An edge that carries a surface: the bar's or the dock's.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RevealEdge {
    /// The top edge, the bar's.
    Top,
    /// The bottom edge, the dock's.
    Bottom,
}

/// The two edges that carry a surface, and the outputs in full screen.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EdgeFeed {
    /// The bar's edge.
    pub top: EdgeState,
    /// The dock's edge.
    pub bottom: EdgeState,
    /// The outputs showing a full-screen Space.
    pub full_screen: FullScreens,
}

impl EdgeFeed {
    /// The state of `edge`.
    pub fn at(&self, edge: RevealEdge) -> &EdgeState {
        match edge {
            RevealEdge::Top => &self.top,
            RevealEdge::Bottom => &self.bottom,
        }
    }

    fn at_mut(&mut self, edge: RevealEdge) -> &mut EdgeState {
        match edge {
            RevealEdge::Top => &mut self.top,
            RevealEdge::Bottom => &mut self.bottom,
        }
    }
}

/// The shared channel, a root context of the bar and the dock. Clone is cheap.
#[derive(Debug, Clone, Default)]
pub struct EdgeLink {
    feed: Shared<EdgeFeed>,
}

impl EdgeLink {
    /// Both edges away.
    pub fn new() -> EdgeLink {
        EdgeLink::default()
    }

    /// `edge` of `output` is now `shown`.
    pub fn set(&self, edge: RevealEdge, output: OutputId, shown: Shown) {
        self.feed.modify_if(|feed| {
            let outputs = &mut feed.at_mut(edge).0;
            match shown {
                Shown::Shown => outputs.insert(output),
                Shown::Hidden => outputs.remove(&output),
            }
        });
    }

    /// `edge` is hidden on every output (a compositor that names no output hid it).
    pub fn clear(&self, edge: RevealEdge) {
        self.feed.modify_if(|feed| {
            let outputs = &mut feed.at_mut(edge).0;
            let changed = !outputs.is_empty();
            outputs.clear();
            changed
        });
    }

    /// The outputs showing a full-screen Space are now `outputs`.
    pub fn set_full_screen(&self, outputs: FullScreens) {
        self.feed.modify_if(|feed| {
            let changed = feed.full_screen != outputs;
            feed.full_screen = outputs;
            changed
        });
    }

    /// What the surfaces follow.
    pub fn subscribe(&self) -> watch::Receiver<EdgeFeed> {
        self.feed.subscribe()
    }

    /// Both edges now.
    pub fn latest(&self) -> EdgeFeed {
        self.feed.now()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::{EdgeLink, FullScreens, RevealEdge};
    use shell_host::OutputId;
    use sill_model::desktop::Shown;

    #[test]
    fn each_output_keeps_its_own_reveal_and_a_clear_hides_all() {
        let link = EdgeLink::new();
        let (a, b) = (OutputId(1), OutputId(2));
        link.set(RevealEdge::Top, a, Shown::Shown);
        link.set(RevealEdge::Top, b, Shown::Shown);
        link.set(RevealEdge::Bottom, b, Shown::Shown);
        link.set(RevealEdge::Top, a, Shown::Hidden);
        let feed = link.latest();
        assert_eq!(feed.top.on(a), Shown::Hidden);
        assert_eq!(feed.top.on(b), Shown::Shown);
        assert_eq!(feed.bottom.on(a), Shown::Hidden);
        assert_eq!(feed.bottom.on(b), Shown::Shown);
        link.clear(RevealEdge::Bottom);
        assert_eq!(link.latest().bottom.on(b), Shown::Hidden);
        assert_eq!(link.latest().top.on(b), Shown::Shown);
        link.set_full_screen(FullScreens(BTreeSet::from([a])));
        assert_eq!(link.latest().full_screen.on(a), Shown::Shown);
        assert_eq!(link.latest().full_screen.on(b), Shown::Hidden);
    }
}
