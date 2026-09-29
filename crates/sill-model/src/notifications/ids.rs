//! The identifiers and units a notification carries: its id, the app it groups under, an
//! action's key, an activation token and the wall-clock time it arrived.

use serde::{Deserialize, Serialize};

/// A notification's id as the server hands it out: never 0 (the spec reserves 0 for "no
/// replacement"), counting up, and not reused while the history holds the old one.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct NotificationId(pub u32);

impl NotificationId {
    /// The id a `Notify` with `replaces_id` names: `None` for the spec's 0.
    pub fn replacing(wire: u32) -> Option<NotificationId> {
        (wire != 0).then_some(NotificationId(wire))
    }
}

/// What notifications group under, in the banner stack and in the history: the sender's
/// `desktop-entry` hint when it gave one (`org.gnome.Evolution`), else its `app_name` as sent.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AppKey(pub String);

/// An action's key as the sender named it; `default` is the body's click.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ActionKey(pub String);

impl ActionKey {
    /// The key the spec reserves for activating the notification itself.
    pub const DEFAULT: &'static str = "default";

    /// The body's click.
    pub fn default_action() -> ActionKey {
        ActionKey(ActionKey::DEFAULT.to_owned())
    }

    /// Whether this is the spec's `default` action, compared whole.
    pub fn is_default(&self) -> bool {
        self.0 == ActionKey::DEFAULT
    }
}

/// An xdg-activation token, handed to the sender (`ActivationToken`) so the window it raises
/// takes focus.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ActivationToken(pub String);

/// Whether an invoked action carries an activation token.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum Activation {
    /// The surface got one from the compositor: `ActivationToken` is emitted first.
    Token(ActivationToken),
    /// No xdg-activation, or the compositor refused: only `ActionInvoked`.
    #[default]
    NoToken,
}

/// Wall-clock milliseconds since the Unix epoch: when a notification arrived. Persisted in the
/// history, so not the daemon's monotonic `Stamp`.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct UnixMs(pub i64);

impl UnixMs {
    /// Milliseconds from `earlier` to `self`, zero if `earlier` is later.
    pub fn since(self, earlier: UnixMs) -> u64 {
        u64::try_from(self.0.saturating_sub(earlier.0)).unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::{ActionKey, NotificationId, UnixMs};

    #[test]
    fn zero_replaces_nothing() {
        const CASES: &[(u32, Option<NotificationId>)] = &[
            (0, None),
            (1, Some(NotificationId(1))),
            (u32::MAX, Some(NotificationId(u32::MAX))),
        ];
        for &(wire, want) in CASES {
            assert_eq!(NotificationId::replacing(wire), want, "{wire}");
        }
    }

    #[test]
    fn only_the_whole_word_default_is_the_default_action() {
        const CASES: &[(&str, bool)] = &[
            ("default", true),
            ("Default", false),
            ("default ", false),
            ("defaults", false),
            ("reply", false),
        ];
        for &(key, want) in CASES {
            assert_eq!(ActionKey(key.to_owned()).is_default(), want, "{key:?}");
        }
        assert!(ActionKey::default_action().is_default());
    }

    #[test]
    fn elapsed_time_never_goes_negative() {
        assert_eq!(UnixMs(5_000).since(UnixMs(2_000)), 3_000);
        assert_eq!(UnixMs(2_000).since(UnixMs(5_000)), 0);
    }
}
