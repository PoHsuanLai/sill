//! `Debug`: the development probes. Every one is answered even in a release build; the ones that
//! act on a surface, unlock a session or wake idle are refused unless the daemon was built with
//! the `debug` feature.

mod fakes;
mod inject;
mod launcher;
mod stats;

use shell_host::HostCtx;
use sill_ipc::debug::DebugRequest;
use sill_ipc::response::Response;

use super::reach::Reach;
use crate::daemon::msg::ShellMsg;
use inject::Injection;

/// The probe `request` asks for.
pub fn dispatch(ctx: &mut HostCtx<ShellMsg>, reach: &Reach, request: DebugRequest) -> Response {
    match request {
        DebugRequest::Stats => Response::Stats(stats::collect(ctx, reach.outputs)),
        DebugRequest::Pointer { surface, at } => {
            inject::inject(ctx, surface, Injection::Pointer(at))
        }
        DebugRequest::Leave { surface } => inject::inject(ctx, surface, Injection::Leave),
        DebugRequest::Capture { surface, path } => {
            inject::inject(ctx, surface, Injection::Capture(path))
        }
        DebugRequest::Click {
            surface,
            at,
            button,
        } => inject::inject(ctx, surface, Injection::Click(at, button)),
        DebugRequest::PointerPress {
            surface,
            at,
            button,
        } => inject::inject(ctx, surface, Injection::Press(at, button)),
        DebugRequest::PointerRelease {
            surface,
            at,
            button,
        } => inject::inject(ctx, surface, Injection::Release(at, button)),
        DebugRequest::Osd {
            topic,
            percent,
            mute,
        } => fakes::osd(reach, topic, percent, mute),
        DebugRequest::Notify(posting) => fakes::notify(reach, *posting),
        DebugRequest::Screenshot { path } => fakes::screenshot(reach, path),
        DebugRequest::Corner(corner) => fakes::corner(reach, corner),
        DebugRequest::Auth(verdict) => fakes::auth(reach, verdict),
        DebugRequest::Polkit(request) => fakes::polkit(reach, request),
        DebugRequest::Switcher { apps } => fakes::switcher(reach, apps),
        DebugRequest::IdleWake => fakes::idle_wake(reach),
        DebugRequest::LauncherKey(key) => launcher::key(reach.launcher, key),
        DebugRequest::LauncherRows => launcher::rows(reach.launcher),
    }
}
