//! The desktop's command line: `sill dock`. The verbs
//! that are a wire enum's words (`sill power-menu`, `sill force-quit`, `sill screenshot`,
//! `sill companion`, `sill voice`) take the enum itself, parsed by `word::word`.

use clap::Subcommand;
use sill_ipc::request::Request;
use sill_launcher::AppId;
use sill_launcher::actions::ActionId;
use sill_model::actions::{Ask, Subject};

use super::verbs::do_request;

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

/// The request `sill dock ACTION` sends.
pub fn dock_request(action: &DockAction) -> Request {
    match action {
        DockAction::Pin { app } => app_request(ActionId::PinApp, app),
        DockAction::Unpin { app } => app_request(ActionId::UnpinApp, app),
        DockAction::ToggleHide => do_request(ActionId::ToggleDockHide),
    }
}

/// The request that does `action` to the app with desktop id `app`.
fn app_request(action: ActionId, app: &str) -> Request {
    Request::Do(Ask::on(action, Subject::App(AppId(app.to_owned()))))
}
