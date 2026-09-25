//! The center's words, pure: a group's name, its fold line, and the footer that says when sill
//! is not the one receiving notifications.

use sill_services::{AppGroup, NameHolder, ServerMode, ServerStatus};

use super::rows::Fold;

/// The column's title.
pub const TITLE: &str = "Notifications";

/// What an empty history says.
pub const EMPTY: &str = "No Notifications";

/// A group's name: its desktop entry's `Name` when the apps index knows it, else the newest
/// sender's `app_name`, else "Notification". lane b's `app_label` says the same for a card
/// (FINDINGS F322); this is the group's, from the group's newest notification.
pub fn group_title(group: &AppGroup, entry_name: Option<&str>) -> String {
    let sent = group.notes.first().map(|note| note.app.name.as_str());
    [entry_name, sent]
        .into_iter()
        .flatten()
        .find(|name| !name.trim().is_empty())
        .unwrap_or("Notification")
        .to_owned()
}

/// The fold line's words.
pub fn fold_label(fold: Fold) -> String {
    match fold {
        Fold::More(n) => format!("{n} more"),
        Fold::Less => "Show less".to_owned(),
    }
}

/// The footer, when sill is not receiving notifications: another daemon holds the name, or
/// `notifications.server` is passive.
pub fn server_line(server: &ServerStatus) -> Option<String> {
    match (&server.holder, server.mode) {
        (NameHolder::Other(owner), _) => Some(format!(
            "Notifications are shown by {}",
            owner.command.as_deref().unwrap_or("another app")
        )),
        (_, ServerMode::Passive) => Some("Not receiving notifications from apps".to_owned()),
        (NameHolder::NoBus | NameHolder::Nobody | NameHolder::Sill, ServerMode::Own) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{fold_label, group_title, server_line};
    use crate::surfaces::notification_center::rows::Fold;
    use sill_services::{
        AppGroup, AppKey, AppRef, Body, Expiry, Hints, IconRef, NameHolder, NameOwner,
        Notification, NotificationId, ServerMode, ServerStatus, UnixMs,
    };

    fn group(sent: &str) -> AppGroup {
        AppGroup {
            key: AppKey(sent.to_owned()),
            notes: vec![Notification {
                id: NotificationId(1),
                arrived: UnixMs(0),
                app: AppRef {
                    name: sent.to_owned(),
                    icon: IconRef::None,
                    desktop_entry: None,
                },
                summary: String::new(),
                body: Body::default(),
                actions: Vec::new(),
                hints: Hints::default(),
                expiry: Expiry::Default,
            }],
        }
    }

    #[test]
    fn a_group_is_named_by_its_entry_then_its_sender() {
        assert_eq!(
            group_title(&group("evolution"), Some("Evolution")),
            "Evolution"
        );
        assert_eq!(group_title(&group("evolution"), None), "evolution");
        assert_eq!(group_title(&group("evolution"), Some(" ")), "evolution");
        assert_eq!(group_title(&group(""), None), "Notification");
        assert_eq!(fold_label(Fold::More(2)), "2 more");
        assert_eq!(fold_label(Fold::Less), "Show less");
    }

    #[test]
    fn the_footer_names_whoever_else_shows_notifications() {
        let other = |command: Option<&str>| {
            NameHolder::Other(NameOwner {
                unique: ":1.9".to_owned(),
                pid: Some(9),
                command: command.map(str::to_owned),
            })
        };
        let cases = [
            (ServerMode::Own, NameHolder::Sill, None),
            (ServerMode::Own, NameHolder::Nobody, None),
            (
                ServerMode::Own,
                other(Some("plasmashell")),
                Some("Notifications are shown by plasmashell"),
            ),
            (
                ServerMode::Passive,
                other(None),
                Some("Notifications are shown by another app"),
            ),
            (
                ServerMode::Passive,
                NameHolder::NoBus,
                Some("Not receiving notifications from apps"),
            ),
        ];
        for (mode, holder, want) in cases {
            let status = ServerStatus { mode, holder };
            assert_eq!(server_line(&status).as_deref(), want, "{status:?}");
        }
    }
}
