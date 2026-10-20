//! accountd (`org.quire.Accounts1`), as the shell hears it: when an account needs the person to
//! sign in again, sill says so in a notification with a "Sign in again" button, and takes the
//! notice back when the account is well again (design/31 §4.4, PLAN §2.8).
//!
//! The service follows accountd's name ([`crate::owner`]); on meeting it, it asks for the
//! accounts needing sign-in now (which also makes the connection one accountd sends signals to),
//! then hears `NeedsReauth` and watches each such account's `State` until it is no longer
//! `needs_reauth`. The button's key carries the account, and its `ActionInvoked` from the
//! notifications server answers only when its id is the notice sill holds for that account: the
//! service then calls the account's `Reauthenticate`,
//! and accountd asks sill's own sheet (`org.quire.AccountsSheet1`) to draw the sign-in.
//!
//! Without accountd the state stays at its default. A bus that is absent by choice ends the
//! service silently.

mod backend;
pub mod sheet;

use sill_model::accounts::{AccountsCommand, AccountsState};

use crate::env::Env;
use crate::error::ServiceError;
use crate::service::{Service, ServiceIo};

pub use backend::AccountsLinks;
pub(crate) use backend::wire::label_of;

/// The accounts that need signing in again, on `env`'s session bus, told through the
/// notifications service in its [`AccountsLinks`].
#[derive(Debug)]
pub enum AccountsService {}

impl Service for AccountsService {
    const NAME: &'static str = "accounts";
    type State = AccountsState;
    type Command = AccountsCommand;
    type Deps = AccountsLinks;

    fn run(
        env: &Env,
        deps: Self::Deps,
        io: ServiceIo<Self>,
    ) -> impl Future<Output = Result<(), ServiceError>> + Send + 'static {
        let bus = env.buses.session.clone();
        async move { backend::run(bus, deps, io).await }
    }
}
