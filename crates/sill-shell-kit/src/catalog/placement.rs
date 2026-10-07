//! Placements as data: what the person put where, as a list a host stores in its settings.

use serde::{Deserialize, Serialize};

/// A placement's identity in its list, stable across edits (a size change keeps it).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PlacementId(pub u32);

/// One placed item: which kind, at which size, where.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Placed<K, S, A> {
    /// Its identity.
    pub id: PlacementId,
    /// What is placed.
    pub kind: K,
    /// How big.
    pub size: S,
    /// Where.
    pub at: A,
}

/// Every placement on one surface, in the order they were placed, and the next identity to
/// hand out. Serialises as `{"items":[…],"next":n}`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Placements<K, S, A> {
    items: Vec<Placed<K, S, A>>,
    next: PlacementId,
}

impl<K, S, A> Default for Placements<K, S, A> {
    fn default() -> Self {
        Placements {
            items: Vec::new(),
            next: PlacementId(1),
        }
    }
}

impl<K, S, A> Placements<K, S, A> {
    /// Every placement.
    pub fn items(&self) -> &[Placed<K, S, A>] {
        &self.items
    }

    /// The placement `id`.
    pub fn get(&self, id: PlacementId) -> Option<&Placed<K, S, A>> {
        self.items.iter().find(|item| item.id == id)
    }

    /// This list with `kind` placed at `size` and `at`, and the new placement's identity.
    pub fn added(self, kind: K, size: S, at: A) -> (Self, PlacementId) {
        let id = self.next;
        let mut items = self.items;
        items.push(Placed { id, kind, size, at });
        let next = PlacementId(id.0.saturating_add(1));
        (Placements { items, next }, id)
    }

    /// This list without `id` (unchanged if it holds none).
    pub fn removed(self, id: PlacementId) -> Self {
        let items = self
            .items
            .into_iter()
            .filter(|item| item.id != id)
            .collect();
        Placements { items, ..self }
    }

    /// This list with `id` changed by `change` (unchanged if it holds none).
    pub fn changed(
        self,
        id: PlacementId,
        change: impl FnOnce(Placed<K, S, A>) -> Placed<K, S, A>,
    ) -> Self {
        let mut change = Some(change);
        let items = self
            .items
            .into_iter()
            .map(|item| match change.take_if(|_| item.id == id) {
                Some(change) => change(item),
                None => item,
            })
            .collect();
        Placements { items, ..self }
    }
}

#[cfg(test)]
mod tests {
    use super::{PlacementId, Placements};

    #[test]
    fn edits_keep_identities_and_order() {
        let list = Placements::<&str, u8, u8>::default();
        let (list, a) = list.added("clock", 1, 0);
        let (list, b) = list.added("battery", 2, 1);
        assert_eq!((a, b), (PlacementId(1), PlacementId(2)));
        let list = list.changed(a, |item| super::Placed { size: 3, ..item });
        assert_eq!(list.get(a).map(|item| item.size), Some(3));
        let list = list.removed(a);
        assert_eq!(list.items().len(), 1);
        let (list, c) = list.added("month", 1, 2);
        assert_eq!(c, PlacementId(3), "an identity is never reused");
        assert_eq!(list.items()[0].id, b);
    }
}
