//! The daemon's half of launching: ask shell-host for an xdg-activation token on behalf of the
//! surface the click came from, then hand the request to the launch service. `sill launch` goes
//! through the same two steps ([`ask`] on the host thread, [`serve`] off it) and is answered with
//! the launch's timing.

use std::sync::mpsc::SyncSender;

use shell_host::{HostCtx, SurfaceHandle};
use sill_dock::Dock;
use sill_ipc::response::{Failure, FailureKind, Response};
use sill_launcher::{AppId, DesktopAction};
use sill_model::launch::{
    FailureCause, LaunchCommand, LaunchDone, LaunchRequest, Launches, answer_to, failure_sentence,
};
use sill_services::launch::monotonic_now;
use sill_services::{Control, ServiceError};
use sill_ui_kit::services::Services;
use sill_ui_kit::surface::ACTIVATION_DEADLINE;
use tokio::runtime::Handle;
use tokio::sync::{oneshot, watch};

use crate::daemon::msg::ShellMsg;
use crate::ipc::server::REPLY_TIMEOUT;
use crate::outputs::Outputs;

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

/// One `sill launch` on its way from the host thread to the launch service.
#[derive(Debug)]
pub struct LaunchAsk {
    pub app: AppId,
    /// The app's name, for the sentence that says it did not open.
    pub name: String,
    /// The activation token the dock's surface was asked for.
    pub token: oneshot::Receiver<Option<String>>,
    pub reply: SyncSender<Response>,
}

/// Begin `sill launch app` on the host thread, where surfaces can be asked for a token: an app
/// that is not installed is refused at once, else the token is asked of the first dock (the
/// surface a dock click comes from) and the rest is [`serve`]d on `runtime`.
pub fn ask(
    ctx: &mut HostCtx<ShellMsg>,
    services: &Services,
    outputs: &Outputs,
    runtime: &Handle,
    app: AppId,
    reply: SyncSender<Response>,
) {
    let name = services
        .apps
        .state
        .borrow()
        .get(&app)
        .map(|found| found.name.clone());
    let Some(name) = name else {
        // A client that has gone away cannot be told.
        let _gone_is_dropped = reply.send(unknown_app(&app));
        return;
    };
    let dock = outputs
        .outputs
        .iter()
        .find_map(|(_, surfaces)| surfaces.surface_of::<Dock>());
    let token = match dock {
        Some(surface) => ctx.activation_token(surface, Some(app.0.clone())),
        None => oneshot::channel().1,
    };
    let ask = LaunchAsk {
        app,
        name,
        token,
        reply,
    };
    runtime.spawn(serve(
        ask,
        services.launch.control.clone(),
        services.launch.state.clone(),
    ));
}

/// Wait for the token (as long as a dock click would), start the launch, and answer `ask` when
/// the service has published how it ended. Silent when it does not within the IPC server's reply
/// time: the server has answered by then.
pub async fn serve(
    ask: LaunchAsk,
    control: Control<LaunchCommand>,
    mut state: watch::Receiver<Launches>,
) {
    let LaunchAsk {
        app,
        name,
        token,
        reply,
    } = ask;
    let token = tokio::time::timeout(ACTIVATION_DEADLINE, token)
        .await
        .ok()
        .and_then(Result::ok)
        .flatten();
    let asked_at = monotonic_now();
    let sent = control.send(LaunchCommand::Launch(LaunchRequest {
        app: app.clone(),
        action: DesktopAction::Default,
        files: Vec::new(),
        uris: Vec::new(),
        token,
    }));
    let response = match sent {
        Ok(()) => {
            let ended = tokio::time::timeout(REPLY_TIMEOUT, async {
                let launches = state
                    .wait_for(|launches| answer_to(&launches.done, &app, asked_at).is_some())
                    .await
                    .ok()?;
                answer_to(&launches.done, &app, asked_at).cloned()
            })
            .await
            .ok()
            .flatten();
            match ended {
                Some(done) => response_of(&done, &name),
                None => return,
            }
        }
        Err(error) => refusal(
            FailureKind::Internal,
            format!("{name} could not be opened because the launcher stopped: {error}."),
        ),
    };
    let _gone_is_dropped = reply.send(response);
}

/// What `sill launch` is told of a launch that ended as `done`.
pub fn response_of(done: &LaunchDone, name: &str) -> Response {
    match done {
        LaunchDone::Spawned(timing) => Response::Launched(timing.clone()),
        LaunchDone::Refused { cause, .. } => refusal(
            match cause {
                FailureCause::NotInstalled => FailureKind::UnknownApp,
                _ => FailureKind::Refused,
            },
            failure_sentence(name, *cause),
        ),
    }
}

/// The refusal of a `sill launch` for an id no installed app has.
fn unknown_app(app: &AppId) -> Response {
    refusal(
        FailureKind::UnknownApp,
        format!("There is no installed app with the id {}.", app.0),
    )
}

fn refusal(kind: FailureKind, message: String) -> Response {
    Response::Failed(Failure { kind, message })
}

#[cfg(test)]
#[path = "launch_tests.rs"]
mod tests;
