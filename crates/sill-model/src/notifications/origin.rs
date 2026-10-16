//! Whose a notification is. The shell's own notices (accounts, sync hold, keycap, guest asks) carry
//! a desktop entry in the `sill-` namespace; the D-Bus boundary drops that namespace from an app's
//! hint, so an entry in it proves the notice was posted inside the shell.

use super::content::AppRef;

/// The namespace of the shell's own desktop entries.
pub const SYSTEM_ENTRY_PREFIX: &str = "sill-";

/// Who posted a notification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Origin {
    /// The shell itself: its alarms may pass Do Not Disturb and persist.
    System,
    /// Any app on the bus.
    App,
}

impl Origin {
    /// The origin `app` claims; it is proof only for what the boundary let through.
    pub fn of(app: &AppRef) -> Origin {
        match app.desktop_entry.as_deref() {
            Some(entry) if entry.starts_with(SYSTEM_ENTRY_PREFIX) => Origin::System,
            _ => Origin::App,
        }
    }
}

/// An app's `desktop-entry` hint, `None` when it is in the shell's own namespace.
pub fn app_entry(entry: String) -> Option<String> {
    (!entry.starts_with(SYSTEM_ENTRY_PREFIX)).then_some(entry)
}

#[cfg(test)]
mod tests {
    use super::{Origin, app_entry};
    use crate::notifications::{AppRef, IconRef};

    fn app(entry: Option<&str>) -> AppRef {
        AppRef {
            name: "Accounts".to_owned(),
            icon: IconRef::None,
            desktop_entry: entry.map(str::to_owned),
        }
    }

    #[test]
    fn only_the_shells_namespace_is_the_system() {
        const CASES: &[(Option<&str>, Origin)] = &[
            (Some("sill-accounts"), Origin::System),
            (Some("sill-keycap"), Origin::System),
            (Some("org.example.Mail"), Origin::App),
            (Some("sill"), Origin::App),
            (Some("not-sill-accounts"), Origin::App),
            (None, Origin::App),
        ];
        for &(entry, want) in CASES {
            assert_eq!(Origin::of(&app(entry)), want, "{entry:?}");
        }
    }

    #[test]
    fn an_app_cannot_claim_the_shells_namespace() {
        assert_eq!(app_entry("sill-accounts".to_owned()), None);
        assert_eq!(
            app_entry("org.example.Mail".to_owned()).as_deref(),
            Some("org.example.Mail")
        );
    }
}
