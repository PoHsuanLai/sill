//! What the windows thread publishes: the projected state, whole, after every dispatch.

use sill_model::windows::WindowModel;
use tokio::sync::watch;

/// The sending half of the service's state. Receivers wake only when a step changed something.
#[derive(Debug)]
pub(crate) struct Publish {
    tx: watch::Sender<WindowModel>,
}

impl Publish {
    pub(crate) fn new(tx: watch::Sender<WindowModel>) -> Publish {
        Publish { tx }
    }

    /// Publish `next` whole, as the arms do after every dispatch: they project the full
    /// state, so there is no event to step.
    pub(crate) fn replace(&self, next: WindowModel) {
        self.tx
            .send_if_modified(|now| replace_if_changed(now, |_| next));
    }

    /// Back to empty: the compositor has gone, and its windows and workspaces with it.
    pub(crate) fn clear(&self) {
        self.tx
            .send_if_modified(|now| replace_if_changed(now, |_| WindowModel::default()));
    }
}

/// Put `step(now)` into `now`; whether that changed it (the `send_if_modified` contract).
fn replace_if_changed<T: Clone + PartialEq>(now: &mut T, step: impl FnOnce(T) -> T) -> bool {
    let next = step(now.clone());
    let changed = next != *now;
    *now = next;
    changed
}
