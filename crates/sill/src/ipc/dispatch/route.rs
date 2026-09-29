//! The request to its group's dispatch fn.

use shell_host::HostCtx;
use sill_ipc::request::Request;
use sill_ipc::response::{FailureKind, Response};

use super::reach::Reach;
use super::reply::failed;
use super::{act, debug, show, status};
use crate::daemon::msg::ShellMsg;

/// Answer one request. `ctx` is the host thread's: `Quit` ends it, `Reload` re-applies the scroll
/// engine's settings, and the debug probes inject into surfaces through it.
pub fn route(ctx: &mut HostCtx<ShellMsg>, reach: &Reach, request: Request) -> Response {
    match request {
        Request::Ping => Response::Pong,
        Request::Quit => {
            ctx.exit();
            Response::Done
        }
        Request::Reload => reload(ctx, reach),
        Request::Show(request) => show::dispatch(reach, request),
        Request::Status(request) => status::dispatch(reach, request),
        Request::Act(request) => act::dispatch(reach, request),
        Request::Debug(request) => debug::dispatch(ctx, reach, request),
    }
}

/// Re-read `settings.toml` and publish it: the bar and every surface pick it up through the
/// settings feed, the scroll engine through `ctx`. Dock pins are not re-read here: the pins
/// service watches its own file.
fn reload(ctx: &mut HostCtx<ShellMsg>, reach: &Reach) -> Response {
    let response = match reach.settings_dir {
        Some(dir) => {
            crate::daemon::settings::reload(dir, reach.settings);
            Response::Done
        }
        None => failed(
            FailureKind::Internal,
            "no settings directory (XDG_CONFIG_HOME and HOME are both unset)".to_owned(),
        ),
    };
    ctx.set_scroll_config(crate::scroll::config(&reach.settings.borrow()));
    response
}
