//! The one pure mapping from a parsed command to the request it sends. Each verb group's file
//! maps its own verbs; this file only picks the group.

use sill_ipc::act::ActRequest;
use sill_ipc::request::Request;

use super::Command;
use super::debug::debug_request;
use super::desktop::{
    companion_request, control_center_request, dock_request, power_menu_request,
    screenshot_request, widgets_request,
};
use super::display::display_request;
use super::intents::intents_request;
use super::keycap::keycap_request;
use super::launcher::{clipboard_request, launcher_request};
use super::notifications::notifications_request;
use super::session::{
    idle_request, lock_request, polkit_request, switcher_request, workspace_request,
};

/// The request `command` sends; `None` for `run`, `style` and `auth-check`, which are not requests.
pub fn request(command: &Command) -> Option<Request> {
    Some(match command {
        Command::Run(_) | Command::AuthCheck | Command::Style { .. } | Command::Session { .. } => {
            return None;
        }
        Command::Ping => Request::Ping,
        Command::Quit => Request::Quit,
        Command::Reload => Request::Reload,
        Command::FocusLatest => Request::Act(ActRequest::FocusLatest),
        Command::Launcher { action } => launcher_request(action),
        Command::Dock { action } => dock_request(action),
        Command::PowerMenu { action } => power_menu_request(*action),
        Command::ControlCenter { action } => control_center_request(*action),
        Command::Screenshot { kind } => screenshot_request(*kind),
        Command::Widgets { action } => widgets_request(*action),
        Command::Companion { action } => companion_request(*action),
        Command::Display { action } => display_request(action),
        Command::Notifications { action } => notifications_request(*action),
        Command::Lock { verb } => lock_request(*verb),
        Command::Switcher { action } => switcher_request(*action),
        Command::Workspace { action } => workspace_request(*action),
        Command::Polkit { action } => polkit_request(*action),
        Command::Idle { action } => idle_request(*action),
        Command::Keycap { action } => keycap_request(*action),
        Command::Intents { action } => intents_request(action),
        Command::Clipboard { action } => clipboard_request(*action),
        Command::Debug { action } => debug_request(action),
    })
}
