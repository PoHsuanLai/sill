//! The one pure mapping from a parsed command to the request it sends. Each verb group's file
//! maps its own verbs; this file only picks the group.

use sill_ipc::act::ActRequest;
use sill_ipc::request::Request;
use sill_ipc::show::{Reveal, ShowRequest};
use sill_ipc::status::StatusRequest;
use sill_launcher::actions::ActionId;

use super::Command;
use super::action::do_action_request;
use super::debug::debug_request;
use super::desktop::dock_request;
use super::intents::intents_request;
use super::keycap::keycap_request;
use super::launch::launch_request;
use super::launcher::{clipboard_request, launcher_request};
use super::notifications::notifications_request;
use super::session::lock_request;
use super::verbs::{action_of, do_request, screenshot_action, switcher_action, workspace_action};

/// The request `command` sends; `None` for `run`, `style` and `auth-check`, which are not requests.
pub fn request(command: &Command) -> Option<Request> {
    Some(match command {
        Command::Run(_) | Command::AuthCheck | Command::Style { .. } => return None,
        Command::Ping => Request::Ping,
        Command::Quit => Request::Quit,
        Command::Reload => Request::Reload,
        Command::FocusLatest => Request::Act(ActRequest::FocusLatest),
        Command::NextWindow => do_request(ActionId::NextWindow),
        Command::Verb(verb) => do_request(action_of(*verb)),
        Command::Launch(args) => launch_request(args),
        Command::Do(args) => do_action_request(args),
        Command::Launcher { action } => launcher_request(action),
        Command::Dock { action } => dock_request(action),
        Command::PowerMenu { action } => match action {
            Reveal::Open => do_request(ActionId::PowerMenu),
            Reveal::Toggle | Reveal::Close => Request::Show(ShowRequest::PowerMenu(*action)),
        },
        Command::ForceQuit { action } => match action {
            Reveal::Toggle => do_request(ActionId::ForceQuitPanel),
            Reveal::Open | Reveal::Close => Request::Show(ShowRequest::ForceQuit(*action)),
        },
        Command::ControlCenter { .. } => do_request(ActionId::ControlCenter),
        Command::Screenshot { kind } => do_request(screenshot_action(*kind)),
        Command::Companion { action } => Request::Show(ShowRequest::Companion(*action)),
        Command::Voice { action } => Request::Show(ShowRequest::Voice(*action)),
        Command::Display { action } => Request::Act(ActRequest::Display(*action)),
        Command::Notifications { action } => notifications_request(*action),
        Command::Lock { verb } => lock_request(*verb),
        Command::Switcher { action } => do_request(switcher_action(*action)),
        Command::Workspace { action } => do_request(workspace_action(*action)),
        Command::Polkit { .. } => Request::Status(StatusRequest::Polkit),
        Command::Idle { .. } => Request::Status(StatusRequest::Idle),
        Command::Keycap { action } => keycap_request(*action),
        Command::Intents { action } => intents_request(action),
        Command::Clipboard { action } => clipboard_request(*action),
        Command::Debug { action } => debug_request(action),
    })
}
