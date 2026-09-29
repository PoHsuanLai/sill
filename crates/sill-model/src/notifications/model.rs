//! What the service publishes, on two receivers: [`Notifications`] (who holds the bus name and
//! the live banners, which the banner surfaces follow) and [`History`] (every kept notification
//! grouped by app, which the notification center follows). Two channels, as Wi-Fi is apart from
//! `Network`: a banner's arrival and departure should not repaint a closed center, and a history
//! clear should not repaint the banners.

use serde::{Deserialize, Serialize};

use super::content::Notification;
use super::ids::{AppKey, NotificationId};

/// Who holds `org.freedesktop.Notifications` and the banners on screen.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Notifications {
    pub server: ServerStatus,
    /// Every notification presenting a banner, newest first. The banner surface shows the first
    /// `notifications.stack_max` cards (grouped by app) and the rest wait their turn.
    pub banners: Vec<LiveBanner>,
}

impl Notifications {
    /// The live banner for `id`.
    pub fn banner(&self, id: NotificationId) -> Option<&LiveBanner> {
        self.banners.iter().find(|banner| banner.note.id == id)
    }
}

/// One notification presenting a banner.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LiveBanner {
    pub note: Notification,
    pub life: BannerLife,
}

/// Whether a banner leaves by itself (design/13 §13.3.6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BannerLife {
    /// A banner: `notifications.banner_hold_ms` after it has entered, paused while hovered.
    #[default]
    Expires,
    /// An alert: urgency critical, `resident`, `expire_timeout == 0`, or the Alert style. Stays
    /// until acted on or closed.
    Persists,
}

/// The history: every kept notification, grouped by app. Groups are ordered by their newest
/// notification, newest first; a group's notifications are newest first; no group is empty.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct History {
    pub groups: Vec<AppGroup>,
}

impl History {
    /// How many notifications the history holds.
    pub fn len(&self) -> usize {
        self.groups.iter().map(|group| group.notes.len()).sum()
    }

    /// Whether the history holds nothing.
    pub fn is_empty(&self) -> bool {
        self.groups.is_empty()
    }

    /// The kept notification `id`.
    pub fn find(&self, id: NotificationId) -> Option<&Notification> {
        self.groups
            .iter()
            .flat_map(|group| group.notes.iter())
            .find(|note| note.id == id)
    }

    /// `key`'s group.
    pub fn group(&self, key: &AppKey) -> Option<&AppGroup> {
        self.groups.iter().find(|group| group.key == *key)
    }
}

/// One app's kept notifications, newest first (never empty).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppGroup {
    pub key: AppKey,
    pub notes: Vec<Notification>,
}

/// Whether the service tries to own the bus name (`notifications.server`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServerMode {
    /// Ask for the name (never replacing a holder), and ask again whenever it falls free.
    Own,
    /// Never ask: another daemon (cosmic-notifications, mako) serves notifications. The service
    /// still reports who holds the name, and `sill debug notify` still shows banners.
    #[default]
    Passive,
}

/// The name, and who holds it.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ServerStatus {
    /// What the service was told to do (passive until the daemon's first policy).
    pub mode: ServerMode,
    pub holder: NameHolder,
}

/// Who holds `org.freedesktop.Notifications` on the bus the service is connected to.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum NameHolder {
    /// No bus (not connected yet, or none reachable).
    #[default]
    NoBus,
    /// Nobody: notifications sent now are lost (or D-Bus-activate another daemon).
    Nobody,
    /// This service.
    Sill,
    /// Another connection.
    Other(NameOwner),
}

/// Another holder of the name, as the bus describes it.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NameOwner {
    /// Its unique connection name (`:1.42`).
    pub unique: String,
    /// `GetConnectionUnixProcessID`, when the bus answered.
    pub pid: Option<u32>,
    /// `/proc/<pid>/comm` (`plasmashell`, `cosmic-notifica`), when readable.
    pub command: Option<String>,
}

/// Everything the service holds: what it publishes on its two receivers, and the last id it
/// handed out.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct NotesState {
    pub live: Notifications,
    pub history: History,
    pub last_id: NotificationId,
}
