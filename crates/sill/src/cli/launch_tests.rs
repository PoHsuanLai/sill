use clap::Parser;
use sill_ipc::request::Request;
use sill_ipc::response::{Failure, FailureKind, Response};
use sill_launcher::AppId;
use sill_model::launch::{LaunchTiming, Pid, ScopeUnit, Stamp};

use super::super::client::ClientError;
use super::super::{Cli, Command, USAGE, usage_code};
use super::{Ending, Output, PrintTiming, ending, launch_request, report};

fn parsed(args: &[&str]) -> Result<Cli, clap::Error> {
    Cli::try_parse_from(std::iter::once("sill").chain(args.iter().copied()))
}

fn timing() -> LaunchTiming {
    LaunchTiming {
        app: AppId("foot".to_owned()),
        unit: Some(ScopeUnit("app-sill-foot-1.scope".to_owned())),
        requested_at: Stamp(100),
        spawned_at: Stamp(250),
        pid: Some(Pid(4242)),
    }
}

fn failure(kind: FailureKind) -> Result<Response, ClientError> {
    Ok(Response::Failed(Failure {
        kind,
        message: "why".to_owned(),
    }))
}

#[test]
fn the_flag_and_the_id_are_read_and_the_desktop_ending_is_dropped() {
    let cases: &[(&[&str], &str, PrintTiming)] = &[
        (&["launch", "foot"], "foot", PrintTiming::Skip),
        (
            &["launch", "org.mozilla.firefox", "--print-timing"],
            "org.mozilla.firefox",
            PrintTiming::Print,
        ),
        (
            &["launch", "--print-timing", "org.mozilla.firefox.desktop"],
            "org.mozilla.firefox",
            PrintTiming::Print,
        ),
    ];
    for (args, id, print) in cases {
        let Some(Command::Launch(launch)) = parsed(args)
            .unwrap_or_else(|e| panic!("{args:?}: {e}"))
            .command
        else {
            panic!("{args:?}: not a launch");
        };
        assert_eq!(launch.timing, *print, "{args:?}");
        assert_eq!(
            launch_request(&launch),
            Request::Launch(AppId((*id).to_owned())),
            "{args:?}"
        );
    }
}

#[test]
fn what_the_shell_answers_decides_the_exit_code() {
    let cases: Vec<(&str, Result<Response, ClientError>, Ending)> = vec![
        (
            "launched",
            Ok(Response::Launched(timing())),
            Ending::Launched(timing()),
        ),
        (
            "an id nobody installed",
            failure(FailureKind::UnknownApp),
            Ending::Failed {
                code: 2,
                sentence: "why".to_owned(),
            },
        ),
        (
            "an app that would not start",
            failure(FailureKind::Refused),
            Ending::Failed {
                code: 1,
                sentence: "why".to_owned(),
            },
        ),
        (
            "no answer in time",
            failure(FailureKind::Timeout),
            Ending::Failed {
                code: 1,
                sentence: "why".to_owned(),
            },
        ),
        (
            "another kind of answer",
            Ok(Response::Done),
            Ending::Failed {
                code: 1,
                sentence: "The shell answered something else.".to_owned(),
            },
        ),
        (
            "no shell running",
            Err(ClientError::NotRunning),
            Ending::Failed {
                code: 1,
                sentence: "The desktop shell is not running.".to_owned(),
            },
        ),
    ];
    for (name, reply, want) in cases {
        assert_eq!(ending(reply), want, "{name}");
    }
}

#[test]
fn the_timing_line_goes_to_stdout_only_when_asked_and_a_failure_to_stderr() {
    let line = "launch desktop_id=foot unit=app-sill-foot-1.scope requested_at=100 spawned_at=250 pid=4242\n";
    let failed = Ending::Failed {
        code: 2,
        sentence: "No app has that id.".to_owned(),
    };
    let cases = [
        (
            Ending::Launched(timing()),
            PrintTiming::Print,
            Output::Stdout(line.to_owned()),
            0,
        ),
        (
            Ending::Launched(timing()),
            PrintTiming::Skip,
            Output::Nothing,
            0,
        ),
        (
            failed.clone(),
            PrintTiming::Print,
            Output::Stderr("sill: No app has that id.".to_owned()),
            2,
        ),
        (
            failed,
            PrintTiming::Skip,
            Output::Stderr("sill: No app has that id.".to_owned()),
            2,
        ),
    ];
    for (given, print, output, code) in cases {
        assert_eq!(report(&given, print), (output, code), "{given:?} {print:?}");
    }
}

#[test]
fn a_wrong_command_line_ends_with_the_usage_code_and_help_with_zero() {
    let cases: &[(&[&str], u8)] = &[
        (&["launch"], USAGE),
        (&["launch", "foot", "extra"], USAGE),
        (&["launch", "foot", "--timing"], USAGE),
        (&["frobnicate"], USAGE),
        (&["launch", "--help"], 0),
        (&["--version"], 0),
    ];
    for (args, want) in cases {
        let error = parsed(args)
            .err()
            .unwrap_or_else(|| panic!("{args:?} was accepted"));
        assert_eq!(usage_code(&error), *want, "{args:?}");
    }
    assert_eq!(USAGE, 64);
}
