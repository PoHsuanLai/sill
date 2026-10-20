//! What the session service reads and drives besides logind.

use sill_model::apps::AppIndex;
use sill_model::locker::LockerCommand;
use sill_model::notifications::NotificationsCommand;
use sill_model::windows::{WindowModel, WlCommand};
use tokio::sync::watch;

use crate::control::Control;

/// The other services the session service uses.
#[derive(Debug, Clone)]
pub struct SessionLinks {
    /// Where `Run(Lock)` goes.
    pub locker: Control<LockerCommand>,
    /// The open windows, which Log Out, Restart and Shut Down wait to see closed.
    pub windows: watch::Receiver<WindowModel>,
    /// Where the request to close each window goes.
    pub wl: Control<WlCommand>,
    /// The desktop entries that name an app that kept the session from ending.
    pub apps: watch::Receiver<AppIndex>,
    /// Where the notice that the session did not end is posted.
    pub notifications: Control<NotificationsCommand>,
}
