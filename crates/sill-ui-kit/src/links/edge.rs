//! The compositor's edge reveals (`quire_shell_v1.reveal`), from the daemon to the bar and dock
//! surfaces: in full screen it shows the bar when the pointer rests at the top edge and the dock
//! at the bottom, and hides them when it goes away. The note names no output, so the daemon
//! writes the focused window's output (the full-screen window) for a reveal; a hide is for
//! every output. The compositor moves the surface itself; the surfaces only make their content
//! drawn and hit-testable.

use shell_host::OutputId;
use sill_model::desktop::Shown;
use tokio::sync::watch;

use super::Shared;

/// One edge: away everywhere, or revealed on one output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EdgeState {
    /// Hidden on every output.
    #[default]
    Away,
    /// Revealed on this output.
    Revealed(OutputId),
}

impl EdgeState {
    /// Whether the surface on `output` is revealed.
    pub fn on(self, output: OutputId) -> Shown {
        match self {
            EdgeState::Revealed(on) if on == output => Shown::Shown,
            EdgeState::Revealed(_) | EdgeState::Away => Shown::Hidden,
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

/// The two edges that carry a surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct EdgeFeed {
    /// The bar's edge.
    pub top: EdgeState,
    /// The dock's edge.
    pub bottom: EdgeState,
}

impl EdgeFeed {
    /// The state of `edge`.
    pub fn at(&self, edge: RevealEdge) -> EdgeState {
        match edge {
            RevealEdge::Top => self.top,
            RevealEdge::Bottom => self.bottom,
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

    /// The top edge (the bar's) is now `state`.
    pub fn set_top(&self, state: EdgeState) {
        self.feed.modify_if(|feed| {
            let changed = feed.top != state;
            feed.top = state;
            changed
        });
    }

    /// The bottom edge (the dock's) is now `state`.
    pub fn set_bottom(&self, state: EdgeState) {
        self.feed.modify_if(|feed| {
            let changed = feed.bottom != state;
            feed.bottom = state;
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
    use super::{EdgeLink, EdgeState};
    use shell_host::OutputId;
    use sill_model::desktop::Shown;

    #[test]
    fn a_reveal_is_for_one_output_and_a_hide_for_all() {
        let link = EdgeLink::new();
        let (a, b) = (OutputId(1), OutputId(2));
        link.set_top(EdgeState::Revealed(a));
        let feed = link.latest();
        assert_eq!(feed.top.on(a), Shown::Shown);
        assert_eq!(feed.top.on(b), Shown::Hidden);
        assert_eq!(feed.bottom.on(a), Shown::Hidden);
        link.set_top(EdgeState::Away);
        assert_eq!(link.latest().top.on(a), Shown::Hidden);
    }
}
