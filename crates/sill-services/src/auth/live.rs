//! The auth service's loop (lane a fills it; a stub that returns at once until then, so the
//! service is inert and `Control::send` answers `Gone`).
//!
//! What it does once filled: loads the [`super::Account`] once at start (uid from
//! `/proc/self`'s owner, [`super::user_name_in_passwd`] over `/etc/passwd`, AccountsService's
//! `IconFile` over the system bus when it answers within 1 s, [`super::avatar_source`]); feeds
//! every command and every finished check through [`super::lockout::step`] with a monotonic
//! [`crate::units::Stamp`]; runs each `Check` with `authenticator.check` on `rt`'s blocking
//! pool, tagged so a verdict that comes back after a `Reset` is dropped; turns `Fake` into a
//! verdict without the authenticator; ticks at [`super::lockout::next_deadline`]; publishes
//! [`super::Auth`] (a `LockedOut` wait as wall-clock `UnixMs`); ends when every control is gone.

use std::sync::Arc;

use tokio::runtime::Handle;
use tokio::sync::{mpsc, watch};

use super::authenticator::Authenticator;
use super::types::{Auth, AuthCommand};

pub(super) async fn run(
    tx: watch::Sender<Auth>,
    commands: mpsc::UnboundedReceiver<AuthCommand>,
    authenticator: Arc<dyn Authenticator>,
    rt: Handle,
) {
    let _ = (tx, commands, authenticator, rt);
}
