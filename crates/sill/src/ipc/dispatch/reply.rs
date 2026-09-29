//! The two ways a request is not carried out, and how a service's answer becomes a reply.

use std::fmt::Display;

use sill_ipc::response::{Failure, FailureKind, Response};

/// `Done` when a command reached its service, else `Internal` naming `subsystem`.
pub fn sent<E: Display>(result: Result<(), E>, subsystem: &str) -> Response {
    match result {
        Ok(()) => Response::Done,
        Err(error) => failed(FailureKind::Internal, format!("{subsystem}: {error}")),
    }
}

/// This build or this compositor cannot do it: `message` says what is missing.
pub fn unsupported(message: &str) -> Response {
    failed(FailureKind::Unsupported, message.to_owned())
}

/// A failure of `kind`.
pub fn failed(kind: FailureKind, message: String) -> Response {
    Response::Failed(Failure { kind, message })
}
