//! The activation token for a launch, asked of the compositor with a deadline. A compositor that
//! never answers must cost a launch its focus hand-over, not the launch itself: the app starts
//! without a token.

use std::time::Duration;

use shell_host::SurfaceHandle;
use sill_launcher::{AppId, DesktopAction, SettingsLink};
use sill_model::launch::{LaunchCommand, LaunchRequest, SettingsOpen};
use sill_services::ServiceError;
use sill_services::control::Control;

/// How long a launch waits for the compositor's token. It answers within a frame or two when it
/// answers at all; a person notices a click that takes longer than this to do anything.
pub const ACTIVATION_DEADLINE: Duration = Duration::from_millis(300);

/// An xdg-activation token from `surface` for launching `app` (or none in particular), or `None`
/// when the compositor has none to give, refused, or did not answer within
/// [`ACTIVATION_DEADLINE`].
pub async fn activation_token(surface: &SurfaceHandle, app: Option<String>) -> Option<String> {
    tokio::time::timeout(ACTIVATION_DEADLINE, surface.activation_token(app))
        .await
        .ok()
        .flatten()
}

/// Launch `app` through `launch` with a token asked of `surface` (none when the surface is gone
/// or the compositor gives none): the one path every launch from a surface takes, so the app's
/// window is let to the front.
pub async fn launch_with_token(
    surface: Option<&SurfaceHandle>,
    launch: &Control<LaunchCommand>,
    app: AppId,
    action: DesktopAction,
    uris: Vec<String>,
) -> Result<(), ServiceError> {
    let token = match surface {
        Some(surface) => activation_token(surface, Some(app.0.clone())).await,
        None => None,
    };
    launch.send(LaunchCommand::Launch(LaunchRequest {
        app,
        action,
        files: Vec::new(),
        uris,
        token,
    }))
}

/// The app Settings runs as: the one an activation token for Settings is asked for.
pub const SETTINGS_APP: &str = "org.quire.Settings";

/// Open Settings at `link` through `launch` with a token asked of `surface` (none when the
/// surface is gone or the compositor gives none): the one path every entrance to Settings takes.
pub async fn open_settings_with_token(
    surface: Option<&SurfaceHandle>,
    launch: &Control<LaunchCommand>,
    link: SettingsLink,
) -> Result<(), ServiceError> {
    let token = match surface {
        Some(surface) => activation_token(surface, Some(SETTINGS_APP.to_owned())).await,
        None => None,
    };
    launch.send(LaunchCommand::OpenSettings(
        SettingsOpen::at(link).with_token(token),
    ))
}
