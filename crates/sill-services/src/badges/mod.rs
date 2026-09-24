//! Launcher entry badges: counts, progress and urgency apps publish over
//! `com.canonical.Unity.LauncherEntry.Update(s app_uri, a{sv})` (plan hooks table,
//! `use_launcher_entry`: "zbus match on `com.canonical.Unity.LauncherEntry.Update`").

#![allow(unused_variables, dead_code)] // Freeze stubs

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
/// (`NameOwnerChanged` clears their apps' badges).
pub fn spawn(rt: &Handle) -> (watch::Receiver<Badges>, Control<BadgesCommand>) {
    todo!("M3 badges: zbus MatchRule on com.canonical.Unity.LauncherEntry Update -> reduce_badges")
}
