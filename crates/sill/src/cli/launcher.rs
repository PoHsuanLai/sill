//! The launcher's command line: `sill launcher {toggle,open,close}` and `sill clipboard
//! {status,clear}`.

use clap::Subcommand;
use sill_ipc::act::ActRequest;
use sill_ipc::request::Request;
use sill_ipc::show::{LauncherRequest, ShowRequest};
use sill_ipc::status::StatusRequest;

/// `sill launcher ...`.
#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub enum LauncherAction {
    Toggle,
    Open {
        /// Text to start with.
        #[arg(long, default_value = "")]
        query: String,
    },
    Close,
}

/// `sill clipboard ...`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Subcommand)]
pub enum ClipboardAction {
    /// Whether the clipboard history is kept, and how many entries it holds (never what).
    Status,
    /// Forget every entry now.
    Clear,
}

/// The request `sill launcher ACTION` sends.
pub fn launcher_request(action: &LauncherAction) -> Request {
    Request::Show(ShowRequest::Launcher(match action {
        LauncherAction::Toggle => LauncherRequest::Toggle,
        LauncherAction::Open { query } => LauncherRequest::Open {
            query: query.clone(),
        },
        LauncherAction::Close => LauncherRequest::Close,
    }))
}

/// The request `sill clipboard ACTION` sends.
pub fn clipboard_request(action: ClipboardAction) -> Request {
    match action {
        ClipboardAction::Status => Request::Status(StatusRequest::Clipboard),
        ClipboardAction::Clear => Request::Act(ActRequest::ClearClipboard),
    }
}
