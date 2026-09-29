//! Launcher entry badges: counts, progress and urgency apps publish over
//! `com.canonical.Unity.LauncherEntry.Update(s app_uri, a{sv})`.
//!
//! [`BadgesService`] and [`live`] do the effect (a session-bus match rule, zbus 5 on tokio);
//! [`model`] is the pure reducer. Neither reaches `dioxus`, `ds` or `shell-host`
//! (`scripts/check-boundary.sh`): the bar and dock read this module only through [`Badges`] and
//! [`BadgesCommand`], via `use_service::<BadgesService>`.

mod backend;

use sill_model::badges::{Badges, BadgesCommand};

use crate::env::Env;
use crate::error::ServiceError;
use crate::service::{Service, ServiceIo};

/// LauncherEntry badges as a [`Service`]: follows `LauncherEntry.Update` on the session bus and
/// senders leaving it (`NameOwnerChanged` clears their apps' badges). Without a session bus the
/// state stays at `Badges::default()`.
#[derive(Debug)]
pub enum BadgesService {}

impl Service for BadgesService {
    const NAME: &'static str = "badges";
    type State = Badges;
    type Command = BadgesCommand;
    type Deps = ();

    fn run(
        env: &Env,
        (): (),
        io: ServiceIo<Self>,
    ) -> impl Future<Output = Result<(), ServiceError>> + Send + 'static {
        backend::run(env.buses.session.clone(), io.state, io.commands)
    }
}
