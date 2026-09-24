//! The keys the thread hands out for protocol handles, so no protocol object leaves the thread.

use std::collections::HashMap;
use std::hash::Hash;

/// Handles of one kind and the numbers published for them. A key is never reused: a closed
/// window's key cannot come back as a different window.
#[derive(Debug)]
pub(super) struct Keyring<H> {
    keys: HashMap<H, u64>,
    next: u64,
}

impl<H> Default for Keyring<H> {
    fn default() -> Self {
        Keyring {
            keys: HashMap::new(),
            next: 1,
        }
    }
}

impl<H: Hash + Eq + Clone> Keyring<H> {
    /// The key of `handle`, given one now if it has none.
    pub(super) fn key(&mut self, handle: &H) -> u64 {
        if let Some(&key) = self.keys.get(handle) {
            return key;
        }
        let key = self.next;
        self.next += 1;
        self.keys.insert(handle.clone(), key);
        key
    }

    /// The key `handle` already has, without giving it one.
    pub(super) fn existing(&self, handle: &H) -> Option<u64> {
        self.keys.get(handle).copied()
    }

    /// The handle `key` was given to, while it is still known.
    pub(super) fn handle(&self, key: u64) -> Option<&H> {
        self.keys
            .iter()
            .find_map(|(handle, &k)| (k == key).then_some(handle))
    }

    /// Drop `handle`'s key (the object is gone).
    pub(super) fn forget(&mut self, handle: &H) {
        self.keys.remove(handle);
    }

    /// Drop every key whose handle `alive` refuses.
    pub(super) fn retain(&mut self, alive: impl Fn(&H) -> bool) {
        self.keys.retain(|handle, _| alive(handle));
    }
}

#[cfg(test)]
mod tests {
    use super::Keyring;

    #[test]
    fn keys_are_stable_and_never_reused() {
        let mut ring = Keyring::<&str>::default();
        let a = ring.key(&"a");
        let b = ring.key(&"b");
        assert_eq!(ring.key(&"a"), a, "asking again gives the same key");
        assert_ne!(a, b);
        assert_eq!(ring.handle(b), Some(&"b"));
        assert_eq!(
            ring.existing(&"c"),
            None,
            "asking without giving hands out nothing"
        );
        ring.forget(&"a");
        assert_eq!(ring.handle(a), None);
        let again = ring.key(&"a");
        assert!(again > b, "a returning handle gets a fresh key, not {a}");
        ring.retain(|h| *h != "b");
        assert_eq!(ring.handle(b), None);
        assert_eq!(ring.handle(again), Some(&"a"));
    }
}
