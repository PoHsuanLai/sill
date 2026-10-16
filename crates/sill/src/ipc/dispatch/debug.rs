//! `Debug`: the development probes. Only the read-only ones (`stats`, `launcher-rows`) are answered
//! in a release build; every probe that acts (injects input, makes up a notification, a request
//! to approve, a verdict that unlocks) is refused unless the daemon was built with the `debug`
//! feature.

mod fakes;
mod inject;
mod launcher;
mod stats;

use shell_host::HostCtx;
use sill_ipc::debug::DebugRequest;
use sill_ipc::response::Response;

use super::reach::Reach;
use super::reply::unsupported;
use crate::daemon::msg::ShellMsg;
use inject::Injection;

/// The probe `request` asks for.
pub fn dispatch(ctx: &mut HostCtx<ShellMsg>, reach: &Reach, request: DebugRequest) -> Response {
    match (probe_of(&request), BUILD) {
        (Probe::Acts, Build::Release) => unsupported("built without the debug feature"),
        (Probe::Acts, Build::Debug) | (Probe::Reads, _) => run_probe(ctx, reach, request),
    }
}

/// Whether the daemon was built with the `debug` feature.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Build {
    Debug,
    Release,
}

const BUILD: Build = if cfg!(feature = "debug") {
    Build::Debug
} else {
    Build::Release
};

/// Whether a probe only reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Probe {
    Reads,
    Acts,
}

fn probe_of(request: &DebugRequest) -> Probe {
    match request {
        DebugRequest::Stats | DebugRequest::LauncherRows => Probe::Reads,
        DebugRequest::Pointer { .. }
        | DebugRequest::Leave { .. }
        | DebugRequest::Capture { .. }
        | DebugRequest::Click { .. }
        | DebugRequest::PointerPress { .. }
        | DebugRequest::PointerRelease { .. }
        | DebugRequest::Key { .. }
        | DebugRequest::Osd { .. }
        | DebugRequest::Notify(_)
        | DebugRequest::Screenshot { .. }
        | DebugRequest::Corner(_)
        | DebugRequest::Auth(_)
        | DebugRequest::Polkit(_)
        | DebugRequest::Switcher { .. }
        | DebugRequest::IdleWake => Probe::Acts,
    }
}

fn run_probe(ctx: &mut HostCtx<ShellMsg>, reach: &Reach, request: DebugRequest) -> Response {
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
        DebugRequest::Key {
            surface,
            key,
            action,
        } => inject::inject(ctx, surface, Injection::Key(key, action)),
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
        DebugRequest::LauncherRows => launcher::rows(&reach.links.launcher),
    }
}

#[cfg(test)]
mod tests {
    use sill_ipc::debug::{CornerName, DebugRequest};

    use super::{Probe, probe_of};

    #[test]
    fn only_the_read_only_probes_are_answered_in_a_release_build() {
        let cases = [
            ("stats", DebugRequest::Stats, Probe::Reads),
            ("launcher rows", DebugRequest::LauncherRows, Probe::Reads),
            (
                "corner",
                DebugRequest::Corner(CornerName::BottomRight),
                Probe::Acts,
            ),
            ("idle wake", DebugRequest::IdleWake, Probe::Acts),
        ];
        for (name, request, want) in cases {
            assert_eq!(probe_of(&request), want, "{name}");
        }
    }
}
