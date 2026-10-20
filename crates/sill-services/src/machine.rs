//! A service's state machine and its published value, in one place: an event is fed through the
//! model's pure `step`, the result is published when it differs from what readers hold, and the
//! effects the step asked for are handed back to be carried out. Every service publishes the same
//! way: readers wake on a change, never on an equal value.

use std::ops::Deref;

use tokio::sync::watch;

/// A published value reduced by `step`, for the services whose step returns only the next value.
#[derive(Debug)]
pub(crate) struct Published<T, E> {
    tx: watch::Sender<T>,
    step: fn(T, E) -> T,
}

impl<T: Clone + PartialEq, E> Published<T, E> {
    /// `tx`'s value reduced by `step`.
    pub(crate) fn new(tx: watch::Sender<T>, step: fn(T, E) -> T) -> Published<T, E> {
        Published { tx, step }
    }

    /// Reduce `event` onto the value now and publish the result when it differs.
    pub(crate) fn feed(&self, event: E) {
        self.set(self.next(event));
    }

    /// The value `event` would make, for a service that must do something (save it) before
    /// readers see it: publish the result with [`Published::set`].
    pub(crate) fn next(&self, event: E) -> T {
        (self.step)(self.tx.borrow().clone(), event)
    }

    /// Publish `next` when it differs from the value now.
    pub(crate) fn set(&self, next: T) {
        publish(&self.tx, next);
    }
}

impl<T, E> Clone for Published<T, E> {
    fn clone(&self) -> Published<T, E> {
        Published {
            tx: self.tx.clone(),
            step: self.step,
        }
    }
}

impl<T, E> Deref for Published<T, E> {
    type Target = watch::Sender<T>;

    fn deref(&self) -> &watch::Sender<T> {
        &self.tx
    }
}

/// A state that steps through the model's pure `step` and returns the effects to carry out. The
/// state lives here and is published on every change; readers get it from the sender.
#[derive(Debug)]
pub(crate) struct Machine<S, E, F> {
    state: S,
    tx: watch::Sender<S>,
    step: fn(S, E) -> (S, Vec<F>),
}

impl<S: Default + Clone + PartialEq, E, F> Machine<S, E, F> {
    /// A machine at `S::default()`, publishing on `tx`.
    pub(crate) fn new(tx: watch::Sender<S>, step: fn(S, E) -> (S, Vec<F>)) -> Machine<S, E, F> {
        Machine {
            state: S::default(),
            tx,
            step,
        }
    }

    /// The state now.
    pub(crate) fn state(&self) -> &S {
        &self.state
    }

    /// Step by `event`, publish the next state when it differs, and return the effects.
    pub(crate) fn feed(&mut self, event: E) -> Vec<F> {
        let (next, effects) = (self.step)(std::mem::take(&mut self.state), event);
        publish(&self.tx, next.clone());
        self.state = next;
        effects
    }
}

/// Make `tx` hold `next`, waking its readers only when that differs.
fn publish<T: PartialEq>(tx: &watch::Sender<T>, next: T) {
    tx.send_if_modified(|held| {
        if *held == next {
            return false;
        }
        *held = next;
        true
    });
}

#[cfg(test)]
mod tests {
    use tokio::sync::watch;

    use super::{Machine, Published};

    fn add(total: u32, n: u32) -> u32 {
        total + n
    }

    fn count(total: u32, n: u32) -> (u32, Vec<String>) {
        (total + n, vec![format!("added {n}")])
    }

    #[test]
    fn a_published_value_wakes_its_readers_only_when_it_changes() {
        let (tx, mut rx) = watch::channel(1_u32);
        let published = Published::new(tx, add);
        published.feed(0);
        assert!(
            !rx.has_changed().unwrap_or(true),
            "an equal value is not a change"
        );
        published.feed(2);
        assert!(rx.has_changed().unwrap_or(false));
        assert_eq!(*rx.borrow_and_update(), 3);
    }

    #[test]
    fn a_machine_publishes_the_next_state_and_returns_the_effects() {
        let (tx, rx) = watch::channel(0_u32);
        let mut machine = Machine::new(tx, count);
        assert_eq!(machine.feed(2), vec!["added 2".to_owned()]);
        assert_eq!(machine.feed(3), vec!["added 3".to_owned()]);
        assert_eq!((*machine.state(), *rx.borrow()), (5, 5));
    }
}
