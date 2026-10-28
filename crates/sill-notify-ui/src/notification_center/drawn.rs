//! Which rows the center still draws: the ones it lists, and the ones quire's `List` is
//! still folding away after the model dropped them. The empty state ("No Notifications") waits
//! for the last of them to settle, so after Clear All it takes the column's place once the rows
//! have folded rather than under them.

use dioxus::prelude::*;

use super::rows::RowKey;

/// The rows drawn, and a count of settles that redraws the column when one goes.
#[derive(Clone, Copy)]
pub(super) struct Drawn {
    keys: CopyValue<Vec<RowKey>>,
    settles: Signal<u64>,
}

/// The rows drawn this render, `listed` among them.
pub(super) fn use_drawn(listed: &[RowKey]) -> Drawn {
    let mut keys = use_hook(|| CopyValue::new(Vec::<RowKey>::new()));
    let settles = use_signal(|| 0u64);
    let next = drawn_after(&keys.peek(), listed);
    keys.set(next);
    Drawn { keys, settles }
}

impl Drawn {
    /// Whether any row is drawn; the caller redraws when a leaving row settles.
    pub(super) fn any_rows(&self) -> bool {
        let _redraw = (self.settles)();
        !self.keys.peek().is_empty()
    }

    /// `key`'s exit has settled: it is no longer drawn.
    pub(super) fn settled(&self, key: &RowKey) {
        let mut keys = self.keys;
        keys.write().retain(|held| held != key);
        let mut settles = self.settles;
        settles += 1;
    }
}

/// The rows drawn once `listed` is listed: all of them, then the ones drawn before that are
/// still leaving.
fn drawn_after(before: &[RowKey], listed: &[RowKey]) -> Vec<RowKey> {
    listed
        .iter()
        .cloned()
        .chain(before.iter().filter(|key| !listed.contains(key)).cloned())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::drawn_after;
    use crate::notification_center::rows::RowKey;
    use sill_model::notifications::{AppKey, NotificationId};

    #[test]
    fn a_dropped_row_stays_drawn_until_it_settles() {
        let head = RowKey::Head(AppKey("a".to_owned()));
        let card = RowKey::Card(NotificationId(1));
        let both = vec![head.clone(), card.clone()];
        assert_eq!(drawn_after(&[], &both), both);
        assert_eq!(drawn_after(&both, &[]), both, "cleared: still folding");
        assert_eq!(
            drawn_after(&both, std::slice::from_ref(&card)),
            vec![card, head]
        );
        assert_eq!(drawn_after(&[], &[]), Vec::<RowKey>::new());
    }
}
