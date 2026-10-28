//! The request to its answer: one match, one arm per request. An arm calls a link, a service's
//! control or a follower and returns a `Response`; it holds no logic.

use shell_host::HostCtx;
use sill_ipc::request::Request;
use sill_ipc::response::{FailureKind, Response};
use sill_ipc::show::{LauncherRequest, Reveal};
use sill_ipc::status::{
    ClipboardStatus, IntentsStatus, NotificationsStatus, PolkitStatus, StatusReport, StatusRequest,
};
use sill_model::actions::Panel;
use sill_model::clipboard::ClipboardCommand;
use sill_model::notifications::NotificationsCommand;
use sill_ui_kit::links::Origin;
use sill_ui_kit::services::Services;

use super::keys;
use super::quit::{Leaving, REFUSED, leaving_at};
use super::reach::Reach;
use super::reply::{failed, sent};
use crate::actions::panel;
use crate::daemon::msg::ShellMsg;

/// Answer one request. `ctx` is the host thread's: `Quit` ends it, `Reload` re-applies the scroll
/// engine's settings, and the debug probes inject into surfaces through it.
pub fn route(ctx: &mut HostCtx<ShellMsg>, reach: &Reach, request: Request) -> Response {
    let services = reach.services;
    match request {
        Request::Ping => Response::Pong,
        Request::Quit => quit(ctx, reach),
        Request::Reload => crate::actions::reload(ctx, reach),
        Request::Do(ask) => crate::actions::perform(ctx, reach, ask, Origin::anywhere()),
        Request::Launcher(LauncherRequest::Close) => {
            panel::launcher(reach, Reveal::Close, String::new(), None)
        }
        Request::Launcher(LauncherRequest::Open { query }) => {
            panel::launcher(reach, Reveal::Open, query, None)
        }
        Request::PowerMenu(reveal) => panel::show(reach, Panel::PowerMenu, reveal, None),
        Request::ForceQuit(reveal) => panel::show(reach, Panel::ForceQuit, reveal, None),
        Request::NotificationCenter(reveal) => {
            panel::show(reach, Panel::NotificationCenter, reveal, None)
        }
        Request::Companion(request) => crate::companion::dispatch(reach, request),
        Request::Voice(request) => crate::voice::dispatch(reach, request),
        Request::Status(request) => Response::Status(report(services, request)),
        Request::Display(request) => keys::press(reach, request),
        Request::ClearClipboard => sent(
            services.clipboard.control.send(ClipboardCommand::Clear),
            "clipboard",
        ),
        Request::DismissNotifications => sent(
            services
                .notifications
                .control
                .send(NotificationsCommand::DismissAll),
            "notifications",
        ),
        Request::Keycap(command) => sent(services.keycap.control.send(command.into()), "keycap"),
        Request::Intents(ask) => sent(services.intents.control.send(ask.into()), "intents"),
        Request::FocusLatest => {
            reach.links.banners.ask_keyboard();
            Response::Done
        }
        // Answered by the host loop, which waits for the app's process off this thread
        // (`daemon::events::handle`); it never reaches here.
        Request::Launch(_) => failed(
            FailureKind::Internal,
            "launching is answered by the host loop".to_owned(),
        ),
        Request::Debug(request) => super::debug::dispatch(ctx, reach, request),
    }
}

/// The state a service publishes, as the wire's report.
fn report(services: &Services, request: StatusRequest) -> StatusReport {
    match request {
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
        StatusRequest::Keycap => StatusReport::Keycap(services.keycap.state.borrow().clone()),
        StatusRequest::Intents => {
            StatusReport::Intents(IntentsStatus::of(&services.intents.state.borrow()))
        }
    }
}

/// End the shell, unless the session is covered: a locked session needs the shell to take the
/// password, so the request is refused until it is unlocked.
fn quit(ctx: &mut HostCtx<ShellMsg>, reach: &Reach) -> Response {
    let phase = reach.services.locker.state.borrow().phase;
    match leaving_at(phase) {
        Leaving::Allowed => {
            ctx.exit();
            Response::Done
        }
        Leaving::Refused => failed(FailureKind::Refused, REFUSED.to_owned()),
    }
}
