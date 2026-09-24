//! The daemon's half of launching (plan "Launch"): ask shell-host for an xdg-activation token
//! on behalf of the surface the click came from, then hand the request to the launch service.

#![allow(unused_variables, dead_code)] // Freeze stubs

use shell_host::SurfaceHandle;
use sill_launcher::{AppId, DesktopAction};
use sill_services::{Control, LaunchCommand, ServiceError};

/// Launch `app` (its `action`) from `surface`, with a fresh activation token when the
/// compositor grants one.
pub async fn launch(
    surface: &SurfaceHandle,
    service: &Control<LaunchCommand>,
    app: AppId,
    action: DesktopAction,
) -> Result<(), ServiceError> {
    todo!("M2 launch: surface.activation_token(Some(app id)).await, LaunchCommand::Launch")
}
