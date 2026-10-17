//! Telling the update service the release in use started well. The service keeps the previous
//! release until one has been confirmed, and takes a release back if sessions on it keep not
//! reaching a working desktop; so a session that did reach one says so, with `slipway confirm`.
//!
//! "Reached one" is: the shell has run for [`SETTLE`] on the person's own session bus, its host
//! thread (the connection to the compositor) still answers, and nothing has taken it down. A
//! shell that crashes in its first seconds, or whose compositor is gone, never gets here.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Duration;

use shell_host::HostHandle;
use sill_ipc::request::Request;
use sill_ipc::response::Response;
use sill_services::env::{BusRoute, Children, Env, ProcessGroup, SessionMode, Start};
use tokio::runtime::Handle;

use crate::daemon::msg::ShellMsg;

/// How long the shell must have been up before the release is called good: past the first
/// crash-and-restart of a bad release, short of a person giving up and logging out.
pub const SETTLE: Duration = Duration::from_secs(30);

/// How long the host thread has to answer a ping before the shell is called not healthy.
pub const ANSWER_WITHIN: Duration = Duration::from_secs(5);

/// Where the update service's program is exported on the computer, for a shell that does not run
/// from a release folder.
const EXPORTED: &str = "/usr/local/bin/slipway";

/// Whether the host thread answered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Answered {
    Yes,
    No,
}

/// Whether this run of the shell says anything about the installed release.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Speaks {
    /// It does: the person's own session, started the usual way.
    ForTheRelease,
    /// It does not: a private bus (a test, a development script) is not the installed release.
    NotTheInstalled,
    /// It does not: the session fell back to the safe compositor because the usual one kept
    /// exiting, and a release that gets confirmed from there is never taken back.
    FellBackToSafe,
}

fn speaks(session: &BusRoute, mode: SessionMode) -> Speaks {
    match (session, mode) {
        (BusRoute::Usual, SessionMode::Normal) => Speaks::ForTheRelease,
        (BusRoute::Usual, SessionMode::Safe) => Speaks::FellBackToSafe,
        (BusRoute::Absent | BusRoute::Address(_), _) => Speaks::NotTheInstalled,
    }
}

/// The update service's program: beside this one in the release folder, else where it is
/// exported. `exists` says whether a file is there.
fn slipway_program(bin_dir: Option<&Path>, exists: impl Fn(&Path) -> bool) -> Option<PathBuf> {
    bin_dir
        .map(|dir| dir.join("slipway"))
        .into_iter()
        .chain([PathBuf::from(EXPORTED)])
        .find(|program| exists(program))
}

/// The command that confirms the release: a program of the update service, not of the shell.
fn confirm_command(program: PathBuf) -> Start {
    Start {
        argv: vec![program.into_os_string(), OsString::from("confirm")],
        token: None,
        group: ProcessGroup::Own,
    }
}

/// After [`SETTLE`], confirm the release if the host answers. Only on the usual session bus and
/// not in the safe fallback: a shell run on a private bus (a test, a development script) says
/// nothing about the installed release, and one run in the safe compositor says nothing good.
pub fn spawn_confirmation(rt: &Handle, env: &Env, host: HostHandle<ShellMsg>) {
    match speaks(env.buses.session.route(), env.session_mode) {
        Speaks::NotTheInstalled => return,
        Speaks::FellBackToSafe => {
            eprintln!(
                "sill: not telling the update service this release is good: the session fell \
                 back to the safe compositor, so this run proves nothing about the release"
            );
            return;
        }
        Speaks::ForTheRelease => {}
    }
    let (children, bin_dir) = (env.children.clone(), env.release_bin.clone());
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
            let program = slipway_program(bin_dir.as_deref(), Path::is_file);
            if let Confirmed::Not(why) = confirm(children.as_ref(), program) {
                eprintln!("sill: telling the update service this release is good: {why}");
            }
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

/// Whether the update service was told.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Confirmed {
    Told,
    /// It was not, and why, in words for the journal.
    Not(String),
}

