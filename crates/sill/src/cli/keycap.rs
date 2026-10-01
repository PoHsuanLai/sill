//! `sill keycap status|retry|resume` and its pure mapping to a request.

use clap::Subcommand;
use sill_ipc::act::ActRequest;
use sill_ipc::request::Request;
use sill_ipc::status::StatusRequest;
use sill_model::keycap::KeycapAsk;

/// `sill keycap ...`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Subcommand)]
pub enum KeycapAction {
    /// Whether keycap runs, and what it says about its mode and the keyboards.
    Status,
    /// Start keycap again after it stopped.
    Retry,
    /// Turn the layout back on after the escape chord paused it.
    Resume,
}

/// The request `sill keycap ACTION` sends.
pub fn keycap_request(action: KeycapAction) -> Request {
    match action {
        KeycapAction::Status => Request::Status(StatusRequest::Keycap),
        KeycapAction::Retry => Request::Act(ActRequest::Keycap(KeycapAsk::Retry)),
        KeycapAction::Resume => Request::Act(ActRequest::Keycap(KeycapAsk::Resume)),
    }
}
