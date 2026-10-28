//! The session's command line: `sill lock [now|status]` and its pure mapping to a request.

use clap::ValueEnum;
use sill_ipc::request::Request;
use sill_ipc::status::StatusRequest;
use sill_launcher::actions::ActionId;

/// `sill lock [VERB]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum LockVerb {
    /// Lock now.
    Now,
    /// What the lock screen is doing.
    Status,
}

/// The request `sill lock VERB` sends.
pub fn lock_request(verb: LockVerb) -> Request {
    match verb {
        LockVerb::Now => super::verbs::do_request(ActionId::Lock),
        LockVerb::Status => Request::Status(StatusRequest::Lock),
    }
}
