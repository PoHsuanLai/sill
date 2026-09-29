//! Notifications (design/20 §1.6, design/13 §13.3.6): an `org.freedesktop.Notifications` server
//! on the session bus, the live banners, and the history grouped by app, kept in
//! `$XDG_STATE_HOME/sill/notifications.json`.
//!
//! **It never fights another daemon.** The service asks for the name only while the daemon's
//! policy says `notifications.server = "own"` (the setting's default), with `AllowReplacement |
//! DoNotQueue` and never `ReplaceExisting`: when cosmic-notifications, mako or plasmashell holds
//! it, sill stays passive, reports the holder (`NameHolder::Other`), and asks again only when
//! the name falls free. Until the daemon's first [`NotificationsCommand::Policy`] the service is
//! passive, so a test or a headless render that spawns `crate::Services` never takes the name.
//!
//! **Where the time goes.** The banner's hold (`notifications.banner_hold_ms`, paused while
//! hovered) is counted by the banner surface's machine, which says so with
//! [`NotificationsCommand::BannerTimedOut`]; the sender's `expire_timeout` only decides whether
//! a banner persists (0). A banner leaving by itself keeps its notification in the history.
//!
//! **Without D-Bus.** `sill debug notify` posts a `Posting` straight to the service
//! ([`NotificationsCommand::Post`]), through the same reducer, so the banners and the center can
//! be accepted without a bus (and on a machine whose session already has a notification daemon).
//!
//! The shape is every service's: [`NotificationsService`] publishes [`NotesState`] (the live
//! banners and name holder, and the history: one server's state, stepped together), and `step`
//! is the only place it changes.

mod backend;

use sill_model::notifications::{NotesState, NotificationsCommand};

use crate::env::Env;
use crate::error::ServiceError;
use crate::service::{Service, ServiceIo};

pub use backend::{BUS_NAME, OBJECT_PATH};

/// The notifications server as a [`Service`]: on `env`'s session bus with its clock, the
/// history in its state directory. Passive until the daemon's first
/// policy (module doc).
#[derive(Debug)]
pub enum NotificationsService {}

impl Service for NotificationsService {
    const NAME: &'static str = "notifications";
    type State = NotesState;
    type Command = NotificationsCommand;
    type Deps = ();

    fn run(
        env: &Env,
        (): (),
        io: ServiceIo<Self>,
    ) -> impl Future<Output = Result<(), ServiceError>> + Send + 'static {
        let bus = env.buses.session.clone();
        let file = backend::history_path(&env.dirs);
        let clock = std::sync::Arc::clone(&env.clock);
        async move {
            backend::run(bus, file, clock, io.state, io.commands).await;
            Ok(())
        }
    }
}
