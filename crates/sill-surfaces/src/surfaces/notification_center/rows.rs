//! The rows the center draws, in order, and what a press, Return or Delete on one asks for,
//! pure. Each group is its header (quire's `GroupHeader`: the app, the count, "N more" or "Show
//! less", Clear) and then its cards: a collapsed group of several is its newest card stacked over
//! the rest (macOS: a press on the stack opens it, its close clears the group).

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

/// One row the keys move over.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CenterRow {
    /// A group's header.
    Head { group: AppKey },
    Note {
        group: AppKey,
        id: NotificationId,
        stack: Stack,
    },
}

/// What a row is, for the center's `LeavingList`: a header by its group, a card by its
/// notification. Not by its `Stack`, which changes as the group folds, so the newest card stays
/// put when a group opens or closes.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RowKey {
    Head(AppKey),
    Card(NotificationId),
}

impl CenterRow {
    /// The row's key in the list.
    pub fn key(&self) -> RowKey {
        match self {
            CenterRow::Head { group } => RowKey::Head(group.clone()),
            CenterRow::Note { id, .. } => RowKey::Card(*id),
        }
    }

    /// The group the row belongs to.
    pub fn group(&self) -> &AppKey {
        match self {
            CenterRow::Head { group } | CenterRow::Note { group, .. } => group,
        }
    }

    /// How many a card's count chip says; a header counts one.
    pub fn stack_count(&self) -> usize {
        match self {
            CenterRow::Note { stack, .. } => stack.count(),
            CenterRow::Head { .. } => 1,
        }
    }
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
    let head = CenterRow::Head {
        group: view.key.clone(),
    };
    let notes = view.shown.iter().map(|&id| CenterRow::Note {
        group: view.key.clone(),
        id,
        stack,
    });
    std::iter::once(head).chain(notes).collect()
}

/// Return, or a press on the row's body: a lone card's `default`; a stack or a header opens or
/// closes its group.
pub fn activate(row: &CenterRow) -> RowAct {
    match row {
        CenterRow::Note {
            id,
            stack: Stack::Alone,
            ..
        } => RowAct::Invoke(*id, ActionKey::default_action()),
        CenterRow::Note { group, .. } | CenterRow::Head { group } => RowAct::Toggle(group.clone()),
    }
}

/// Delete, or a card's close: a lone card is dismissed; a stack or a header clears its group.
pub fn dismiss(row: &CenterRow) -> RowAct {
    match row {
        CenterRow::Note {
            id,
            stack: Stack::Alone,
            ..
        } => RowAct::Dismiss(*id),
        CenterRow::Note { group, .. } | CenterRow::Head { group } => RowAct::Clear(group.clone()),
    }
}

/// What a press on a card asks; a link in the body asks nothing of the center.
pub fn press(row: &CenterRow, press: CardPress) -> Option<RowAct> {
    match (row, press) {
        (_, CardPress::Body) => Some(activate(row)),
        (_, CardPress::Close) => Some(dismiss(row)),
        (CenterRow::Note { id, .. }, CardPress::Action(key)) => Some(RowAct::Invoke(*id, key)),
        (CenterRow::Head { .. }, CardPress::Action(_)) | (_, CardPress::Link(_)) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{CenterRow, RowAct, RowKey, Stack, activate, dismiss, press, rows_of};
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

    fn head(k: &str) -> CenterRow {
        CenterRow::Head { group: key(k) }
    }

    #[test]
    fn groups_become_a_header_and_cards() {
        let cases = [
            (vec![], vec![]),
            (
                vec![view("a", &[4], 0)],
                vec![head("a"), note("a", 4, Stack::Alone)],
            ),
            (
                vec![view("a", &[5], 2), view("b", &[4], 0)],
                vec![
                    head("a"),
                    note("a", 5, Stack::Stacked(3)),
                    head("b"),
                    note("b", 4, Stack::Alone),
                ],
            ),
            (
                vec![view("a", &[5, 3], 0)],
                vec![
                    head("a"),
                    note("a", 5, Stack::Alone),
                    note("a", 3, Stack::Alone),
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
    fn a_row_is_keyed_by_what_it_is_not_how_it_stacks() {
        let folded = note("a", 5, Stack::Stacked(3));
        let opened = note("a", 5, Stack::Alone);
        assert_eq!(folded.key(), opened.key());
        assert_eq!(folded.key(), RowKey::Card(NotificationId(5)));
        assert_eq!(head("a").key(), RowKey::Head(key("a")));
        assert_ne!(head("a").key(), head("b").key());
        assert_eq!(folded.group(), &key("a"));
        assert_eq!(folded.stack_count(), 3);
        assert_eq!(head("a").stack_count(), 1);
    }

    #[test]
    fn rows_open_invoke_dismiss_and_clear() {
        let id = NotificationId(5);
        let alone = note("a", 5, Stack::Alone);
        let stacked = note("a", 5, Stack::Stacked(3));
        let header = head("a");
        let default = ActionKey::default_action();
        let cases = [
            (
                &alone,
                RowAct::Invoke(id, default.clone()),
                RowAct::Dismiss(id),
            ),
            (&stacked, RowAct::Toggle(key("a")), RowAct::Clear(key("a"))),
            (&header, RowAct::Toggle(key("a")), RowAct::Clear(key("a"))),
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
            (&header, CardPress::Action(reply), None),
            (&alone, CardPress::Link(Href("https://x".to_owned())), None),
        ];
        for (row, card, want) in presses {
            assert_eq!(press(row, card.clone()), want, "{row:?} {card:?}");
        }
    }
}
