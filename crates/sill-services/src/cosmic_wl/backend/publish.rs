//! What the `cosmic_wl` thread publishes, fed only through the pure `step`s.

use sill_model::cosmic_wl::toplevels::ToplevelEvent;
use sill_model::cosmic_wl::workspaces::WorkspaceEvent;
use sill_model::cosmic_wl::{CosmicWl, toplevels, workspaces};
use tokio::sync::watch;

/// The sending half of the service's state. Receivers wake only when a step changed something.
#[derive(Debug)]
pub(crate) struct Publish {
    tx: watch::Sender<CosmicWl>,
}

impl Publish {
    pub(crate) fn new(tx: watch::Sender<CosmicWl>) -> Publish {
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

    /// Publish `next` whole, as the standard-protocol arm does after every dispatch: it
    /// projects the full state, so there is no event to step.
    pub(crate) fn replace(&self, next: CosmicWl) {
        self.tx
            .send_if_modified(|now| replace_if_changed(now, |_| next));
    }

    /// Back to empty: the compositor has gone, and its windows and workspaces with it.
    pub(crate) fn clear(&self) {
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
