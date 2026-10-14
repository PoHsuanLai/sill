//! What the launch service reads and drives.

use sill_model::apps::AppIndex;
use sill_model::notifications::NotificationsCommand;
use tokio::sync::watch;

use crate::control::Control;

/// The app index that resolves ids to entries, and the notifications service a failed launch is
/// told through.
#[derive(Debug, Clone)]
pub struct LaunchLinks {
    /// The desktop entries.
    pub apps: watch::Receiver<AppIndex>,
    /// Where the notice of a failed launch is posted.
    pub notifications: Control<NotificationsCommand>,
}
