//! The password check behind sill's own lock screen (design/20 §1.9). The lock surface sends the
//! typed secret ([`AuthCommand::Submit`]); the service runs it through an [`Authenticator`] off the
//! UI thread, counts failures through the pure [`gate`] machine (one check at a time; no lockout),
//! and publishes [`Auth`], whose [`AuthPhase::Accepted`] is what the lock surfaces unlock on (the
//! daemon as a backstop). The polkit prompt does not come through here: polkit's own helper runs
//! PAM for it (`crate::polkit_agent`).
//!
//! How lockers authenticate without root (researched): PAM's `pam_unix` sees a caller that is not
//! root and runs the setuid `unix_chkpwd` helper, which checks the caller's
//! own password against `/etc/shadow` and nobody else's. swaylock (MIT) runs PAM in a forked
//! child that it talks to over a pipe (service `swaylock`, whose file is `auth include login`);
//! hyprlock runs it on a thread (service `hyprlock`); cosmic-greeter's locker calls `pam-client`
//! on tokio's blocking pool (service `cosmic-greeter`), then `acct_mgmt`. sill takes swaylock's
//! shape: [`PamHelper`] runs `sill auth-check` as a child per attempt, which calls PAM through
//! the safe `nonstick` crate ([`pam::check`]), so a module that hangs (a network directory, a
//! fingerprint reader) is killed with its child instead of wedging a thread, and the lock
//! surface's process never holds a PAM handle. The PAM service is `sill` when
//! `/etc/pam.d/sill` exists (`dist/pam.d/sill`: `auth include login`, as swaylock's), else
//! `login` ([`pam::service`]).
//!
//! Nothing here runs PAM until a [`AuthCommand::Submit`] arrives, and every test drives a
//! [`FakeAuthenticator`] (the [`Env`]'s authenticator); no test calls PAM with anyone's real
//! password.

mod account;
mod authenticator;
pub mod gate;
mod helper;
mod live;
pub mod pam;
pub mod protocol;
mod types;

pub use account::{avatar_source, user_name_in_passwd};
pub use authenticator::{Authenticator, FakeAuthenticator, FakeRule};
pub use gate::{Checks, Gate, GateIn, GateOut};
pub use helper::PamHelper;
pub use types::{
    Account, Attempt, Attempts, Auth, AuthCommand, AuthFault, AuthPhase, AvatarSource, FakeVerdict,
    Outcome, PamNote, RealName, Secret, UserName, Verdict,
};

use std::sync::Arc;

use crate::env::{Bus, Env};
use crate::error::ServiceError;
use crate::service::{Service, ServiceIo};

/// Where the person's name and picture are read.
#[derive(Debug, Clone, PartialEq, Eq)]
struct AccountPlace {
    /// AccountsService's bus, which may name the picture.
    system: Bus,
    /// The home folder, whose `.face` is the fallback picture.
    home: std::path::PathBuf,
}

/// The lock screen's password check as a [`Service`], through `env`'s [`Authenticator`] (PAM
/// through `sill auth-check` in the daemon, a [`FakeAuthenticator`] in every test), the account
/// read on `env`'s system bus and home. Idle until the first `Submit`: nothing is checked, and
/// no PAM module runs, before that.
#[derive(Debug)]
pub enum AuthService {}

impl Service for AuthService {
    const NAME: &'static str = "auth";
    type State = Auth;
    type Command = AuthCommand;
    type Deps = ();

    fn run(
        env: &Env,
        (): (),
        io: ServiceIo<Self>,
    ) -> impl Future<Output = Result<(), ServiceError>> + Send + 'static {
        let account = AccountPlace {
            system: env.buses.system.clone(),
            home: env.dirs.home.clone(),
        };
        let authenticator = Arc::clone(&env.authenticator);
        async move {
            live::run(io.state, io.commands, account, authenticator).await;
            Ok(())
        }
    }
}
