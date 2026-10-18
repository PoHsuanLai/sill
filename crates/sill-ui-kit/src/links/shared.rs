//! One value shared between the daemon and the surfaces that follow it: a watch channel with the
//! few operations every link needs, so a link names its verbs and not the plumbing.

use tokio::sync::watch;

/// A shared value. Clone is cheap and every clone is the same value.
#[derive(Debug, Clone)]
pub struct Shared<T>(watch::Sender<T>);

impl<T: Default> Default for Shared<T> {
    fn default() -> Self {
        Shared(watch::Sender::new(T::default()))
    }
}

impl<T> Shared<T> {
    /// A receiver for a surface to follow.
    pub fn subscribe(&self) -> watch::Receiver<T> {
        self.0.subscribe()
    }

    /// The current value, borrowed: read it and let go, since a writer waits for the borrow.
    pub fn read(&self) -> watch::Ref<'_, T> {
        self.0.borrow()
    }

    /// Change the value in place; every follower is told.
    pub fn modify(&self, change: impl FnOnce(&mut T)) {
        self.0.send_modify(change);
    }

    /// Change the value in place; followers are told when `change` says it changed.
    pub fn modify_if(&self, change: impl FnOnce(&mut T) -> bool) {
        self.0.send_if_modified(change);
    }
}

impl<T: Clone> Shared<T> {
    /// The current value.
    pub fn now(&self) -> T {
        self.0.borrow().clone()
    }
}

impl<T: PartialEq> Shared<T> {
    /// Store `next`; followers are told only when it differs.
    pub fn set(&self, next: T) {
        self.0.send_if_modified(|now| {
            let changed = *now != next;
            *now = next;
            changed
        });
    }

    /// Store what `next` makes of the current value; followers are told only when it differs.
    pub fn update(&self, next: impl FnOnce(&T) -> T) {
        let next = {
            let now = self.0.borrow();
            next(&now)
        };
        self.set(next);
    }
}
