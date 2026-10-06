//! What the accounts service reads and drives.

use sill_model::notifications::{NotesState, NotificationsCommand};
use tokio::sync::watch;

use crate::control::Control;

/// The notifications service, which the notices go through.
#[derive(Debug, Clone)]
pub struct AccountsLinks {
    /// Where the notices are posted and withdrawn.
    pub notifications: Control<NotificationsCommand>,
    /// What the notifications service holds, to find the notice already shown.
    pub notes: watch::Receiver<NotesState>,
}
