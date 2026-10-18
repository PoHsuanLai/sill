//! The one pure mapping from a parsed command to the request it sends. Each verb group's file
//! maps its own verbs; this file only picks the group.

use sill_ipc::act::ActRequest;
use sill_ipc::request::Request;
use sill_ipc::show::ShowRequest;

use super::Command;
use super::debug::debug_request;
use super::desktop::{control_center_request, dock_request, widgets_request};
use super::display::display_request;
use super::intents::intents_request;
use super::keycap::keycap_request;
use super::launch::launch_request;
use super::launcher::{clipboard_request, launcher_request};
use super::notifications::notifications_request;
use super::session::{idle_request, lock_request, polkit_request};

/// The request `command` sends; `None` for `run`, `style` and `auth-check`, which are not requests.
pub fn request(command: &Command) -> Option<Request> {
    Some(match command {
        Command::Run(_) | Command::AuthCheck | Command::Style { .. } => return None,
        Command::Ping => Request::Ping,
        Command::Quit => Request::Quit,
        Command::Reload => Request::Reload,
        Command::FocusLatest => Request::Act(ActRequest::FocusLatest),
        Command::Launch(args) => launch_request(args),
        Command::Launcher { action } => launcher_request(action),
        Command::Dock { action } => dock_request(action),
        Command::PowerMenu { action } => Request::Show(ShowRequest::PowerMenu(*action)),
        Command::ForceQuit { action } => Request::Show(ShowRequest::ForceQuit(*action)),
        Command::ControlCenter { action } => control_center_request(*action),
        Command::Screenshot { kind } => Request::Act(ActRequest::Screenshot(*kind)),
        Command::Widgets { action } => widgets_request(*action),
        Command::Companion { action } => Request::Show(ShowRequest::Companion(*action)),
        Command::Voice { action } => Request::Show(ShowRequest::Voice(*action)),
        Command::Display { action } => display_request(action),
        Command::Notifications { action } => notifications_request(*action),
        Command::Lock { verb } => lock_request(*verb),
        Command::Switcher { action } => Request::Act(ActRequest::Switcher(*action)),
        Command::Workspace { action } => Request::Act(ActRequest::Workspace(*action)),
        Command::Polkit { action } => polkit_request(*action),
        Command::Idle { action } => idle_request(*action),
        Command::Keycap { action } => keycap_request(*action),
        Command::Intents { action } => intents_request(action),
        Command::Clipboard { action } => clipboard_request(*action),
        Command::Debug { action } => debug_request(action),
    })
}
