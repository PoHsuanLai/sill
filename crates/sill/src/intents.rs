//! The shell's own actions as `org.quire.Shell`: what the daemon does when intentd calls its
//! `IntentProvider1.Perform`. The intents service reads the invocation into a `ShellAction` and
//! hands the daemon a `ShellCall`; [`spawn_follower`] makes the request the matching
//! `sill <verb>` would make ([`request_of`]), has the host thread answer it the way an IPC
//! request is answered (`ShellMsg::Ipc`: one dispatch for the command line, the corners and the
//! agents), and tells the service how it went. Nothing here decides who may ask: intentd's gate
//! did, and the provider answers nobody else.

use std::sync::mpsc;
use std::time::Duration;

use shell_host::HostHandle;
use sill_ipc::act::{ActRequest, DockRequest, WorkspaceRequest};
use sill_ipc::request::Request;
use sill_ipc::response::{FailureKind, Response};
use sill_ipc::show::{LauncherRequest, ShowRequest};
use sill_model::intents::shell::{ShellAction, ShellFault, WorkspaceStep};
use sill_services::ShellCall;
use tokio::runtime::Handle;
use tokio::sync::mpsc::UnboundedReceiver;

use crate::daemon::msg::ShellMsg;
use crate::ipc::server::REPLY_TIMEOUT;

/// The request that does `action`: the same one its command-line verb sends.
pub fn request_of(action: ShellAction) -> Request {
    match action {
        ShellAction::LauncherOpen { query } => {
            Request::Show(ShowRequest::Launcher(LauncherRequest::Open { query }))
        }
        ShellAction::DockPin { app } => Request::Act(ActRequest::Dock(DockRequest::Pin { app })),
        ShellAction::DockUnpin { app } => {
            Request::Act(ActRequest::Dock(DockRequest::Unpin { app }))
        }
        ShellAction::WorkspaceSwitch(WorkspaceStep::Next) => {
            Request::Act(ActRequest::Workspace(WorkspaceRequest::Next))
        }
        ShellAction::WorkspaceSwitch(WorkspaceStep::Prev) => {
            Request::Act(ActRequest::Workspace(WorkspaceRequest::Prev))
        }
        ShellAction::Lock => Request::Act(ActRequest::Lock),
        ShellAction::Screenshot(kind) => Request::Act(ActRequest::Screenshot(kind)),
    }
}

/// How the host's answer reads to the provider.
pub fn result_of(response: Response) -> Result<(), ShellFault> {
    match response {
        Response::Done => Ok(()),
        Response::Failed(failure) => match failure.kind {
            FailureKind::Unsupported => Err(ShellFault::Unsupported),
            FailureKind::UnknownSurface
            | FailureKind::UnknownApp
            | FailureKind::Refused
            | FailureKind::Timeout
            | FailureKind::Internal => Err(ShellFault::Failed(failure.message)),
        },
        Response::Pong
        | Response::Status(_)
        | Response::Stats(_)
        | Response::LauncherRows(_)
        | Response::Launched(_) => Err(ShellFault::Failed(
            "the shell answered something else".to_owned(),
        )),
    }
}

/// Carry out each call the intents service hands over, one blocking task each (the host thread
/// answers on a std channel), until the service goes.
pub fn spawn_follower(
    rt: &Handle,
    host: HostHandle<ShellMsg>,
    mut calls: UnboundedReceiver<ShellCall>,
) {
    let blocking = rt.clone();
    rt.spawn(async move {
        while let Some(ShellCall { action, reply }) = calls.recv().await {
            let host = host.clone();
            blocking.spawn_blocking(move || {
                let _ = reply.send(perform(&host, action));
            });
        }
    });
}

/// Post `action`'s request to the host thread and wait for its answer.
fn perform(host: &HostHandle<ShellMsg>, action: ShellAction) -> Result<(), ShellFault> {
    let (reply, answer) = mpsc::sync_channel(1);
    let request = request_of(action);
    host.send(ShellMsg::Ipc { request, reply })
        .map_err(|_| ShellFault::Failed("the shell is shutting down".to_owned()))?;
    match answer.recv_timeout(REPLY_TIMEOUT + Duration::from_millis(500)) {
        Ok(response) => result_of(response),
        Err(_) => Err(ShellFault::Failed("the shell did not answer".to_owned())),
    }
}

#[cfg(test)]
mod tests {
    use sill_ipc::act::{ActRequest, DockRequest, WorkspaceRequest};
    use sill_ipc::request::Request;
    use sill_ipc::response::{Failure, FailureKind, Response};
    use sill_ipc::show::{LauncherRequest, ShowRequest};
    use sill_launcher::AppId;
    use sill_model::intents::shell::{ShellAction, ShellFault, WorkspaceStep};
    use sill_model::screenshots::ShotKind;

    use super::{request_of, result_of};

    #[test]
    fn each_action_makes_the_request_its_verb_makes() {
        let app = || AppId("firefox.desktop".to_owned());
        let cases = vec![
            (
                ShellAction::LauncherOpen {
                    query: "mail".to_owned(),
                },
                Request::Show(ShowRequest::Launcher(LauncherRequest::Open {
                    query: "mail".to_owned(),
                })),
            ),
            (
                ShellAction::DockPin { app: app() },
                Request::Act(ActRequest::Dock(DockRequest::Pin { app: app() })),
            ),
            (
                ShellAction::DockUnpin { app: app() },
                Request::Act(ActRequest::Dock(DockRequest::Unpin { app: app() })),
            ),
            (
                ShellAction::WorkspaceSwitch(WorkspaceStep::Next),
                Request::Act(ActRequest::Workspace(WorkspaceRequest::Next)),
            ),
            (
                ShellAction::WorkspaceSwitch(WorkspaceStep::Prev),
                Request::Act(ActRequest::Workspace(WorkspaceRequest::Prev)),
            ),
            (ShellAction::Lock, Request::Act(ActRequest::Lock)),
            (
                ShellAction::Screenshot(ShotKind::Area),
                Request::Act(ActRequest::Screenshot(ShotKind::Area)),
            ),
        ];
        for (action, want) in cases {
            assert_eq!(request_of(action.clone()), want, "{action:?}");
        }
    }

    fn failed(kind: FailureKind) -> Response {
        Response::Failed(Failure {
            kind,
            message: "why".to_owned(),
        })
    }

    #[test]
    fn the_hosts_answer_reads_as_done_unsupported_or_failed() {
        assert_eq!(result_of(Response::Done), Ok(()));
        assert_eq!(
            result_of(failed(FailureKind::Unsupported)),
            Err(ShellFault::Unsupported)
        );
        for kind in [
            FailureKind::UnknownSurface,
            FailureKind::UnknownApp,
            FailureKind::Timeout,
            FailureKind::Internal,
        ] {
            assert_eq!(
                result_of(failed(kind)),
                Err(ShellFault::Failed("why".to_owned())),
                "{kind:?}"
            );
        }
        assert!(matches!(
            result_of(Response::Pong),
            Err(ShellFault::Failed(_))
        ));
    }
}
