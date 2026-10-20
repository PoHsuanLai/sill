//! Work that starts an app and so wants the activation token of the surface the person used: the
//! token is asked of that surface now, on the host thread, and the command goes to its service
//! when the compositor answers or the deadline passes. The one place a launch gets its token.

use shell_host::HostCtx;
use sill_ipc::response::Response;
use sill_services::ServiceError;
use sill_ui_kit::links::Origin;
use sill_ui_kit::surface::ACTIVATION_DEADLINE;

use crate::daemon::msg::ShellMsg;
use crate::ipc::dispatch::Reach;

/// Ask `origin`'s surface (when it has one) for a token for `app`, then run `send` with it.
/// Answered `Done` once the ask is made: the launch itself ends off this thread.
pub fn with_token(
    ctx: &mut HostCtx<ShellMsg>,
    reach: &Reach,
    origin: Origin,
    app: &str,
    send: impl FnOnce(Option<String>) -> Result<(), ServiceError> + Send + 'static,
) -> Response {
    let answer = origin
        .surface
        .map(|surface| ctx.activation_token(surface, Some(app.to_owned())));
    reach.runtime.spawn(async move {
        let token = match answer {
            Some(answer) => tokio::time::timeout(ACTIVATION_DEADLINE, answer)
                .await
                .ok()
                .and_then(Result::ok)
                .flatten(),
            None => None,
        };
        if let Err(error) = send(token) {
            eprintln!("sill: launching: {error}");
        }
    });
    Response::Done
}
