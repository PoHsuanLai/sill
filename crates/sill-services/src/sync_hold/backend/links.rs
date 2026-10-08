//! What the sync-hold service reads and drives.

use sill_model::launch::LaunchCommand;
use sill_model::notifications::{NotesState, NotificationsCommand};
use tokio::sync::watch;

use crate::control::Control;

/// The notifications service the notices go through, and the launch service that opens Settings.
#[derive(Debug, Clone)]
pub struct SyncHoldLinks {
    /// Where the notices are posted and withdrawn.
    pub notifications: Control<NotificationsCommand>,
    /// What the notifications service holds, to find the notice already shown.
    pub notes: watch::Receiver<NotesState>,
    /// Opens Settings at the account.
    pub launch: Control<LaunchCommand>,
}
