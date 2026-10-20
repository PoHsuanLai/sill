//! The command line's actions: the words that stand for an action of the shell, and the verbs the
//! compositor sends by those words. The compositor's keymap and this command line read one
//! `shell_verbs::ShellVerb`, so a rename is a compile error on both sides; [`action_of`] is where
//! each verb becomes the action it does, and [`do_request`] the request that asks for one.

use std::ffi::OsString;
use std::iter::once;

use clap::Parser;
use shell_verbs::ShellVerb;
use sill_ipc::act::{SwitcherRequest, WorkspaceRequest};
use sill_ipc::request::Request;
use sill_launcher::actions::ActionId;
use sill_model::actions::Ask;
use sill_model::screenshots::ShotKind;

use super::{Cli, Command, request};

/// The request that asks for `action` on the desktop itself.
pub(super) fn do_request(action: ActionId) -> Request {
    Request::Do(Ask::of(action))
}

/// The action `verb` does: one row per verb, no wildcard, so a new verb is a compile error here.
pub fn action_of(verb: ShellVerb) -> ActionId {
    match verb {
        ShellVerb::Lock => ActionId::Lock,
        ShellVerb::NextWindow => ActionId::NextWindow,
        ShellVerb::DockToggleHide => ActionId::ToggleDockHide,
        ShellVerb::LauncherToggle => ActionId::Launcher,
        ShellVerb::PowerMenuOpen => ActionId::PowerMenu,
        ShellVerb::ForceQuitToggle => ActionId::ForceQuitPanel,
        ShellVerb::ScreenshotScreen => screenshot_action(ShotKind::Screen),
        ShellVerb::ScreenshotArea => screenshot_action(ShotKind::Area),
        ShellVerb::ScreenshotWindow => screenshot_action(ShotKind::Window),
        ShellVerb::SwitcherNext => switcher_action(SwitcherRequest::Next),
        ShellVerb::SwitcherPrev => switcher_action(SwitcherRequest::Prev),
        ShellVerb::SwitcherCommit => switcher_action(SwitcherRequest::Commit),
        ShellVerb::SwitcherCancel => switcher_action(SwitcherRequest::Cancel),
        ShellVerb::ControlCenterToggle => ActionId::ControlCenter,
        ShellVerb::NotificationCenterToggle => ActionId::NotificationCenter,
    }
}

/// The action `sill screenshot KIND` is.
pub(super) fn screenshot_action(kind: ShotKind) -> ActionId {
    ActionId::screenshot(kind)
}

/// The action `sill switcher WORD` is.
pub(super) fn switcher_action(word: SwitcherRequest) -> ActionId {
    match word {
        SwitcherRequest::Next => ActionId::SwitcherNext,
        SwitcherRequest::Prev => ActionId::SwitcherPrev,
        SwitcherRequest::Commit => ActionId::SwitcherCommit,
        SwitcherRequest::Cancel => ActionId::SwitcherCancel,
    }
}

/// The action `sill workspace WORD` is.
pub(super) fn workspace_action(word: WorkspaceRequest) -> ActionId {
    match word {
        WorkspaceRequest::Prev => ActionId::PreviousSpace,
        WorkspaceRequest::Next => ActionId::NextSpace,
        WorkspaceRequest::Overview => ActionId::MissionControl,
    }
}

/// The command a line of the verbs the compositor sends spells, read through
/// `ShellVerb::parse`; `None` for any other line (clap's grammar takes those).
pub(super) fn verb_command(args: &[OsString]) -> Option<Command> {
    let words: Vec<&str> = args.iter().map(|arg| arg.to_str()).collect::<Option<_>>()?;
    ShellVerb::parse(&words).map(Command::Verb)
}

/// The request a line of the shell's words sends: a verb of the compositor's, else whatever
/// clap's grammar makes of it. `None` when it is no command that sends a request.
pub fn request_of_words<S: AsRef<str>>(words: &[S]) -> Option<Request> {
    let words: Vec<&str> = words.iter().map(AsRef::as_ref).collect();
    if let Some(verb) = ShellVerb::parse(&words) {
        return Some(do_request(action_of(verb)));
    }
    Cli::try_parse_from(once("sill").chain(words))
        .ok()
        .and_then(|cli| cli.command)
        .and_then(|command| request(&command))
}
