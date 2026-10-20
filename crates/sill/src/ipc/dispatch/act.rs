//! `Act`: change something by telling a service or writing a file. Every arm answers `Done` once
//! the change is asked for; the change itself happens in the service or the settings watch. The
//! shell's own actions are not here: they are `Request::Do`, carried out by `crate::actions`.

use sill_ipc::act::ActRequest;
use sill_ipc::response::{FailureKind, Response};
use sill_model::clipboard::ClipboardCommand;
use sill_model::notifications::NotificationsCommand;

use super::keys;
use super::reach::Reach;
use super::reply::{failed, sent};

/// The change `request` asks for.
pub fn dispatch(reach: &Reach, request: ActRequest) -> Response {
    let services = reach.services;
    match request {
        ActRequest::Display(request) => keys::press(reach, request),
        ActRequest::ClearClipboard => sent(
            services.clipboard.control.send(ClipboardCommand::Clear),
            "clipboard",
        ),
        ActRequest::DismissNotifications => sent(
            services
                .notifications
                .control
                .send(NotificationsCommand::DismissAll),
            "notifications",
        ),
        ActRequest::Keycap(command) => sent(services.keycap.control.send(command.into()), "keycap"),
        ActRequest::Intents(ask) => sent(services.intents.control.send(ask.into()), "intents"),
        ActRequest::FocusLatest => {
            reach.links.banners.ask_keyboard();
            Response::Done
        }
        // Answered by the host loop, which waits for the app's process off this thread
        // (`daemon::events::handle`); it never reaches here.
        ActRequest::Launch(_) => failed(
            FailureKind::Internal,
            "launching is answered by the host loop".to_owned(),
        ),
    }
}
