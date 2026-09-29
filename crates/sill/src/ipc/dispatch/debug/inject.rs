//! The probes that act on one surface: pointer motion, press, release and click, and a capture.

use std::path::PathBuf;

use shell_host::HostCtx;
#[cfg(feature = "debug")]
use shell_host::SurfaceId;
use sill_ipc::debug::{Point, PointerButton, SurfaceName};
use sill_ipc::response::{FailureKind, Response};

use super::super::reply::failed;
#[cfg(not(feature = "debug"))]
use super::super::reply::unsupported;
use crate::daemon::msg::ShellMsg;

/// What [`inject`] was asked to do, independent of `shell_host::Inject` (only defined under
/// shell-host's `debug-inject` feature) so both builds compile.
pub enum Injection {
    Pointer(Point),
    Leave,
    Capture(PathBuf),
    Click(Point, PointerButton),
    Press(Point, PointerButton),
    Release(Point, PointerButton),
}

/// Find `surface` by shell-host's own naming (`HostCtx::surface_named`, unconditional: it needs
/// neither `debug` nor shell-host's `debug-inject`) and deliver `injection` to it.
pub fn inject(ctx: &mut HostCtx<ShellMsg>, surface: SurfaceName, injection: Injection) -> Response {
    let Some(id) = ctx.surface_named(&shell_host::SurfaceName(surface.0.clone())) else {
        return failed(
            FailureKind::UnknownSurface,
            format!("no surface {:?}", surface.0),
        );
    };
    apply(ctx, id, injection)
}

/// Through shell-host's typed `HostCtx::inject` (a capture's success and size come back directly
/// through `Injected`, no bridging over stdout).
#[cfg(feature = "debug")]
fn apply(ctx: &mut HostCtx<ShellMsg>, id: SurfaceId, injection: Injection) -> Response {
    match ctx.inject(id, to_host(injection)) {
        Ok(_injected) => Response::Done,
        Err(error) => failed(FailureKind::Internal, error.to_string()),
    }
}

/// `injection` as shell-host's; every pointer button maps through.
#[cfg(feature = "debug")]
fn to_host(injection: Injection) -> shell_host::Inject {
    let point = |at: Point| shell_host::Point { x: at.x, y: at.y };
    match injection {
        Injection::Pointer(at) => shell_host::Inject::Pointer { at: point(at) },
        Injection::Leave => shell_host::Inject::Leave,
        Injection::Capture(path) => shell_host::Inject::Capture { path },
        Injection::Click(at, button) => shell_host::Inject::Click {
            at: point(at),
            button: host_button(button),
        },
        Injection::Press(at, button) => shell_host::Inject::PointerDown {
            at: point(at),
            button: host_button(button),
        },
        Injection::Release(at, button) => shell_host::Inject::PointerUp {
            at: point(at),
            button: host_button(button),
        },
    }
}

#[cfg(feature = "debug")]
fn host_button(button: PointerButton) -> shell_host::inject::PointerButton {
    match button {
        PointerButton::Primary => shell_host::inject::PointerButton::Primary,
        PointerButton::Secondary => shell_host::inject::PointerButton::Secondary,
        PointerButton::Middle => shell_host::inject::PointerButton::Middle,
    }
}

#[cfg(not(feature = "debug"))]
fn apply(
    _ctx: &mut HostCtx<ShellMsg>,
    _id: shell_host::SurfaceId,
    injection: Injection,
) -> Response {
    // Naming the injection keeps every variant's payload read in this build too.
    let what = match injection {
        Injection::Pointer(at) => format!("pointer at {},{}", at.x, at.y),
        Injection::Click(at, button) => format!("{button:?} click at {},{}", at.x, at.y),
        Injection::Press(at, button) => format!("{button:?} press at {},{}", at.x, at.y),
        Injection::Release(at, button) => format!("{button:?} release at {},{}", at.x, at.y),
        Injection::Leave => "leave".to_owned(),
        Injection::Capture(path) => format!("capture to {}", path.display()),
    };
    unsupported(&format!("built without the debug feature ({what})"))
}

#[cfg(all(test, feature = "debug"))]
mod tests {
    use sill_ipc::debug::{Point, PointerButton};

    use super::{Injection, to_host};

    #[test]
    fn a_click_press_or_release_maps_onto_shell_hosts_injection_with_its_button() {
        let at = Point { x: 1200, y: 16 };
        let host_at = shell_host::Point { x: 1200, y: 16 };
        let buttons = [
            (
                PointerButton::Primary,
                shell_host::inject::PointerButton::Primary,
            ),
            (
                PointerButton::Secondary,
                shell_host::inject::PointerButton::Secondary,
            ),
            (
                PointerButton::Middle,
                shell_host::inject::PointerButton::Middle,
            ),
        ];
        for (ours, theirs) in buttons {
            assert_eq!(
                to_host(Injection::Click(at, ours)),
                shell_host::Inject::Click {
                    at: host_at,
                    button: theirs,
                },
                "click {ours:?}"
            );
            assert_eq!(
                to_host(Injection::Press(at, ours)),
                shell_host::Inject::PointerDown {
                    at: host_at,
                    button: theirs,
                },
                "press {ours:?}"
            );
            assert_eq!(
                to_host(Injection::Release(at, ours)),
                shell_host::Inject::PointerUp {
                    at: host_at,
                    button: theirs,
                },
                "release {ours:?}"
            );
        }
    }
}
