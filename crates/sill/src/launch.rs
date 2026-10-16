//! The daemon's half of launching: ask shell-host for an xdg-activation token on behalf of the
//! surface the click came from, then hand the request to the launch service.

use shell_host::SurfaceHandle;
use sill_launcher::{AppId, DesktopAction};
use sill_model::launch::{LaunchCommand, LaunchRequest};
use sill_services::{Control, ServiceError};

/// Launch `app` (its `action`) from `surface`, with a fresh activation token when the
/// compositor grants one. Dropped files and URIs are the dock and launcher's own concern
/// (drag-and-drop launches are not wired yet: both are empty here).
pub async fn launch(
    surface: &SurfaceHandle,
    service: &Control<LaunchCommand>,
    app: AppId,
    action: DesktopAction,
) -> Result<(), ServiceError> {
    let token = sill_ui_kit::surface::activation_token(surface, Some(app.0.clone())).await;
    service.send(LaunchCommand::Launch(LaunchRequest {
        app,
        action,
        files: Vec::new(),
        uris: Vec::new(),
        token,
    }))
}

// `SurfaceHandle::new` is `pub(crate)` to shell-host, so this file has no way to build one for
// a unit test; the only untested line is the `activation_token` call and the `LaunchRequest`
// shape, both a direct read of shell-host's and sill-services' own frozen signatures. Covered
// live by dock clicks once dock-ui lands (design/10 §10.3.6), and by `launch::run`'s own tests
// for what happens to the request past this point.
