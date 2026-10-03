//! sill's seat at intentd. One connection to the session bus owns three things, because intentd
//! names a caller's role from the well-known names its connection owns:
//!
//! - `org.quire.Shell`, where sill is the **provider** of the shell's own actions
//!   (`IntentProvider1`; `dist/intents/org.quire.Shell.toml` declares them). A `Perform` is handed
//!   to the daemon as a [`ShellCall`] and answered when the daemon has done it. This name also
//!   gives the connection the `launcher`, `confirm` and `control` roles in intentd's shipped
//!   configuration, so it is claimed first.
//! - `org.quire.Confirm1`, the **sheet**: `Confirm(request) -> request object` and `Cancel(id)`.
//!   The service publishes the queue (`Intents::sheets`) for the confirm surface to draw and sends
//!   the answer as `org.quire.Intents1.Request.Response(0, <ConfirmAnswer JSON>)`, to the asker
//!   alone. Only intentd (the owner of `org.quire.Intents1`) may ask; an unanswered question
//!   expires with its own clock and is never an allow.
//! - the **control** client: the terminal's standing grants (`Control.TerminalGrants`,
//!   `Control.RevokeTerminalGrant`) and `Control.Resume` for a terminal the breaker paused.
//!
//! **Passive until the daemon's first [`IntentsCommand::Serve`]**: a test's `start` never claims a
//! name; every live test runs on a private bus with a stand-in asker. With no session bus the link
//! is `Unavailable(NoBus)` and intentd, finding no trusted sheet, dismisses what it asks.

mod backend;

use tokio::sync::{mpsc, oneshot};

use sill_model::intents::shell::{ShellAction, ShellFault};
use sill_model::intents::{Intents, IntentsCommand};

use crate::env::Env;
use crate::error::ServiceError;
use crate::service::{Service, ServiceIo};

pub use backend::manifest;

/// One `Perform` of a shell action, and where to say how it went.
#[derive(Debug)]
pub struct ShellCall {
    pub action: ShellAction,
    pub reply: oneshot::Sender<Result<(), ShellFault>>,
}

/// What the service needs from the daemon: the way to have a shell action done.
#[derive(Debug, Clone)]
pub struct IntentsLinks {
    pub calls: mpsc::UnboundedSender<ShellCall>,
}

/// The seat as a [`Service`].
#[derive(Debug)]
pub enum IntentsService {}

impl Service for IntentsService {
    const NAME: &'static str = "intents";
    type State = Intents;
    type Command = IntentsCommand;
    type Deps = IntentsLinks;

    fn run(
        env: &Env,
        deps: IntentsLinks,
        io: ServiceIo<Self>,
    ) -> impl Future<Output = Result<(), ServiceError>> + Send + 'static {
        let reach = backend::Reach::of(env);
        async move {
            backend::run(reach, deps, io).await;
            Ok(())
        }
    }
}
