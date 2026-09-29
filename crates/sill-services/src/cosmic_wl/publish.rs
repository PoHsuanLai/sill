//! What the `cosmic_wl` thread publishes, fed only through the pure `step`s.

use tokio::sync::watch;

use crate::cosmic_wl::toplevels::{self, ToplevelEvent, Toplevels};
use crate::cosmic_wl::workspaces::{self, WorkspaceEvent, Workspaces};

/// The compositor's workspaces and windows: one Wayland connection's state, published together
/// (a reader that wants one of them projects it).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CosmicWl {
    pub workspaces: Workspaces,
    pub toplevels: Toplevels,
}

/// The sending half of the service's state. Receivers wake only when a step changed something.
#[derive(Debug)]
pub(crate) struct Publish {
    tx: watch::Sender<CosmicWl>,
}

impl Publish {
    pub(super) fn new(tx: watch::Sender<CosmicWl>) -> Publish {
        Publish { tx }
    }

    pub(super) fn toplevel(&self, event: ToplevelEvent) {
        self.tx.send_if_modified(|now| {
            replace_if_changed(&mut now.toplevels, |t| toplevels::step(t, event))
        });
    }

    pub(super) fn workspace(&self, event: WorkspaceEvent) {
        self.tx.send_if_modified(|now| {
            replace_if_changed(&mut now.workspaces, |w| workspaces::step(w, event))
        });
    }

    /// Back to empty: the compositor has gone, and its windows and workspaces with it.
    pub(super) fn clear(&self) {
        self.tx
            .send_if_modified(|now| replace_if_changed(now, |_| CosmicWl::default()));
    }
}

/// Put `step(now)` into `now`; whether that changed it (the `send_if_modified` contract).
fn replace_if_changed<T: Clone + PartialEq>(now: &mut T, step: impl FnOnce(T) -> T) -> bool {
    let next = step(now.clone());
    let changed = next != *now;
    *now = next;
    changed
}
