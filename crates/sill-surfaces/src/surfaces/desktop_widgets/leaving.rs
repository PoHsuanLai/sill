//! A desktop widget taken away plays its exit (G423; quire CONSUMING "Edit Widgets, one size"):
//! when a kind leaves `widgets.desktop_widgets` (an Edit Widgets Remove), the layer keeps drawing
//! its card at the box it last stood in, `CardPresence::Leaving`, until quire's card calls
//! `on_gone` at `settle(WidgetOut)`, and then drops it. A leaving card is out of the grid's
//! placements, so it is out of the input and blur regions and the drag from the moment it
//! starts leaving. A kind added back while it leaves is placed again, and quire's `Placed`
//! takes the exit back. Pure but for [`use_leaving`].

use dioxus::prelude::*;
use sill_settings::WidgetKind;

use super::geometry::PlaceBox;

/// A card on its way out: its kind and the box it last stood in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Leaving {
    pub kind: WidgetKind,
    pub target: PlaceBox,
}

/// The cards leaving once the desktop's kinds went from `was` to `now`: those already leaving
/// that were not added back, then each kind taken away that stood at a box in `boxes` (the
/// last layout's; a kind that fit nowhere had no card to see go).
pub fn leaving_after(
    leaving: &[Leaving],
    was: &[WidgetKind],
    now: &[WidgetKind],
    boxes: &[(WidgetKind, PlaceBox)],
) -> Vec<Leaving> {
    let kept = leaving.iter().filter(|l| !now.contains(&l.kind)).copied();
    let taken = was
        .iter()
        .filter(|kind| !now.contains(kind))
        .filter(|kind| leaving.iter().all(|l| l.kind != **kind))
        .filter_map(|&kind| {
            let target = boxes.iter().find(|(k, _)| *k == kind)?.1;
            Some(Leaving { kind, target })
        });
    kept.chain(taken).collect()
}

/// `leaving` without `kind`'s card: it has gone.
pub fn gone(leaving: &[Leaving], kind: WidgetKind) -> Vec<Leaving> {
    leaving.iter().filter(|l| l.kind != kind).copied().collect()
}

/// The cards leaving the grid, kept across renders.
#[derive(Debug, Clone, Copy)]
pub struct LeavingCards {
    cards: CopyValue<Vec<Leaving>>,
    seen: CopyValue<Vec<WidgetKind>>,
    /// Bumped when a card has gone, so the grid draws again without it.
    gone: Signal<u32>,
}

impl LeavingCards {
    /// The cards leaving now that the desktop shows `kinds`, `boxes` being where the last
    /// layout drew each (read before this render's layout replaces it).
    pub fn now(self, kinds: &[WidgetKind], boxes: &[(WidgetKind, PlaceBox)]) -> Vec<Leaving> {
        // Read, so a card's going draws the grid again.
        let _ = (self.gone)();
        let was = self.seen.peek().clone();
        if was.as_slice() != kinds {
            let next = leaving_after(&self.cards.peek(), &was, kinds, boxes);
            let (mut cards, mut seen) = (self.cards, self.seen);
            cards.set(next);
            seen.set(kinds.to_vec());
        }
        self.cards.peek().clone()
    }

    /// What a leaving card of `kind` calls once its exit has played.
    pub fn on_gone(self, kind: WidgetKind) -> EventHandler<()> {
        EventHandler::new(move |()| {
            let (mut cards, mut tick) = (self.cards, self.gone);
            let rest = gone(&cards.peek(), kind);
            cards.set(rest);
            tick += 1;
        })
    }
}

/// The grid's leaving cards, starting from the kinds it first shows (nothing leaves on a map).
pub fn use_leaving(kinds: &[WidgetKind]) -> LeavingCards {
    let first = kinds.to_vec();
    let cards = use_hook(|| CopyValue::new(Vec::new()));
    let seen = use_hook(|| CopyValue::new(first));
    let gone = use_signal(|| 0_u32);
    LeavingCards { cards, seen, gone }
}

#[cfg(test)]
mod tests {
    use sill_settings::WidgetKind;

    use super::{Leaving, PlaceBox, gone, leaving_after};

    fn at(left: u32) -> PlaceBox {
        PlaceBox {
            left,
            top: 0,
            width: 164,
            height: 164,
        }
    }

    fn leaving(kind: WidgetKind, left: u32) -> Leaving {
        Leaving {
            kind,
            target: at(left),
        }
    }

    #[test]
    fn a_kind_taken_away_leaves_from_its_last_box_until_it_has_gone_or_comes_back() {
        use WidgetKind::{Battery, Calendar, UpNext, WorldClock};
        let boxes = [(Calendar, at(900)), (Battery, at(1080)), (UpNext, at(0))];
        // (what, leaving before, the kinds before, the kinds now, leaving after)
        let cases = [
            (
                "a removal leaves from where it stood",
                vec![],
                vec![Calendar, Battery],
                vec![Calendar],
                vec![leaving(Battery, 1080)],
            ),
            (
                "an add leaves nothing",
                vec![],
                vec![Calendar],
                vec![Calendar, WorldClock],
                vec![],
            ),
            (
                "one still leaving stays while another goes",
                vec![leaving(Battery, 1080)],
                vec![Calendar, UpNext],
                vec![Calendar],
                vec![leaving(Battery, 1080), leaving(UpNext, 0)],
            ),
            (
                "one added back while it leaves is placed again",
                vec![leaving(Battery, 1080)],
                vec![Calendar],
                vec![Calendar, Battery],
                vec![],
            ),
            (
                "a kind with no box had no card to see go",
                vec![],
                vec![Calendar, WorldClock],
                vec![Calendar],
                vec![],
            ),
        ];
        for (name, now_leaving, was, now, want) in cases {
            assert_eq!(
                leaving_after(&now_leaving, &was, &now, &boxes),
                want,
                "{name}"
            );
        }
    }

    #[test]
    fn a_card_that_has_gone_is_dropped_alone() {
        let both = [
            leaving(WidgetKind::Battery, 1080),
            leaving(WidgetKind::UpNext, 0),
        ];
        assert_eq!(
            gone(&both, WidgetKind::Battery),
            vec![leaving(WidgetKind::UpNext, 0)]
        );
        assert_eq!(gone(&[], WidgetKind::Battery), vec![]);
    }
}
