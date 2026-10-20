//! `sill notifications {toggle,open,close,dismiss-all,status}`: the notification center's and the
//! banners' command line, and its pure mapping to a request.

use clap::Subcommand;
use sill_ipc::act::ActRequest;
use sill_ipc::request::Request;
use sill_ipc::show::{Reveal, ShowRequest};
use sill_ipc::status::StatusRequest;
use sill_launcher::actions::ActionId;

use super::verbs::do_request;

/// `sill notifications ...`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Subcommand)]
pub enum NotificationsAction {
    /// Open the notification center when closed, close it when open: a shortcut.
    Toggle,
    /// Open it, or keep it open.
    Open,
    /// Close it if open; nothing otherwise.
    Close,
    /// Dismiss every banner on screen.
    DismissAll,
    /// Who holds `org.freedesktop.Notifications`, and how many banners and kept notifications
    /// there are.
    Status,
}

/// The request `sill notifications ACTION` sends.
pub fn notifications_request(action: NotificationsAction) -> Request {
    match action {
        NotificationsAction::Toggle => do_request(ActionId::NotificationCenter),
        NotificationsAction::Open => center(Reveal::Open),
        NotificationsAction::Close => center(Reveal::Close),
        NotificationsAction::DismissAll => Request::Act(ActRequest::DismissNotifications),
        NotificationsAction::Status => Request::Status(StatusRequest::Notifications),
    }
}

fn center(reveal: Reveal) -> Request {
    Request::Show(ShowRequest::NotificationCenter(reveal))
}
