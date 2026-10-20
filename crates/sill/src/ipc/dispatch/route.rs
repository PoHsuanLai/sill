//! The request to its group's dispatch fn.

use shell_host::HostCtx;
use sill_ipc::request::Request;
use sill_ipc::response::{FailureKind, Response};
use sill_ui_kit::links::Origin;

use super::quit::{Leaving, REFUSED, leaving_at};
use super::reach::Reach;
use super::reply::failed;
use super::{act, debug, show, status};
use crate::daemon::msg::ShellMsg;

/// Answer one request. `ctx` is the host thread's: `Quit` ends it, `Reload` re-applies the scroll
/// engine's settings, and the debug probes inject into surfaces through it.
pub fn route(ctx: &mut HostCtx<ShellMsg>, reach: &Reach, request: Request) -> Response {
    match request {
        Request::Ping => Response::Pong,
        Request::Quit => quit(ctx, reach),
        Request::Reload => crate::actions::reload(ctx, reach),
        Request::Do(ask) => crate::actions::perform(ctx, reach, ask, Origin::anywhere()),
        Request::Show(request) => show::dispatch(reach, request),
        Request::Status(request) => status::dispatch(reach, request),
        Request::Act(request) => act::dispatch(reach, request),
        Request::Debug(request) => debug::dispatch(ctx, reach, request),
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
