//! Reading a value the providers share behind a lock.

use std::sync::Mutex;

/// A copy of what `shared` holds now; a poisoned lock (a panic elsewhere while holding it) still
/// yields the data rather than poisoning every query after it.
pub(super) fn copy_of<T: Clone>(shared: &Mutex<T>) -> T {
    shared
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone()
}
