//! The activation token for a launch, asked of the compositor with a deadline. A compositor that
//! never answers must cost a launch its focus hand-over, not the launch itself: the app starts
//! without a token.

use std::time::Duration;

use shell_host::SurfaceHandle;

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
