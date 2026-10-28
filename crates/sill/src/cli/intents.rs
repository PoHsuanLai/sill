//! `sill intents status|revoke|resume` and its pure mapping to a request.

use clap::Subcommand;
use sill_ipc::request::Request;
use sill_ipc::status::StatusRequest;
use sill_model::intents::terminal::parse_grant;
use sill_model::intents::{ActionRef, IntentsAsk};

/// `sill intents ...`.
#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub enum IntentsAction {
    /// Whether sill serves intentd's sheet, what waits on it, and what the terminal may do
    /// without asking.
    Status,
    /// Withdraw one standing terminal grant: `sill intents revoke org.quire.Shell:shell.dock.pin`.
    Revoke {
        #[arg(value_name = "APP:ACTION", value_parser = parse_grant)]
        grant: ActionRef,
    },
    /// Let the terminal go on after the breaker paused it (too many refusals in a row).
    Resume,
}

/// The request `sill intents ACTION` sends.
pub fn intents_request(action: &IntentsAction) -> Request {
    match action {
        IntentsAction::Status => Request::Status(StatusRequest::Intents),
        IntentsAction::Revoke { grant } => Request::Intents(IntentsAsk::Revoke(grant.clone())),
        IntentsAction::Resume => Request::Intents(IntentsAsk::Resume),
    }
}
