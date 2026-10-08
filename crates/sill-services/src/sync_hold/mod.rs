//! syncd (`org.quire.Sync1`), as the shell hears it: when a dataset's replica loses most of its
//! files, syncd holds the removal here and tells the shell, which says so in a notification
//! whose action opens Settings at the account. The decision is Settings' (it alone may confirm
//! the discard); the shell offers no removal. The notification is taken back when the hold ends.
//!
//! The service subscribes to `NeedsConfirmation`, then calls `Sync1.Watch()` once (holds already
//! on arrive before it returns), and calls it again whenever `org.quire.Sync1` gets a new owner:
//! syncd keeps its roster in memory. Without syncd the state stays empty.

mod backend;

use sill_model::sync_hold::{SyncHoldCommand, SyncHoldState};

use crate::env::Env;
use crate::error::ServiceError;
use crate::service::{Service, ServiceIo};

pub use backend::SyncHoldLinks;

/// The held mass deletions on `env`'s session bus, told through the notifications service and
/// opened in Settings through the launch service, both in its [`SyncHoldLinks`].
#[derive(Debug)]
pub enum SyncHoldService {}

impl Service for SyncHoldService {
    const NAME: &'static str = "sync_hold";
    type State = SyncHoldState;
    type Command = SyncHoldCommand;
    type Deps = SyncHoldLinks;

    fn run(
        env: &Env,
        deps: Self::Deps,
        io: ServiceIo<Self>,
    ) -> impl Future<Output = Result<(), ServiceError>> + Send + 'static {
        let bus = env.buses.session.clone();
        async move { backend::run(bus, deps, io).await }
    }
}