/// Run `slipway confirm` with `program`. A release that cannot say it is good is rolled back after
/// three logins, so a missing program is an error to log, not a thing to pass over.
fn confirm(children: &dyn Children, program: Option<PathBuf>) -> Confirmed {
    let Some(program) = program else {
        return Confirmed::Not(format!(
            "no slipway program beside this one or at {EXPORTED}, so this release stays unconfirmed"
        ));
    };
    match children.detach(&confirm_command(program.clone())) {
        Ok(()) => Confirmed::Told,
        Err(error) => Confirmed::Not(format!("{}: {error}", program.display())),
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};
    use std::time::Duration;

    use sill_ipc::request::Request;
    use sill_ipc::response::Response;
    use sill_services::env::{BusRoute, Children, RecordedChildren, SessionMode};

    use super::{
        Answered, Confirmed, EXPORTED, Speaks, confirm, confirm_command, host_answers,
        slipway_program, speaks,
    };

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
    fn a_run_speaks_for_the_release_only_on_the_usual_bus_and_not_in_safe_mode() {
        let private = || BusRoute::Address("unix:path=/tmp/private".to_owned());
        let cases = [
            (
                "the usual bus, the usual compositor",
                BusRoute::Usual,
                SessionMode::Normal,
                Speaks::ForTheRelease,
            ),
            (
                "the usual bus, the safe fallback",
                BusRoute::Usual,
                SessionMode::Safe,
                Speaks::FellBackToSafe,
            ),
            (
                "a private bus",
                private(),
                SessionMode::Normal,
                Speaks::NotTheInstalled,
            ),
            (
                "a private bus, safe",
                private(),
                SessionMode::Safe,
                Speaks::NotTheInstalled,
            ),
            (
                "no bus",
                BusRoute::Absent,
                SessionMode::Normal,
                Speaks::NotTheInstalled,
            ),
        ];
        for (name, bus, mode, want) in cases {
            assert_eq!(speaks(&bus, mode), want, "{name}");
        }
    }

    #[test]
    fn the_update_service_is_found_beside_this_program_else_where_it_is_exported() {
        let beside = PathBuf::from("/opt/quire/releases/1/bin/slipway");
        let exported = PathBuf::from(EXPORTED);
        let dir = PathBuf::from("/opt/quire/releases/1/bin");
        let cases: [(&str, Option<&Path>, Vec<&PathBuf>, Option<&PathBuf>); 5] = [
            (
                "beside",
                Some(&dir),
                vec![&beside, &exported],
                Some(&beside),
            ),
            (
                "only exported",
                Some(&dir),
                vec![&exported],
                Some(&exported),
            ),
            ("no release folder", None, vec![&exported], Some(&exported)),
            ("nowhere", Some(&dir), vec![], None),
            ("nowhere, no folder", None, vec![], None),
        ];
        for (name, bin_dir, present, want) in cases {
            let found =
                slipway_program(bin_dir, |path| present.iter().any(|p| p.as_path() == path));
            assert_eq!(found.as_ref(), want, "{name}");
        }
    }

    #[test]
    fn confirming_runs_the_found_program_with_confirm_once() {
        let recorder = RecordedChildren::default();
        let program = PathBuf::from("/opt/quire/releases/1/bin/slipway");
        assert_eq!(confirm(&recorder, Some(program.clone())), Confirmed::Told);
        assert_eq!(recorder.started(), [confirm_command(program.clone())]);
        assert_eq!(
            confirm_command(program.clone()).argv,
            vec![
                program.into_os_string(),
                std::ffi::OsString::from("confirm")
            ]
        );
    }

    /// A program that cannot be started.
    #[derive(Debug)]
    struct Unstartable;

    impl Children for Unstartable {
        fn detach(&self, _start: &sill_services::env::Start) -> std::io::Result<()> {
            Err(std::io::ErrorKind::NotFound.into())
        }
    }

    #[test]
    fn a_missing_update_service_is_an_error_to_log_not_a_silence() {
        let recorder = RecordedChildren::default();
        let missing = confirm(&recorder, None);
        assert!(matches!(missing, Confirmed::Not(why) if why.contains(EXPORTED)));
        assert!(recorder.started().is_empty(), "nothing was started");
        let failed = confirm(&Unstartable, Some(PathBuf::from("/x/slipway")));
        assert!(matches!(failed, Confirmed::Not(why) if why.contains("/x/slipway")));
    }
}
