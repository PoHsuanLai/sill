//! The two watch channels, fed only through the pure reducers.

use tokio::sync::watch;

use crate::cosmic_wl::toplevels::{ToplevelEvent, Toplevels, reduce_toplevels};
use crate::cosmic_wl::workspaces::{WorkspaceEvent, Workspaces, reduce_workspaces};

/// The sending halves of `spawn`'s receivers. Receivers wake only when a reduction changed
/// something.
#[derive(Debug)]
pub(crate) struct Publish {
    toplevels: watch::Sender<Toplevels>,
    workspaces: watch::Sender<Workspaces>,
}

impl Publish {
    /// The publisher and the two receivers `spawn` hands out, both empty.
    pub(super) fn channels() -> (
        Publish,
        watch::Receiver<Workspaces>,
        watch::Receiver<Toplevels>,
    ) {
        let (toplevels, toplevels_rx) = watch::channel(Toplevels::default());
        let (workspaces, workspaces_rx) = watch::channel(Workspaces::default());
        (
            Publish {
                toplevels,
                workspaces,
            },
            workspaces_rx,
            toplevels_rx,
        )
    }

    pub(super) fn toplevel(&self, event: ToplevelEvent) {
        self.toplevels
            .send_if_modified(|now| replace_if_changed(now, |t| reduce_toplevels(t, event)));
    }

    pub(super) fn workspace(&self, event: WorkspaceEvent) {
        self.workspaces
            .send_if_modified(|now| replace_if_changed(now, |w| reduce_workspaces(w, event)));
    }

    /// Back to empty: the compositor has gone, and its windows and workspaces with it.
    pub(super) fn clear(&self) {
        self.toplevels
            .send_if_modified(|now| replace_if_changed(now, |_| Toplevels::default()));
        self.workspaces
            .send_if_modified(|now| replace_if_changed(now, |_| Workspaces::default()));
    }
}

/// Put `reduce(now)` into `now`; whether that changed it (the `send_if_modified` contract).
fn replace_if_changed<T: Clone + PartialEq>(now: &mut T, reduce: impl FnOnce(T) -> T) -> bool {
    let next = reduce(now.clone());
    let changed = next != *now;
    *now = next;
    changed
}
