//! The desktop's command line: `sill dock` and `sill control-center`. The verbs
//! that are a wire enum's words (`sill power-menu`, `sill force-quit`, `sill screenshot`,
//! `sill companion`, `sill voice`) take the enum itself, parsed by `word::word`.

use clap::Subcommand;
use sill_ipc::act::{ActRequest, DockRequest};
use sill_ipc::request::Request;
use sill_ipc::show::ShowRequest;
use sill_launcher::AppId;

/// `sill dock ...`.
#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub enum DockAction {
    /// Keep APP (a desktop id, `org.mozilla.firefox`) in the dock.
    Pin { app: String },
    /// Stop keeping APP; a running app keeps its tile.
    Unpin { app: String },
    /// Hide the dock until the pointer reaches its edge when it is shown, keep it shown when it is
    /// hidden (flips `dock.autohide`).
    ToggleHide,
}

/// `sill control-center ...`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Subcommand)]
pub enum ControlCenterAction {
    Toggle,
}

/// The request `sill dock ACTION` sends.
pub fn dock_request(action: &DockAction) -> Request {
    match action {
        DockAction::Pin { app } => Request::Act(ActRequest::Dock(DockRequest::Pin {
            app: AppId(app.clone()),
        })),
        DockAction::Unpin { app } => Request::Act(ActRequest::Dock(DockRequest::Unpin {
            app: AppId(app.clone()),
        })),
        DockAction::ToggleHide => Request::Act(ActRequest::ToggleDockHide),
    }
}

/// The request `sill control-center ACTION` sends.
pub fn control_center_request(action: ControlCenterAction) -> Request {
    match action {
        ControlCenterAction::Toggle => Request::Show(ShowRequest::ControlCenter),
    }
}
