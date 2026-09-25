//! How a notification is presented: the daemon's settings and focus mode as plain values (this
//! crate does not read `sill-settings`: `scripts/check-boundary.sh`), and the pure decisions
//! they drive.
//!
//! The daemon sends a [`NotificationsPolicy`] at start and whenever `notifications.*` or the
//! focus mode changes (`NotificationsCommand::Policy`). Until the first one the service is
//! passive and presents with the defaults below: a test that spawns the service never asks for
//! the bus name.

use serde::{Deserialize, Serialize};

use super::content::{Expiry, Notification, NotifyUrgency, Residency, SoundHint};
use super::state::{BannerLife, ServerMode};

/// Everything the service is told by the daemon.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct NotificationsPolicy {
    /// `notifications.server`.
    pub server: ServerMode,
    /// The focus mode (`notifications.dnd`).
    pub quiet: Quiet,
    /// `notifications.banner_style`.
    pub style: Presentation,
    /// `notifications.history_cap`.
    pub history_cap: HistoryCap,
    /// `notifications.swipe`.
    pub swipe: Swipe,
}

/// `notifications.swipe`: what swiping a banner away does to its notification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Swipe {
    /// The banner leaves; the notification stays in the notification center (macOS).
    #[default]
    KeepInCenter,
    /// The notification is dismissed: gone from the history too.
    Dismiss,
}

/// Whether banners and sounds are held back (design/13 §13.3.6 "Do Not Disturb").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Quiet {
    #[default]
    Off,
    /// No banners and no sounds, except urgency critical; everything still enters the history.
    DoNotDisturb,
}

/// `notifications.banner_style`: how an app's notifications show by default.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Presentation {
    /// A banner that leaves by itself.
    #[default]
    Banner,
    /// A banner that stays until acted on.
    Alert,
    /// No banner: the history only.
    HistoryOnly,
}

/// The most notifications the history keeps; the oldest go first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct HistoryCap(pub u16);

impl Default for HistoryCap {
    /// `notifications.history_cap`'s default.
    fn default() -> Self {
        HistoryCap(100)
    }
}

/// How one notification shows on arrival.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum Presented {
    Banner(BannerLife),
    HistoryOnly,
}

/// Whether a notification's arrival makes a sound.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum Chime {
    Play(SoundHint),
    Silent,
}

/// How `note` shows under `policy`: a critical one always as a persisting banner (through Do
/// Not Disturb and a history-only style); otherwise nothing but the history while Do Not
/// Disturb is on or the style is history-only; else a banner that persists for `resident`,
/// `expire_timeout == 0` or the Alert style, and expires otherwise.
pub fn presented(note: &Notification, policy: &NotificationsPolicy) -> Presented {
    let critical = note.hints.urgency == NotifyUrgency::Critical;
    let held_back =
        policy.quiet == Quiet::DoNotDisturb || policy.style == Presentation::HistoryOnly;
    let persists = note.hints.residency == Residency::Resident
        || note.expiry == Expiry::Never
        || policy.style == Presentation::Alert;
    match (critical, held_back, persists) {
        (true, _, _) => Presented::Banner(BannerLife::Persists),
        (false, true, _) => Presented::HistoryOnly,
        (false, false, true) => Presented::Banner(BannerLife::Persists),
        (false, false, false) => Presented::Banner(BannerLife::Expires),
    }
}

/// Whether `note`'s arrival plays a sound: never when the sender suppressed it, nor while Do
/// Not Disturb holds it back (a critical one still sounds); else its hint (or the default).
pub fn chime(note: &Notification, policy: &NotificationsPolicy) -> Chime {
    let critical = note.hints.urgency == NotifyUrgency::Critical;
    match (&note.hints.sound, policy.quiet) {
        (SoundHint::Suppressed, _) => Chime::Silent,
        (_, Quiet::DoNotDisturb) if !critical => Chime::Silent,
        (hint, _) => Chime::Play(hint.clone()),
    }
}

#[cfg(test)]
mod tests {
    use super::{Chime, NotificationsPolicy, Presentation, Presented, Quiet, chime, presented};
    use crate::notifications::content::{
        AppRef, Expiry, Hints, IconRef, Notification, NotifyUrgency, Residency, SoundHint,
    };
    use crate::notifications::ids::{NotificationId, UnixMs};
    use crate::notifications::markup::Body;
    use crate::notifications::state::BannerLife;

