//! What the notifications service hears and what it can be told.

use serde::{Deserialize, Serialize};

use super::{
    ActionKey, Activation, AppKey, Notification, NotificationId, NotificationsPolicy, Posting,
    ServerStatus, SoundHint, StoredHistory,
};

/// What the notifications service can be told: by the daemon (the policy, a debug post), by the
/// banners and the center (the person's actions), and by `sill notifications`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotificationsCommand {
    /// The settings and focus mode now in effect; the first one ends the passive start.
    Policy(NotificationsPolicy),
    /// A notification as if `Notify` had been called (`sill debug notify`); no reply id.
    Post(Box<Posting>),
    /// The person dismissed it (the close button, a history row's clear).
    Dismiss(NotificationId),
    /// The person swiped or dragged its banner away: kept in the center or dismissed, as
    /// `notifications.swipe` says ([`Swipe`]).
    SwipeAway(NotificationId),
    /// Dismiss every live banner.
    DismissAll,
    /// A banner's hold ran out.
    BannerTimedOut(NotificationId),
    /// The person chose `action` (`default`: the body was clicked).
    InvokeAction {
        id: NotificationId,
        action: ActionKey,
        token: Activation,
    },
    /// The center's Clear.
    ClearHistory(Clear),
}

/// One thing that happened to the service.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum NotificationsEvent {
    /// A `Notify` (or `sill debug notify`), with the id [`assign_id`] gave it.
    Posted(Box<Notification>),
    /// `CloseNotification`.
    ClosedByApp(NotificationId),
    /// The person dismissed it: the close button, a row's clear (a swipe under
    /// `Swipe::Dismiss`).
    Dismissed(NotificationId),
    /// The person swiped its banner away: under `Swipe::KeepInCenter` the banner leaves and the
    /// notification stays in the history (`Closed` with `Dismissed`: the banner is gone); under
    /// `Swipe::Dismiss`, as `Dismissed`.
    SwipedAway(NotificationId),
    /// Every banner dismissed at once.
    DismissedAll,
    /// A banner's hold ran out (the banner surface counts it, pausing while hovered).
    BannerTimedOut(NotificationId),
    /// The person chose an action (`default` for the body).
    Invoked {
        id: NotificationId,
        action: ActionKey,
        token: Activation,
    },
    HistoryCleared(Clear),
    /// The history file, read at start.
    Loaded(StoredHistory),
    Server(ServerStatus),
}

/// Which history to clear.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum Clear {
    All,
    App(AppKey),
}

/// Something the live loop must do.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "v", rename_all = "snake_case")]
pub enum NotificationsEffect {
    /// Emit `NotificationClosed(id, reason)`.
    Closed {
        id: NotificationId,
        reason: CloseReason,
    },
    /// Emit `ActivationToken(id, token)` when there is one, then `ActionInvoked(id, action)`.
    Invoked {
        id: NotificationId,
        action: ActionKey,
        token: Activation,
    },
    /// Write the history file.
    SaveHistory,
    /// Play this sound (through the sound service).
    Sound(SoundHint),
}

/// `NotificationClosed`'s reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CloseReason {
    /// 1: it expired.
    Expired,
    /// 2: the person dismissed it.
    Dismissed,
    /// 3: `CloseNotification`.
    ClosedByCall,
    /// 4: undefined.
    Undefined,
}

impl CloseReason {
    /// The spec's number.
    pub fn wire(self) -> u32 {
        match self {
            CloseReason::Expired => 1,
            CloseReason::Dismissed => 2,
            CloseReason::ClosedByCall => 3,
            CloseReason::Undefined => 4,
        }
    }
}
