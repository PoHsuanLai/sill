//! What the notifications server tells the services that posted a notice: the answer to a
//! button, and the end of the notice.

use super::{ActionKey, CloseReason, NotificationId};

/// What happened to a notification the server held.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Said {
    /// An action was invoked: which notification, and which action of it.
    Pressed { id: NotificationId, key: ActionKey },
    /// The notification closed, and why.
    Closed {
        id: NotificationId,
        reason: CloseReason,
    },
}