    fn note(urgency: NotifyUrgency, residency: Residency, expiry: Expiry) -> Notification {
        Notification {
            id: NotificationId(1),
            arrived: UnixMs(0),
            app: AppRef {
                name: "app".to_owned(),
                icon: IconRef::None,
                desktop_entry: None,
            },
            summary: "s".to_owned(),
            body: Body::default(),
            actions: Vec::new(),
            hints: Hints {
                urgency,
                residency,
                ..Hints::default()
            },
            expiry,
        }
    }

    fn policy(quiet: Quiet, style: Presentation) -> NotificationsPolicy {
        NotificationsPolicy {
            quiet,
            style,
            ..NotificationsPolicy::default()
        }
    }

    #[test]
    fn presentation_follows_urgency_focus_and_style() {
        use BannerLife::{Expires, Persists};
        use NotifyUrgency::{Critical, Low, Normal};
        use Presentation::{Alert, Banner, HistoryOnly};
        use Quiet::{DoNotDisturb, Off};
        use Residency::{ClosesOnAction, Resident};
        let plain = |u| note(u, ClosesOnAction, Expiry::Default);
        let cases = [
            (
                "normal",
                plain(Normal),
                policy(Off, Banner),
                Presented::Banner(Expires),
            ),
            (
                "low",
                plain(Low),
                policy(Off, Banner),
                Presented::Banner(Expires),
            ),
            (
                "app asks 3 s",
                note(Normal, ClosesOnAction, Expiry::Ms(3000)),
                policy(Off, Banner),
                Presented::Banner(Expires),
            ),
            (
                "expire 0",
                note(Normal, ClosesOnAction, Expiry::Never),
                policy(Off, Banner),
                Presented::Banner(Persists),
            ),
            (
                "resident",
                note(Normal, Resident, Expiry::Default),
                policy(Off, Banner),
                Presented::Banner(Persists),
            ),
            (
                "alert style",
                plain(Normal),
                policy(Off, Alert),
                Presented::Banner(Persists),
            ),
            (
                "critical",
                plain(Critical),
                policy(Off, Banner),
                Presented::Banner(Persists),
            ),
            (
                "dnd",
                plain(Normal),
                policy(DoNotDisturb, Banner),
                Presented::HistoryOnly,
            ),
            (
                "dnd alert",
                plain(Normal),
                policy(DoNotDisturb, Alert),
                Presented::HistoryOnly,
            ),
            (
                "dnd critical",
                plain(Critical),
                policy(DoNotDisturb, Banner),
                Presented::Banner(Persists),
            ),
            (
                "history only",
                plain(Normal),
                policy(Off, HistoryOnly),
                Presented::HistoryOnly,
            ),
            (
                "history only critical",
                plain(Critical),
                policy(Off, HistoryOnly),
                Presented::Banner(Persists),
            ),
        ];
        for (name, note, policy, want) in cases {
            assert_eq!(presented(&note, &policy), want, "{name}");
        }
    }

    #[test]
    fn sounds_follow_the_hint_and_do_not_disturb() {
        let with_sound = |urgency, sound: SoundHint| {
            let mut n = note(urgency, Residency::ClosesOnAction, Expiry::Default);
            n.hints.sound = sound;
            n
        };
        let named = SoundHint::Named("message-new-email".to_owned());
        let cases = [
            (
                "default",
                with_sound(NotifyUrgency::Normal, SoundHint::Default),
                Quiet::Off,
                Chime::Play(SoundHint::Default),
            ),
            (
                "named",
                with_sound(NotifyUrgency::Normal, named.clone()),
                Quiet::Off,
                Chime::Play(named.clone()),
            ),
            (
                "suppressed",
                with_sound(NotifyUrgency::Normal, SoundHint::Suppressed),
                Quiet::Off,
                Chime::Silent,
            ),
            (
                "dnd",
                with_sound(NotifyUrgency::Normal, named.clone()),
                Quiet::DoNotDisturb,
                Chime::Silent,
            ),
            (
                "dnd critical",
                with_sound(NotifyUrgency::Critical, named.clone()),
                Quiet::DoNotDisturb,
                Chime::Play(named),
            ),
            (
                "critical suppressed",
                with_sound(NotifyUrgency::Critical, SoundHint::Suppressed),
                Quiet::DoNotDisturb,
                Chime::Silent,
            ),
        ];
        for (name, note, quiet, want) in cases {
            assert_eq!(
                chime(&note, &policy(quiet, Presentation::Banner)),
                want,
                "{name}"
            );
        }
    }
}
