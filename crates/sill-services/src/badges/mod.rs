//! Launcher entry badges: counts, progress and urgency apps publish over
//! `com.canonical.Unity.LauncherEntry.Update(s app_uri, a{sv})` (plan hooks table,
//! `use_launcher_entry`: "zbus match on `com.canonical.Unity.LauncherEntry.Update`").
//!
//! [`spawn`] and [`live`] do the effect (a session-bus match rule, zbus 5 on tokio); [`model`]
//! is the pure reducer. Neither reaches `dioxus`, `ds` or `shell-host`
//! (`scripts/check-boundary.sh`): the bar and dock read this module only through [`Badges`] and
//! [`BadgesCommand`], via `sill-surfaces::hooks::use_launcher_entry`.

mod live;
mod model;

pub use model::{Badge, BadgeEvent, BadgeProp, Badges, parse_app_uri, reduce_badges};

use tokio::runtime::Handle;
use tokio::sync::watch;

use crate::control::Control;

/// What the badges service can be told.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BadgesCommand {
    /// Forget every badge (after `sill reload`).
    Clear,
}

/// Subscribe to `LauncherEntry.Update` on the session bus and follow senders leaving it
/// (`NameOwnerChanged` clears their apps' badges). A missing session bus leaves the receiver at
/// `Badges::default()` — starting a service never fails the shell (`error.rs`).
pub fn spawn(rt: &Handle) -> (watch::Receiver<Badges>, Control<BadgesCommand>) {
    let (tx, rx) = watch::channel(Badges::default());
    let (control, commands) = Control::channel();
    rt.spawn(live::run(tx, commands));
    (rx, control)
}
