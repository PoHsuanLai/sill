//! The center's words, pure: its title, the empty state, and the footer that says when sill is
//! not the one receiving notifications. A group's name is the card's (`use_app_mark`).

use sill_model::notifications::{NameHolder, ServerMode, ServerStatus};

/// The column's title.
pub const TITLE: &str = "Notifications";

/// What an empty history says.
pub const EMPTY: &str = "No Notifications";

/// The footer, when sill is not receiving notifications: another daemon holds the name, or
/// the service has not been configured yet (passive).
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
    use super::server_line;
    use sill_model::notifications::{NameHolder, NameOwner, ServerMode, ServerStatus};

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
