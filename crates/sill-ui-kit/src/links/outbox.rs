//! What a surface says back to the daemon: an answer, a pick, a click. One numbered slot over a
//! watch channel; the daemon's forwarder follows it and carries each item out. A repeat of the
//! same item is a new send, which is what the number is for.

use std::ops::Deref;

use tokio::sync::watch;

use super::Shared;

/// One thing sent, and which send it was.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sent<T> {
    /// Counts up from 1 per outbox.
    pub seq: u64,
    pub item: T,
}

impl<T> Deref for Sent<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.item
    }
}

/// The latest thing a surface sent, if any. Clone is cheap.
#[derive(Debug)]
pub struct Outbox<T>(Shared<Option<Sent<T>>>);

impl<T> Default for Outbox<T> {
    fn default() -> Self {
        Outbox(Shared::default())
    }
}

impl<T: Clone> Clone for Outbox<T> {
    fn clone(&self) -> Self {
        Outbox(self.0.clone())
    }
}

impl<T: Clone> Outbox<T> {
    /// Send `item`; followers are told even when it equals the last.
    pub fn send(&self, item: T) {
        self.0.modify(|last| {
            let seq = last.as_ref().map_or(1, |last| last.seq + 1);
            *last = Some(Sent { seq, item });
        });
    }

    /// The latest send.
    pub fn latest(&self) -> Option<Sent<T>> {
        self.0.now()
    }

    /// A receiver for the daemon to follow.
    pub fn subscribe(&self) -> watch::Receiver<Option<Sent<T>>> {
        self.0.subscribe()
    }
}

#[cfg(test)]
mod tests {
    use super::Outbox;

    #[test]
    fn each_send_counts_on_even_when_it_repeats() {
        let outbox = Outbox::default();
        assert_eq!(outbox.latest(), None);
        outbox.send("term");
        outbox.send("term");
        let last = outbox.latest().expect("sent");
        assert_eq!((last.seq, *last), (2, "term"));
    }
}
