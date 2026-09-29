//! `Status`: the state a service publishes, as the wire's report.

use sill_ipc::response::Response;
use sill_ipc::status::{
    ClipboardStatus, NotificationsStatus, PolkitStatus, StatusReport, StatusRequest,
};

use super::reach::Reach;

/// The report `request` asks for.
pub fn dispatch(reach: &Reach, request: StatusRequest) -> Response {
    let services = reach.services;
    Response::Status(match request {
        StatusRequest::Notifications => {
            let notes = services.notifications.state.borrow();
            StatusReport::Notifications(NotificationsStatus::of(&notes.live, &notes.history))
        }
        StatusRequest::Lock => StatusReport::Lock(*services.locker.state.borrow()),
        StatusRequest::Polkit => {
            StatusReport::Polkit(PolkitStatus::of(&services.polkit_agent.state.borrow()))
        }
        StatusRequest::Clipboard => {
            StatusReport::Clipboard(ClipboardStatus::of(&services.clipboard.state.borrow()))
        }
        StatusRequest::Idle => StatusReport::Idle(*services.idle.state.borrow()),
    })
}
