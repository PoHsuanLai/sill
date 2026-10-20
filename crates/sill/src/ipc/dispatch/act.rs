//! `Act`: change something by telling a service or writing a file. Every arm answers `Done` once
//! the change is asked for; the change itself happens in the service or the settings watch.

use sill_ipc::act::{ActRequest, DockRequest};
use sill_ipc::response::{FailureKind, Response};
use sill_model::clipboard::ClipboardCommand;
use sill_model::locker::LockerCommand;
use sill_model::notifications::NotificationsCommand;
use sill_model::pins::{PinAt, PinEvent};
use sill_model::screenshots::{ShotCommand, ShotKind};

use super::reach::Reach;
use super::reply::{failed, sent};
use super::{keys, switcher, workspace};

/// The change `request` asks for.
pub fn dispatch(reach: &Reach, request: ActRequest) -> Response {
    let services = reach.services;
    match request {
        ActRequest::Dock(request) => dock(reach, request),
        ActRequest::Display(request) => keys::press(reach, request),
        ActRequest::Lock => sent(services.locker.control.send(LockerCommand::Lock), "locker"),
        ActRequest::Screenshot(kind) => screenshot(reach, kind),
        ActRequest::Switcher(request) => switcher::chord(reach, request),
        ActRequest::Workspace(request) => workspace::step(reach, request),
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

fn dock(reach: &Reach, request: DockRequest) -> Response {
    let event = match request {
        DockRequest::Pin { app } => PinEvent::Pin {
            app,
            at: PinAt::End,
        },
        DockRequest::Unpin { app } => PinEvent::Unpin(app),
    };
    sent(reach.services.pins.control.send(event), "pins")
}

fn screenshot(reach: &Reach, kind: ShotKind) -> Response {
    sent(
        reach
            .services
            .screenshots
            .control
            .send(ShotCommand::Take(kind)),
        "screenshots",
    )
}
