//! Telling the update service the release in use started well. The service keeps the previous
//! release until one has been confirmed, and takes a release back if sessions on it keep not
//! reaching a working desktop; so a session that did reach one says so, with `slipway confirm`.
//!
//! "Reached one" is: the shell has run for [`SETTLE`] on the person's own session bus, its host
//! thread (the connection to the compositor) still answers, and nothing has taken it down. A
//! shell that crashes in its first seconds, or whose compositor is gone, never gets here.

use std::sync::{Arc, mpsc};
use std::time::Duration;

use shell_host::HostHandle;
use sill_ipc::request::Request;
use sill_ipc::response::Response;
use sill_services::env::{BusRoute, Children, ProcessGroup, Start};
use tokio::runtime::Handle;

use crate::daemon::msg::ShellMsg;

/// How long the shell must have been up before the release is called good: past the first
/// crash-and-restart of a bad release, short of a person giving up and logging out.
pub const SETTLE: Duration = Duration::from_secs(30);

/// How long the host thread has to answer a ping before the shell is called not healthy.
pub const ANSWER_WITHIN: Duration = Duration::from_secs(5);

/// Whether the host thread answered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Answered {
    Yes,
    No,
}

/// The command that confirms the release: a program of the update service, not of the shell.
fn confirm_command() -> Start {
    Start {
        argv: vec!["slipway".into(), "confirm".into()],
        token: None,
        group: ProcessGroup::Own,
    }
}

/// After [`SETTLE`], confirm the release if the host answers. Only on the usual session bus: a
/// shell run on a private bus (a test, a development script) says nothing about the installed
/// release.
pub fn spawn_confirmation(
    rt: &Handle,
    session: &BusRoute,
    host: HostHandle<ShellMsg>,
    children: Arc<dyn Children>,
) {
    if *session != BusRoute::Usual {
        return;
    }
    rt.spawn(async move {
        tokio::time::sleep(SETTLE).await;
        let answered = tokio::task::spawn_blocking(move || {
            host_answers(
                |request, reply| {
                    let _ = host.send(ShellMsg::Ipc { request, reply });
                },
                ANSWER_WITHIN,
            )
        })
        .await
        .unwrap_or(Answered::No);
        if answered == Answered::Yes {
            confirm(children.as_ref());
        }
    });
}

/// Ping the host thread through `send`, which hands it the request and the channel to answer on.
fn host_answers(
    send: impl FnOnce(Request, mpsc::SyncSender<Response>),
    within: Duration,
) -> Answered {
    let (reply, answer) = mpsc::sync_channel(1);
    send(Request::Ping, reply);
    match answer.recv_timeout(within) {
        Ok(Response::Pong) => Answered::Yes,
        Ok(_) | Err(_) => Answered::No,
    }
}

/// Run `slipway confirm`. A computer without the update service has nothing to confirm to, which
/// is no error.
fn confirm(children: &dyn Children) {
    match children.detach(&confirm_command()) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => eprintln!("sill: telling the update service this release is good: {error}"),
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use sill_ipc::request::Request;
    use sill_ipc::response::Response;
    use sill_services::env::{Children, RecordedChildren};

    use super::{Answered, confirm, confirm_command, host_answers};

    #[test]
    fn only_a_host_that_pongs_in_time_counts() {
        let soon = Duration::from_millis(100);
        let answers = host_answers(
            |request, reply| {
                assert_eq!(request, Request::Ping);
                let _ = reply.send(Response::Pong);
            },
            soon,
        );
        assert_eq!(answers, Answered::Yes);
        let silent = host_answers(|_request, _reply| {}, soon);
        assert_eq!(silent, Answered::No, "a host that never answers");
        let other = host_answers(
            |_request, reply| {
                let _ = reply.send(Response::Done);
            },
            soon,
        );
        assert_eq!(other, Answered::No, "an answer that is not a pong");
    }

    #[test]
    fn confirming_runs_slipway_confirm_once() {
        let recorder = RecordedChildren::default();
        confirm(&recorder);
        assert_eq!(recorder.started(), [confirm_command()]);
        assert_eq!(
            confirm_command().argv,
            vec!["slipway".to_owned(), "confirm".to_owned()]
        );
    }

    /// A computer without the update service: the program is not found, and nothing is said.
    #[derive(Debug)]
    struct Missing;

    impl Children for Missing {
        fn detach(&self, _start: &sill_services::env::Start) -> std::io::Result<()> {
            Err(std::io::ErrorKind::NotFound.into())
        }
    }

    #[test]
    fn a_computer_without_the_update_service_confirms_nothing_and_says_nothing() {
        confirm(&Missing);
    }
}
