//! The last value each key was listed with, for a list that keeps drawing a row after its caller
//! has stopped listing it: a banner or a tile that is leaving still needs its card or its app
//! until its exit has settled.

use dioxus::prelude::*;

/// A book of `(key, value)`, refreshed from what the caller lists on each render. A key the
/// caller drops keeps its last value until [`Kept::forget`].
#[derive(Debug)]
pub(crate) struct Kept<K: 'static, V: 'static>(CopyValue<Vec<(K, V)>>);

impl<K: 'static, V: 'static> Clone for Kept<K, V> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<K: 'static, V: 'static> Copy for Kept<K, V> {}

impl<K: Clone + PartialEq + 'static, V: Clone + 'static> Kept<K, V> {
    /// The value `key` was last listed with.
    pub(crate) fn of(&self, key: &K) -> Option<V> {
        self.0
            .peek()
            .iter()
            .find(|(held, _)| held == key)
            .map(|(_, value)| value.clone())
    }

    /// Drop `key`'s value: its exit has settled.
    pub(crate) fn forget(&self, key: &K) {
        let mut book = self.0;
        let _ = book
            .try_write()
            .map(|mut book| book.retain(|(held, _)| held != key));
    }
}

/// The book, refreshed from this render's `listed` (a listed value is always the newest one; a
/// dropped one keeps its last).
pub(crate) fn use_kept<K, V>(listed: Vec<(K, V)>) -> Kept<K, V>
where
    K: Clone + PartialEq + 'static,
    V: Clone + 'static,
{
    let mut book = use_hook(|| CopyValue::new(Vec::<(K, V)>::new()));
    let dropped = book
        .peek()
        .iter()
        .filter(|(key, _)| listed.iter().all(|(listed_key, _)| listed_key != key))
        .cloned()
        .collect::<Vec<_>>();
    let mut next = listed;
    next.extend(dropped);
    book.set(next);
    Kept(book)
}
