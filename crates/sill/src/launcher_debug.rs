//! `sill debug launcher-key KEY` and `sill debug launcher-rows` (M9, FINDINGS "M9 freeze" F653):
//! what `dev/accept-launcher-v2.sh` drives the open launcher with on the nested compositor,
//! where there is no keyboard to type on. Lane d fills both through the `LauncherLink` (a key
//! the panel steps as if pressed; the rows it shows, clipboard titles replaced). Refused by a
//! daemon built without `debug`.

use sill_ipc::{Failure, FailureKind, FakeLauncherKey, Response};
use sill_surfaces::surfaces::launcher::LauncherLink;

/// Press `key` on the open launcher.
pub fn key(launcher: &LauncherLink, key: FakeLauncherKey) -> Response {
    let _ = (launcher, key);
    not_yet("launcher keys are not filled yet (M9 lane d)")
}

/// The open launcher's rows as shown.
pub fn rows(launcher: &LauncherLink) -> Response {
    let _ = launcher;
    not_yet("launcher rows are not filled yet (M9 lane d)")
}

fn not_yet(message: &str) -> Response {
    Response::Failed(Failure {
        kind: FailureKind::Unsupported,
        message: message.to_owned(),
    })
}
