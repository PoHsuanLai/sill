//! What is done to an app: open it, show it in Files, hide or close its windows, end it, keep it
//! in the dock or take it out.

use shell_host::HostCtx;
use sill_ipc::response::{FailureKind, Response};
use sill_launcher::WindowKey;
use sill_model::actions::{AppVerb, Subject};
use sill_model::launch::{LaunchCommand, LaunchRequest};
use sill_model::pins::{PinAt, PinEvent};
use sill_model::windows::WlCommand;
use sill_ui_kit::links::Origin;

use super::launch::with_token;
use crate::daemon::msg::ShellMsg;
use crate::ipc::dispatch::{Reach, failed, sent};

/// `verb` done to `subject`; the subject was checked to be what `verb` needs.
pub fn perform(
    ctx: &mut HostCtx<ShellMsg>,
    reach: &Reach,
    verb: AppVerb,
    subject: Subject,
    origin: Origin,
) -> Response {
    let services = reach.services;
    match (verb, subject) {
        (AppVerb::Open, Subject::App(app)) => {
            let launch = services.launch.control.clone();
            let id = app.0.clone();
            with_token(ctx, reach, origin, &id, move |token| {
                launch.send(LaunchCommand::Launch(LaunchRequest::app(app).token(token)))
            })
        }
        (AppVerb::ShowInFiles, Subject::App(app)) => sent(
            services
                .launch
                .control
                .send(LaunchCommand::ShowInFiles(app)),
            "launch",
        ),
        (AppVerb::ForceQuit, Subject::App(app)) => sent(
            services.launch.control.send(LaunchCommand::ForceQuit(app)),
            "launch",
        ),
        (AppVerb::Pin, Subject::App(app)) => pin(
            reach,
            PinEvent::Pin {
                app,
                at: PinAt::End,
            },
        ),
        (AppVerb::Unpin, Subject::App(app)) => pin(reach, PinEvent::Unpin(app)),
        (AppVerb::Hide, Subject::Windows(keys)) => windows(reach, keys, WlCommand::Minimize),
        (AppVerb::Quit, Subject::Windows(keys)) => windows(reach, keys, WlCommand::Close),
        (_, subject) => failed(
            FailureKind::Internal,
            format!("an action was given {subject:?} that it cannot use"),
        ),
    }
}

fn pin(reach: &Reach, event: PinEvent) -> Response {
    sent(reach.services.pins.control.send(event), "pins")
}

fn windows(reach: &Reach, keys: Vec<WindowKey>, command: fn(WindowKey) -> WlCommand) -> Response {
    let control = &reach.services.windows.control;
    sent(
        keys.into_iter()
            .try_for_each(|key| control.send(command(key))),
        "windows",
    )
}
