//! sill draws the accounts sheet: this is the half that serves it (`org.quire.AccountsSheet1`,
//! porter's frozen interface; PLAN §2.4).
//!
//! accountd calls `Open(handle, parent_window, view)`, `Update(handle, view)` and `Close(handle)`
//! with a `SheetView` as JSON, and hears the person's answers as `Input(handle, input)` signals
//! carrying a `SheetInput` as JSON. The service publishes the open sheets
//! ([`AccountsSheets`]) for the accounts-sheet surface to draw, and sends what the person does
//! (an [`AccountsSheetCommand::Act`]) back through the pure step.
//!
//! - **Only the account services.** A call is let in only from accountd (`org.quire.Accounts1`),
//!   inferd (`org.quire.Inference1`: AI keys, the AI permit and agent key sheets) or syncd
//!   (`org.quire.Sync1`: its own sign-in), and only when the system shows the caller to be that
//!   service: the program the table of the desktop's own programs names for its role, owning
//!   the name ([`crate::callers`]). A call from any other connection is refused
//!   (`AccessDenied`) and draws nothing. `Update` and `Close` also must come from the connection
//!   that opened the handle.
//! - **`Input` goes to accountd alone.** It is sent to the opener's unique name, never broadcast,
//!   because it may carry a typed password; nothing here logs it or keeps it after it is sent.
//! - **Passive until [`AccountsSheetCommand::Serve`]**: a test's `start` never claims a name.
//! - **The host opens the page** a browser step shows (accountd opens none); showing the same page
//!   again (accountd's answer to `OpenAgain`) opens it again, through the launch service.
//! - A service leaving the bus (or being replaced) takes down its own sheets, and only those.

mod backend;

use sill_model::accounts::sheet::{AccountsSheetCommand, AccountsSheets};
use sill_model::launch::LaunchCommand;

use crate::control::Control;
use crate::env::Env;
use crate::error::ServiceError;
use crate::service::{Service, ServiceIo};

/// What the service drives: the launch service, which opens the page a browser step shows.
#[derive(Debug, Clone)]
pub struct AccountsSheetLinks {
    /// Where the browser step's page is opened.
    pub launch: Control<LaunchCommand>,
}

/// The accounts sheet's host side as a [`Service`].
#[derive(Debug)]
pub enum AccountsSheetService {}

impl Service for AccountsSheetService {
    const NAME: &'static str = "accounts-sheet";
    type State = AccountsSheets;
    type Command = AccountsSheetCommand;
    type Deps = AccountsSheetLinks;

    fn run(
        env: &Env,
        deps: AccountsSheetLinks,
        io: ServiceIo<Self>,
    ) -> impl Future<Output = Result<(), ServiceError>> + Send + 'static {
        let bus = env.buses.session.clone();
        let callers = std::sync::Arc::clone(&env.callers);
        async move {
            backend::run(bus, callers, deps, io).await;
            Ok(())
        }
    }
}
