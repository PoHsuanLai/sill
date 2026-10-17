//! `sill launch DESKTOP-ID [--print-timing]`: open an installed app exactly as a click in the
//! dock does. The running shell does the launch and answers once the app's process exists; with
//! `--print-timing` the timing line is printed on stdout.
//!
//! Exit codes: 0 launched, 1 the app could not be opened (one sentence on stderr), 2 no installed
//! app has that id, 64 the command line is wrong.

use std::process::ExitCode;

use clap::builder::{BoolValueParser, TypedValueParser};
use clap::{ArgAction, Args};
use sill_ipc::act::ActRequest;
use sill_ipc::request::Request;
use sill_ipc::response::{FailureKind, Response};
use sill_launcher::AppId;
use sill_model::launch::{LaunchTiming, launch_line};

use super::client::{ClientError, send};
use super::stdout;

/// `sill launch ...`.
#[derive(Debug, Clone, PartialEq, Eq, Args)]
pub struct LaunchArgs {
    /// The app's desktop id, for example `org.mozilla.firefox`; a trailing `.desktop` is fine.
    /// Only apps with an installed desktop file open.
    pub app: String,
    /// Print one line saying when the app was asked for and when it started, in nanoseconds on
    /// the machine's monotonic clock.
    #[arg(
        long = "print-timing",
        action = ArgAction::SetTrue,
        value_parser = BoolValueParser::new().map(PrintTiming::of),
    )]
    pub timing: PrintTiming,
}

/// Whether the timing line is printed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrintTiming {
    Print,
    Skip,
}

impl PrintTiming {
    fn of(asked: bool) -> PrintTiming {
        if asked {
            PrintTiming::Print
        } else {
            PrintTiming::Skip
        }
    }
}

/// The request `sill launch APP` sends: the desktop id without its `.desktop` ending, as the
/// shell's app index names apps.
pub fn launch_request(args: &LaunchArgs) -> Request {
    let id = args.app.strip_suffix(".desktop").unwrap_or(&args.app);
    Request::Act(ActRequest::Launch(AppId(id.to_owned())))
}

/// How a launch ended, as the command line reports it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ending {
    /// The app's process exists.
    Launched(LaunchTiming),
    /// It did not open: the exit code, and one sentence for the person.
    Failed { code: u8, sentence: String },
}

/// What the shell's answer (or the lack of one) means.
pub fn ending(reply: Result<Response, ClientError>) -> Ending {
    let failed = |code, sentence: String| Ending::Failed { code, sentence };
    match reply {
        Ok(Response::Launched(timing)) => Ending::Launched(timing),
        Ok(Response::Failed(failure)) => match failure.kind {
            FailureKind::UnknownApp => failed(2, failure.message),
            _ => failed(1, failure.message),
        },
        Ok(_) => failed(1, "The shell answered something else.".to_owned()),
        Err(ClientError::NotRunning) => failed(1, "The desktop shell is not running.".to_owned()),
        Err(error) => failed(1, format!("The shell could not be reached: {error}.")),
    }
}

/// What is printed for `ending`, with the timing line asked for or not (`print`): the line for stdout, or the sentence for
/// stderr, and the exit code.
pub fn report(ending: &Ending, print: PrintTiming) -> (Output, u8) {
    match (ending, print) {
        (Ending::Launched(timing), PrintTiming::Print) => {
            (Output::Stdout(format!("{}\n", launch_line(timing))), 0)
        }
        (Ending::Launched(_), PrintTiming::Skip) => (Output::Nothing, 0),
        (Ending::Failed { code, sentence }, _) => {
            (Output::Stderr(format!("sill: {sentence}")), *code)
        }
    }
}

/// Where a report's text goes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Output {
    Nothing,
    Stdout(String),
    Stderr(String),
}

/// Run `sill launch`.
pub fn main(args: &LaunchArgs) -> ExitCode {
    let (output, code) = report(&ending(send(&launch_request(args))), args.timing);
    match output {
        Output::Nothing => {}
        Output::Stdout(text) => {
            if let stdout::Emitted::Failed(_) = stdout::print(&text) {
                return ExitCode::FAILURE;
            }
        }
        Output::Stderr(text) => eprintln!("{text}"),
    }
    ExitCode::from(code)
}

#[cfg(test)]
#[path = "launch_tests.rs"]
mod tests;
