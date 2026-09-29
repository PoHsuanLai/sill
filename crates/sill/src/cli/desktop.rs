//! The desktop's command line: `sill dock`, `sill power-menu`, `sill control-center`, `sill
//! screenshot` and `sill widgets`.

use clap::{Subcommand, ValueEnum};
use sill_ipc::act::{ActRequest, DockRequest};
use sill_ipc::request::Request;
use sill_ipc::show::{EditWidgetsRequest, Reveal, ShowRequest};
use sill_launcher::AppId;
use sill_model::screenshots::ShotKind;

/// `sill dock ...`.
#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub enum DockAction {
    /// Keep APP (a desktop id, `org.mozilla.firefox`) in the dock.
    Pin { app: String },
    /// Stop keeping APP; a running app keeps its tile.
    Unpin { app: String },
}

/// `sill power-menu ...`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Subcommand)]
pub enum PowerMenuAction {
    Open,
    Close,
    Toggle,
}

/// `sill control-center ...`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Subcommand)]
pub enum ControlCenterAction {
    Toggle,
}

/// `sill screenshot KIND`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ShotArg {
    /// A rectangle you drag out (COSMIC's picker).
    Area,
    /// One window (COSMIC's picker).
    Window,
    /// The whole screen, at once.
    Screen,
}

/// `sill widgets ...`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Subcommand)]
pub enum WidgetsAction {
    /// Show the desktop widgets when hidden, hide them when shown.
    ToggleDesktop,
    /// Open Edit Widgets, the widget gallery.
    Edit,
    /// Close Edit Widgets.
    EditClose,
}

/// The request `sill dock ACTION` sends.
pub fn dock_request(action: &DockAction) -> Request {
    Request::Act(ActRequest::Dock(match action {
        DockAction::Pin { app } => DockRequest::Pin {
            app: AppId(app.clone()),
        },
        DockAction::Unpin { app } => DockRequest::Unpin {
            app: AppId(app.clone()),
        },
    }))
}

/// The request `sill power-menu ACTION` sends.
pub fn power_menu_request(action: PowerMenuAction) -> Request {
    Request::Show(ShowRequest::PowerMenu(match action {
        PowerMenuAction::Open => Reveal::Open,
        PowerMenuAction::Close => Reveal::Close,
        PowerMenuAction::Toggle => Reveal::Toggle,
    }))
}

/// The request `sill control-center ACTION` sends.
pub fn control_center_request(action: ControlCenterAction) -> Request {
    match action {
        ControlCenterAction::Toggle => Request::Show(ShowRequest::ControlCenter),
    }
}

/// The request `sill screenshot KIND` sends.
pub fn screenshot_request(kind: ShotArg) -> Request {
    Request::Act(ActRequest::Screenshot(match kind {
        ShotArg::Area => ShotKind::Area,
        ShotArg::Window => ShotKind::Window,
        ShotArg::Screen => ShotKind::Screen,
    }))
}

/// The request `sill widgets ACTION` sends.
pub fn widgets_request(action: WidgetsAction) -> Request {
    match action {
        WidgetsAction::ToggleDesktop => Request::Act(ActRequest::ToggleDesktopWidgets),
        WidgetsAction::Edit => Request::Show(ShowRequest::EditWidgets(EditWidgetsRequest::Open)),
        WidgetsAction::EditClose => {
            Request::Show(ShowRequest::EditWidgets(EditWidgetsRequest::Close))
        }
    }
}
