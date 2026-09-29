//! `sill lock` on the daemon's side.

use sill_ipc::{Failure, FailureKind, Response};
use sill_services::{LockerCommand, Services};

/// `sill lock`.
pub fn lock(services: &Services) -> Response {
    match services.locker_control.send(LockerCommand::Lock) {
        Ok(()) => Response::Done,
        Err(error) => Response::Failed(Failure {
            kind: FailureKind::Internal,
            message: format!("the locker service: {error}"),
        }),
    }
}
