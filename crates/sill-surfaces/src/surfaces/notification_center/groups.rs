//! The center's list (design/13 §13.3.6 "Grouping by app": "the history groups by app, newest
//! first, collapsed to the newest with 'N more'"): which rows each group shows, and which groups
//! the person has opened.

use sill_services::{AppKey, History, NotificationId};

/// The groups the person has opened, in the order they opened them.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Expanded(pub Vec<AppKey>);

/// One group as the center draws it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupView {
    pub key: AppKey,
    /// The rows drawn, newest first: only the newest while collapsed, every one when opened.
    pub shown: Vec<NotificationId>,
    /// The "N more" line's N: how many a collapsed group hides (0: no line).
    pub more: usize,
}

/// `history`'s groups, in its order, each collapsed unless `expanded` names it.
pub fn group_views(history: &History, expanded: &Expanded) -> Vec<GroupView> {
    let _ = (history, expanded);
    todo!("M6 fill: c-center")
}

/// `expanded` with `key` opened when it was closed, closed when it was open.
pub fn toggle_group(expanded: Expanded, key: &AppKey) -> Expanded {
    let _ = (expanded, key);
    todo!("M6 fill: c-center")
}

#[cfg(test)]
mod tests {
    use super::{Expanded, GroupView, group_views, toggle_group};
    use sill_services::{
        AppGroup, AppKey, AppRef, Body, Expiry, Hints, History, IconRef, Notification,
        NotificationId, UnixMs,
    };

    fn note(id: u32) -> Notification {
        Notification {
            id: NotificationId(id),
            arrived: UnixMs(i64::from(id)),
            app: AppRef {
                name: String::new(),
                icon: IconRef::None,
                desktop_entry: None,
            },
            summary: String::new(),
            body: Body::default(),
            actions: Vec::new(),
            hints: Hints::default(),
            expiry: Expiry::Default,
        }
    }

    fn key(k: &str) -> AppKey {
        AppKey(k.to_owned())
    }

    fn history() -> History {
        History {
            groups: vec![
                AppGroup {
                    key: key("a"),
                    notes: vec![note(5), note(3), note(1)],
                },
                AppGroup {
                    key: key("b"),
                    notes: vec![note(4)],
                },
            ],
        }
    }

    fn view(k: &str, shown: &[u32], more: usize) -> GroupView {
        GroupView {
            key: key(k),
            shown: shown.iter().map(|&id| NotificationId(id)).collect(),
            more,
        }
    }

    #[test]
    #[ignore = "M6 fill: c-center"]
    fn groups_collapse_to_the_newest_until_opened() {
        let cases = [
            (
                Expanded::default(),
                vec![view("a", &[5], 2), view("b", &[4], 0)],
            ),
            (
                Expanded(vec![key("a")]),
                vec![view("a", &[5, 3, 1], 0), view("b", &[4], 0)],
            ),
            (
                Expanded(vec![key("gone")]),
                vec![view("a", &[5], 2), view("b", &[4], 0)],
            ),
        ];
        for (expanded, want) in cases {
            assert_eq!(group_views(&history(), &expanded), want, "{expanded:?}");
        }
        assert_eq!(
            group_views(&History::default(), &Expanded::default()),
            vec![]
        );
    }

    #[test]
    #[ignore = "M6 fill: c-center"]
    fn a_group_opens_and_closes() {
        let open = toggle_group(Expanded::default(), &key("a"));
        assert_eq!(open, Expanded(vec![key("a")]));
        let both = toggle_group(open, &key("b"));
        assert_eq!(both, Expanded(vec![key("a"), key("b")]));
        assert_eq!(toggle_group(both, &key("a")), Expanded(vec![key("b")]));
    }
}
