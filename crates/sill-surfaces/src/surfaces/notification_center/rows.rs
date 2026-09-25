//! The rows the center draws, in order, and what a press, Return or Delete on one asks for,
//! pure. A collapsed group is its newest card stacked over the rest (macOS: a click on the stack
//! opens it, its close clears the group) and an "N more" line; an opened group of several ends
//! in "Show less".

use sill_services::{ActionKey, AppKey, NotificationId};

use super::groups::GroupView;
use crate::surfaces::notification_card::CardPress;

/// Whether a card stands alone or on top of its collapsed group.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Stack {
    Alone,
    /// The newest of a collapsed group of this many.
    Stacked(usize),
}

impl Stack {
    /// How many the card's count chip says.
    pub fn count(self) -> usize {
        match self {
            Stack::Alone => 1,
            Stack::Stacked(count) => count,
        }
    }
}

/// A group's opening line.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Fold {
    /// Collapsed, hiding this many.
    More(usize),
    /// Opened, with several shown.
    Less,
}

/// One row the keys move over.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CenterRow {
    Note {
        group: AppKey,
        id: NotificationId,
        stack: Stack,
    },
    Fold {
        group: AppKey,
        fold: Fold,
    },
}

/// What a row asks of the center.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RowAct {
    /// Invoke the action (the body: `default`), then close the center.
    Invoke(NotificationId, ActionKey),
    /// Open or close the group.
    Toggle(AppKey),
    /// Dismiss one notification.
    Dismiss(NotificationId),
    /// Clear the group.
    Clear(AppKey),
}

/// The rows of `views`, in their order.
pub fn rows_of(views: &[GroupView]) -> Vec<CenterRow> {
    views.iter().flat_map(group_rows).collect()
}

fn group_rows(view: &GroupView) -> Vec<CenterRow> {
    let stack = match view.more {
        0 => Stack::Alone,
        more => Stack::Stacked(more + view.shown.len()),
    };
    let notes = view.shown.iter().map(|&id| CenterRow::Note {
        group: view.key.clone(),
        id,
        stack,
    });
    let fold = match (view.more, view.shown.len()) {
        (0, 0 | 1) => None,
        (0, _) => Some(Fold::Less),
        (more, _) => Some(Fold::More(more)),
    };
    let fold = fold.map(|fold| CenterRow::Fold {
        group: view.key.clone(),
        fold,
    });
    notes.chain(fold).collect()
}

/// Return, or a press on the row's body: a lone card's `default`, a stack or a fold line opens
/// or closes its group.
pub fn activate(row: &CenterRow) -> RowAct {
    match row {
        CenterRow::Note {
            id,
            stack: Stack::Alone,
            ..
        } => RowAct::Invoke(*id, ActionKey::default_action()),
        CenterRow::Note { group, .. } | CenterRow::Fold { group, .. } => {
            RowAct::Toggle(group.clone())
        }
    }
}

/// Delete, or a card's close: a lone card is dismissed, a stack or a fold line clears its
/// group.
pub fn dismiss(row: &CenterRow) -> RowAct {
    match row {
        CenterRow::Note {
            id,
            stack: Stack::Alone,
            ..
        } => RowAct::Dismiss(*id),
        CenterRow::Note { group, .. } | CenterRow::Fold { group, .. } => {
            RowAct::Clear(group.clone())
        }
    }
}

/// What a press on a card asks; a link in the body asks nothing of the center (FINDINGS F321).
pub fn press(row: &CenterRow, press: CardPress) -> Option<RowAct> {
    match (row, press) {
        (_, CardPress::Body) => Some(activate(row)),
        (_, CardPress::Close) => Some(dismiss(row)),
        (CenterRow::Note { id, .. }, CardPress::Action(key)) => Some(RowAct::Invoke(*id, key)),
        (CenterRow::Fold { .. }, CardPress::Action(_)) | (_, CardPress::Link(_)) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{CenterRow, Fold, RowAct, Stack, activate, dismiss, press, rows_of};
    use crate::surfaces::notification_card::CardPress;
    use crate::surfaces::notification_center::GroupView;
    use sill_services::{ActionKey, AppKey, Href, NotificationId};

    fn key(k: &str) -> AppKey {
        AppKey(k.to_owned())
    }

    fn view(k: &str, shown: &[u32], more: usize) -> GroupView {
        GroupView {
            key: key(k),
            shown: shown.iter().map(|&id| NotificationId(id)).collect(),
            more,
        }
    }

    fn note(k: &str, id: u32, stack: Stack) -> CenterRow {
        CenterRow::Note {
            group: key(k),
            id: NotificationId(id),
            stack,
        }
    }

    fn fold(k: &str, fold: Fold) -> CenterRow {
        CenterRow::Fold {
            group: key(k),
            fold,
        }
    }

    #[test]
    fn groups_become_cards_and_fold_lines() {
        let cases = [
            (vec![], vec![]),
            (vec![view("a", &[4], 0)], vec![note("a", 4, Stack::Alone)]),
            (
                vec![view("a", &[5], 2), view("b", &[4], 0)],
                vec![
                    note("a", 5, Stack::Stacked(3)),
                    fold("a", Fold::More(2)),
                    note("b", 4, Stack::Alone),
                ],
            ),
            (
                vec![view("a", &[5, 3], 0)],
                vec![
                    note("a", 5, Stack::Alone),
                    note("a", 3, Stack::Alone),
                    fold("a", Fold::Less),
                ],
            ),
        ];
        for (views, want) in cases {
            assert_eq!(rows_of(&views), want, "{views:?}");
        }
        assert_eq!(Stack::Stacked(3).count(), 3);
        assert_eq!(Stack::Alone.count(), 1);
    }

    #[test]
    fn rows_open_invoke_dismiss_and_clear() {
        let id = NotificationId(5);
        let alone = note("a", 5, Stack::Alone);
        let stacked = note("a", 5, Stack::Stacked(3));
        let more = fold("a", Fold::More(2));
        let default = ActionKey::default_action();
        let cases = [
            (
                &alone,
                RowAct::Invoke(id, default.clone()),
                RowAct::Dismiss(id),
            ),
            (&stacked, RowAct::Toggle(key("a")), RowAct::Clear(key("a"))),
            (&more, RowAct::Toggle(key("a")), RowAct::Clear(key("a"))),
        ];
        for (row, on_return, on_delete) in cases {
            assert_eq!(activate(row), on_return, "{row:?}");
            assert_eq!(dismiss(row), on_delete, "{row:?}");
        }
        let reply = ActionKey("reply".to_owned());
        let presses = [
            (&alone, CardPress::Body, Some(RowAct::Invoke(id, default))),
            (&alone, CardPress::Close, Some(RowAct::Dismiss(id))),
            (
                &stacked,
                CardPress::Action(reply.clone()),
                Some(RowAct::Invoke(id, reply.clone())),
            ),
            (&more, CardPress::Action(reply), None),
            (&alone, CardPress::Link(Href("https://x".to_owned())), None),
        ];
        for (row, card, want) in presses {
            assert_eq!(press(row, card.clone()), want, "{row:?} {card:?}");
        }
    }
}
