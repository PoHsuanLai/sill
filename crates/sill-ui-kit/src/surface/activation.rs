//! The activation token for a launch, asked of the compositor with a deadline. A compositor that
//! never answers must cost a launch its focus hand-over, not the launch itself: the app starts
//! without a token.

use std::time::Duration;

use tokio::sync::oneshot;

use shell_host::SurfaceHandle;
use sill_launcher::{AppId, DesktopAction, SettingsLink};
use sill_model::launch::{LaunchCommand, LaunchRequest, SETTINGS_BUS_NAME, SettingsOpen};
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

/// A token already asked of the compositor on the host thread, waited for at most
/// [`ACTIVATION_DEADLINE`]: `None` when it gave none, refused, or was too slow.
pub async fn token_within(asked: oneshot::Receiver<Option<String>>) -> Option<String> {
    tokio::time::timeout(ACTIVATION_DEADLINE, asked)
        .await
        .ok()
        .and_then(Result::ok)
        .flatten()
}

/// The token for launching `app` from `surface`, when the launch has a surface to ask for it:
/// the one place the "has a surface" question is answered, so no entrance launches without a
/// token it could have had.
pub async fn token_from(surface: Option<&SurfaceHandle>, app: Option<String>) -> Option<String> {
    match surface {
        Some(surface) => activation_token(surface, app).await,
        None => None,
    }
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
    let token = token_from(surface, Some(app.0.clone())).await;
    launch.send(LaunchCommand::Launch(
        LaunchRequest::app(app)
            .action(action)
            .uris(uris)
            .token(token),
    ))
}

/// Open Settings at `link` through `launch` with a token asked of `surface` (none when the
/// surface is gone or the compositor gives none): the one path every entrance to Settings takes.
pub async fn open_settings_with_token(
    surface: Option<&SurfaceHandle>,
    launch: &Control<LaunchCommand>,
    link: SettingsLink,
) -> Result<(), ServiceError> {
    let token = token_from(surface, Some(SETTINGS_BUS_NAME.to_owned())).await;
    launch.send(LaunchCommand::OpenSettings(
        SettingsOpen::at(link).with_token(token),
    ))
}
