//! The session state's pure transition.

use super::{Session, SessionEvent};

/// `session` after `event`.
pub fn step(session: Session, event: SessionEvent) -> Session {
    let _ = session;
    match event {
        SessionEvent::Probed(probed) => probed,
        SessionEvent::Gone => Session::default(),
    }
}
